use serde::{Deserialize, Serialize};

use crate::{
    AdapterDescriptor, Capability, CapabilityView, SourceClaim, SourceFingerprint,
    SourceObservation, SourceSnapshot, SummaryFact, WorkRecordWarning,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceInventoryEntry {
    pub relative_path: String,
    pub observation: SourceObservation,
}

impl SourceInventoryEntry {
    pub fn new(relative_path: impl Into<String>, observation: SourceObservation) -> Self {
        Self {
            relative_path: normalize_relative(&relative_path.into()),
            observation,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSourceInventory {
    pub project_id: String,
    pub project_name: String,
    pub sources: Vec<SourceInventoryEntry>,
}

impl ProjectSourceInventory {
    pub fn new(
        project_id: impl Into<String>,
        project_name: impl Into<String>,
        mut sources: Vec<SourceInventoryEntry>,
    ) -> Self {
        sources.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        sources.dedup_by(|left, right| left.relative_path == right.relative_path);
        Self {
            project_id: project_id.into(),
            project_name: project_name.into(),
            sources,
        }
    }

    pub fn source(&self, relative_path: &str) -> Option<&SourceInventoryEntry> {
        self.sources
            .binary_search_by(|source| source.relative_path.as_str().cmp(relative_path))
            .ok()
            .map(|index| &self.sources[index])
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedRecord {
    pub adapter_record_key: String,
    pub display_name: String,
    pub recognition_level: crate::RecognitionLevel,
    pub claims: Vec<SourceClaim>,
    pub evidence: Vec<String>,
    pub capabilities: Vec<Capability>,
}

impl DetectedRecord {
    pub fn new(
        adapter_record_key: impl Into<String>,
        display_name: impl Into<String>,
        recognition_level: crate::RecognitionLevel,
        mut claims: Vec<SourceClaim>,
        mut evidence: Vec<String>,
        mut capabilities: Vec<Capability>,
    ) -> Self {
        claims.sort();
        claims.dedup();
        evidence.sort();
        evidence.dedup();
        capabilities.sort();
        capabilities.dedup();
        Self {
            adapter_record_key: normalize_relative(&adapter_record_key.into()),
            display_name: display_name.into(),
            recognition_level,
            claims,
            evidence,
            capabilities,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceCaptureFailure {
    pub relative_path: String,
    pub code: String,
    pub message: String,
}

impl SourceCaptureFailure {
    pub fn new(
        relative_path: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            relative_path: normalize_relative(&relative_path.into()),
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordSourceCapture {
    pub snapshots: Vec<SourceSnapshot>,
    pub failures: Vec<SourceCaptureFailure>,
}

impl RecordSourceCapture {
    pub fn complete(mut snapshots: Vec<SourceSnapshot>) -> Self {
        snapshots.sort_by(|left, right| left.relative_path().cmp(right.relative_path()));
        Self {
            snapshots,
            failures: vec![],
        }
    }

    pub fn partial(
        mut snapshots: Vec<SourceSnapshot>,
        mut failures: Vec<SourceCaptureFailure>,
    ) -> Self {
        snapshots.sort_by(|left, right| left.relative_path().cmp(right.relative_path()));
        failures.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        Self {
            snapshots,
            failures,
        }
    }

    pub fn snapshot(&self, relative_path: &str) -> Option<&SourceSnapshot> {
        self.snapshots
            .binary_search_by(|snapshot| snapshot.relative_path().cmp(relative_path))
            .ok()
            .map(|index| &self.snapshots[index])
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterSummary {
    pub facts: Vec<SummaryFact>,
    pub warnings: Vec<WorkRecordWarning>,
    pub capabilities: Vec<Capability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<SourceFingerprint>,
}

impl AdapterSummary {
    pub fn new(
        mut facts: Vec<SummaryFact>,
        mut warnings: Vec<WorkRecordWarning>,
        mut capabilities: Vec<Capability>,
        fingerprint: Option<SourceFingerprint>,
    ) -> Self {
        facts.sort_by(|left, right| left.key.cmp(&right.key));
        warnings.sort();
        warnings.dedup();
        capabilities.sort();
        capabilities.dedup();
        Self {
            facts,
            warnings,
            capabilities,
            fingerprint,
        }
    }

    pub fn empty() -> Self {
        Self::new(vec![], vec![], vec![], None)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterHandoff {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_source_path: Option<String>,
    pub continuation_prompt: String,
}

impl AdapterHandoff {
    pub fn new(
        primary_source_path: Option<impl Into<String>>,
        continuation_prompt: impl Into<String>,
    ) -> Self {
        Self {
            primary_source_path: primary_source_path.map(Into::into),
            continuation_prompt: continuation_prompt.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{code}: {message}")]
pub struct AdapterFailure {
    pub code: String,
    pub message: String,
}

impl AdapterFailure {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

pub trait PlanningFormatAdapter: Send + Sync {
    fn descriptor(&self) -> &AdapterDescriptor;

    fn detect(
        &self,
        inventory: &ProjectSourceInventory,
    ) -> Result<Vec<DetectedRecord>, AdapterFailure>;

    fn summarize(
        &self,
        record: &DetectedRecord,
        capture: &RecordSourceCapture,
    ) -> Result<AdapterSummary, AdapterFailure>;

    fn build_detail(
        &self,
        _record: &DetectedRecord,
        _capture: &RecordSourceCapture,
    ) -> Result<Vec<CapabilityView>, AdapterFailure> {
        Ok(vec![])
    }

    fn build_handoff(
        &self,
        record: &DetectedRecord,
        _capture: &RecordSourceCapture,
    ) -> Result<AdapterHandoff, AdapterFailure> {
        Ok(AdapterHandoff::new(
            record
                .claims
                .first()
                .map(|claim| claim.relative_path.clone()),
            "Inspect the exact source before continuing.",
        ))
    }
}

fn normalize_relative(path: &str) -> String {
    let mut normalized = path.replace('\\', "/");
    while let Some(stripped) = normalized.strip_prefix("./") {
        normalized = stripped.to_owned();
    }
    normalized
}
