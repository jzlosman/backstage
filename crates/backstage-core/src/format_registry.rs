use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    AdapterDescriptor, AdapterFailure, AdapterSummary, Capability, DetectedRecord,
    PlanningFormatAdapter, ProjectSourceInventory, RecognitionLevel, RecordLocator,
    RecordSourceCapture, SourceClaim, WorkRecord, WorkRecordRecognition, WorkRecordSource,
    WorkRecordWarning,
};

pub struct FormatRegistry {
    adapters: Vec<Box<dyn PlanningFormatAdapter>>,
}

impl FormatRegistry {
    pub fn new(mut adapters: Vec<Box<dyn PlanningFormatAdapter>>) -> Self {
        adapters.sort_by(|left, right| {
            left.descriptor()
                .precedence()
                .cmp(&right.descriptor().precedence())
                .then_with(|| {
                    left.descriptor()
                        .adapter_id()
                        .cmp(right.descriptor().adapter_id())
                })
        });
        Self { adapters }
    }

    pub fn detect(&self, inventory: &ProjectSourceInventory) -> RegistryDetection {
        let inventory_paths = inventory
            .sources
            .iter()
            .map(|source| source.relative_path.as_str())
            .collect::<BTreeSet<_>>();
        let mut candidates = Vec::new();
        let mut registry_warnings = Vec::new();

        for adapter in &self.adapters {
            match adapter.detect(inventory) {
                Ok(records) => {
                    for mut record in records {
                        record
                            .claims
                            .retain(|claim| inventory_paths.contains(claim.relative_path.as_str()));
                        if record.claims.is_empty() {
                            continue;
                        }
                        candidates.push(Candidate {
                            descriptor: adapter.descriptor().clone(),
                            record,
                            warnings: vec![],
                        });
                    }
                }
                Err(failure) => registry_warnings.push(WorkRecordWarning::without_source(
                    "adapter_detection_failed",
                    format!(
                        "Adapter {} failed during detection ({}): {}",
                        adapter.descriptor().adapter_id(),
                        failure.code,
                        failure.message
                    ),
                )),
            }
        }

        candidates.sort_by(candidate_order);
        let mut claimants: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (candidate_index, candidate) in candidates.iter().enumerate() {
            for claim in &candidate.record.claims {
                claimants
                    .entry(claim.relative_path.clone())
                    .or_default()
                    .push(candidate_index);
            }
        }

        let mut winning_claims = vec![Vec::new(); candidates.len()];
        for (relative_path, candidate_indices) in claimants {
            let winner = candidate_indices
                .iter()
                .copied()
                .min_by(|left, right| candidate_order(&candidates[*left], &candidates[*right]))
                .expect("claim groups are non-empty");
            winning_claims[winner].push(SourceClaim::new(relative_path.clone()));

            let specialized = candidate_indices
                .iter()
                .copied()
                .filter(|index| {
                    candidates[*index].record.recognition_level == RecognitionLevel::Recognized
                })
                .collect::<Vec<_>>();
            let competing_adapters = specialized
                .iter()
                .map(|index| candidates[*index].descriptor.adapter_id().to_owned())
                .collect::<BTreeSet<_>>();
            if competing_adapters.len() > 1 {
                let winner_adapter = candidates[winner].descriptor.adapter_id().to_owned();
                candidates[winner].warnings.push(WorkRecordWarning::new(
                    "adapter_claim_overlap",
                    format!(
                        "Source {relative_path} was claimed by {}; {winner_adapter} won by explicit precedence",
                        competing_adapters.into_iter().collect::<Vec<_>>().join(", ")
                    ),
                    Some(relative_path),
                ));
            }
        }

        let mut records = candidates
            .into_iter()
            .zip(winning_claims)
            .filter_map(|(mut candidate, mut claims)| {
                if claims.is_empty() {
                    return None;
                }
                claims.sort();
                candidate.warnings.sort();
                candidate.warnings.dedup();
                Some(DetectedWorkRecord {
                    descriptor: candidate.descriptor,
                    adapter_record_key: candidate.record.adapter_record_key,
                    display_name: candidate.record.display_name,
                    recognition_level: candidate.record.recognition_level,
                    claims,
                    evidence: candidate.record.evidence,
                    capabilities: candidate.record.capabilities,
                    warnings: candidate.warnings,
                })
            })
            .collect::<Vec<_>>();
        records.sort_by(|left, right| {
            left.display_name
                .to_lowercase()
                .cmp(&right.display_name.to_lowercase())
                .then_with(|| {
                    left.descriptor
                        .format_id()
                        .cmp(right.descriptor.format_id())
                })
                .then_with(|| left.adapter_record_key.cmp(&right.adapter_record_key))
        });
        registry_warnings.sort();
        registry_warnings.dedup();

        RegistryDetection {
            records,
            source_count: inventory.sources.len(),
            warnings: registry_warnings,
        }
    }

    pub fn summarize(
        &self,
        inventory: &ProjectSourceInventory,
        record: &DetectedWorkRecord,
        capture: &RecordSourceCapture,
    ) -> Result<WorkRecord, AdapterFailure> {
        let adapter = self
            .adapters
            .iter()
            .find(|adapter| {
                adapter.descriptor().adapter_id() == record.descriptor.adapter_id()
                    && adapter.descriptor().version() == record.descriptor.version()
            })
            .ok_or_else(|| {
                AdapterFailure::new(
                    "adapter_unavailable",
                    format!(
                        "adapter {} version {} is not registered",
                        record.descriptor.adapter_id(),
                        record.descriptor.version()
                    ),
                )
            })?;
        let detected = record.as_detected_record();
        let mut warnings = record.warnings.clone();
        warnings.extend(capture.failures.iter().map(|failure| {
            WorkRecordWarning::new(
                failure.code.clone(),
                failure.message.clone(),
                Some(failure.relative_path.clone()),
            )
        }));
        let AdapterSummary {
            facts,
            warnings: summary_warnings,
            capabilities: summary_capabilities,
            fingerprint,
        } = match adapter.summarize(&detected, capture) {
            Ok(summary) => summary,
            Err(failure) => {
                warnings.push(WorkRecordWarning::without_source(
                    "adapter_summary_failed",
                    format!(
                        "Adapter {} could not summarize this record ({}): {}",
                        record.descriptor.adapter_id(),
                        failure.code,
                        failure.message
                    ),
                ));
                AdapterSummary::empty()
            }
        };
        warnings.extend(summary_warnings);

        let sources = record
            .claims
            .iter()
            .filter_map(|claim| {
                let modified = capture
                    .snapshot(&claim.relative_path)
                    .map(|snapshot| snapshot.observation().modified_unix_nanos)
                    .or_else(|| {
                        inventory
                            .source(&claim.relative_path)
                            .map(|source| source.observation.modified_unix_nanos)
                    })?;
                Some(WorkRecordSource::new(claim.relative_path.clone(), modified))
            })
            .collect();
        let mut capabilities = record.capabilities.clone();
        capabilities.extend(summary_capabilities);
        let locator = RecordLocator::new(
            inventory.project_id.clone(),
            record.descriptor.format_id(),
            record.adapter_record_key.clone(),
        );
        let recognition = WorkRecordRecognition::new(
            record.recognition_level,
            &record.descriptor,
            record.evidence.clone(),
        );
        let work_record = WorkRecord::new(
            locator,
            record.display_name.clone(),
            recognition,
            sources,
            facts,
            warnings,
            capabilities,
        );
        Ok(match fingerprint {
            Some(fingerprint) => work_record.with_fingerprint(fingerprint),
            None => work_record,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedWorkRecord {
    pub descriptor: AdapterDescriptor,
    pub adapter_record_key: String,
    pub display_name: String,
    pub recognition_level: RecognitionLevel,
    pub claims: Vec<SourceClaim>,
    pub evidence: Vec<String>,
    pub capabilities: Vec<Capability>,
    pub warnings: Vec<WorkRecordWarning>,
}

impl DetectedWorkRecord {
    pub fn as_detected_record(&self) -> DetectedRecord {
        DetectedRecord::new(
            self.adapter_record_key.clone(),
            self.display_name.clone(),
            self.recognition_level,
            self.claims.clone(),
            self.evidence.clone(),
            self.capabilities.clone(),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryDetection {
    pub records: Vec<DetectedWorkRecord>,
    pub source_count: usize,
    pub warnings: Vec<WorkRecordWarning>,
}

struct Candidate {
    descriptor: AdapterDescriptor,
    record: DetectedRecord,
    warnings: Vec<WorkRecordWarning>,
}

fn candidate_order(left: &Candidate, right: &Candidate) -> std::cmp::Ordering {
    left.record
        .recognition_level
        .priority()
        .cmp(&right.record.recognition_level.priority())
        .then_with(|| {
            left.descriptor
                .precedence()
                .cmp(&right.descriptor.precedence())
        })
        .then_with(|| {
            left.descriptor
                .adapter_id()
                .cmp(right.descriptor.adapter_id())
        })
        .then_with(|| {
            left.record
                .adapter_record_key
                .cmp(&right.record.adapter_record_key)
        })
}
