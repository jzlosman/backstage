use crate::{
    AdapterDescriptor, AdapterFailure, AdapterSummary, Capability, CapabilityView, DetectedRecord,
    PlanningFormatAdapter, PlanningPattern, ProjectSourceInventory, RecognitionLevel,
    RecordSourceCapture, SourceClaim, SourceReference, StructuredBlock, WorkRecordWarning,
    fingerprint_complete_snapshots, matching_planning_patterns,
};

const MARKDOWN_PRECEDENCE: u16 = 40;
const PLANNING_PATTERN_PRECEDENCE: u16 = 30;

pub struct MarkdownAdapter {
    descriptor: AdapterDescriptor,
}

impl MarkdownAdapter {
    pub fn new() -> Self {
        Self {
            descriptor: AdapterDescriptor::new("markdown-v1", "markdown", 1, MARKDOWN_PRECEDENCE),
        }
    }
}

impl Default for MarkdownAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl PlanningFormatAdapter for MarkdownAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    fn detect(
        &self,
        inventory: &ProjectSourceInventory,
    ) -> Result<Vec<DetectedRecord>, AdapterFailure> {
        Ok(inventory
            .sources
            .iter()
            .map(|source| {
                DetectedRecord::new(
                    source.relative_path.clone(),
                    display_name(&source.relative_path),
                    RecognitionLevel::Plain,
                    vec![SourceClaim::new(source.relative_path.clone())],
                    vec!["Plain Markdown fallback".to_owned()],
                    vec![Capability::new("source", "Source")],
                )
            })
            .collect())
    }

    fn summarize(
        &self,
        record: &DetectedRecord,
        capture: &RecordSourceCapture,
    ) -> Result<AdapterSummary, AdapterFailure> {
        Ok(generic_summary(record, capture))
    }

    fn build_detail(
        &self,
        record: &DetectedRecord,
        capture: &RecordSourceCapture,
    ) -> Result<Vec<CapabilityView>, AdapterFailure> {
        Ok(generic_source_detail(record, capture))
    }
}

pub struct PlanningPatternAdapter {
    descriptor: AdapterDescriptor,
    patterns: Vec<PlanningPattern>,
}

impl PlanningPatternAdapter {
    pub fn new(mut patterns: Vec<PlanningPattern>) -> Self {
        patterns.sort_by(|left, right| {
            left.id()
                .cmp(right.id())
                .then_with(|| left.expression().cmp(right.expression()))
        });
        Self {
            descriptor: AdapterDescriptor::new(
                "planning-pattern-v1",
                "planning-pattern",
                1,
                PLANNING_PATTERN_PRECEDENCE,
            ),
            patterns,
        }
    }

    pub fn patterns(&self) -> &[PlanningPattern] {
        &self.patterns
    }
}

impl PlanningFormatAdapter for PlanningPatternAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    fn detect(
        &self,
        inventory: &ProjectSourceInventory,
    ) -> Result<Vec<DetectedRecord>, AdapterFailure> {
        let mut records = Vec::new();
        for source in &inventory.sources {
            let matches = matching_planning_patterns(&source.relative_path, &self.patterns);
            if matches.is_empty() {
                continue;
            }
            let accepted = matches
                .into_iter()
                .map(|pattern| format!("{} ({})", pattern.id(), pattern.expression()))
                .collect::<Vec<_>>()
                .join(", ");
            records.push(DetectedRecord::new(
                source.relative_path.clone(),
                display_name(&source.relative_path),
                RecognitionLevel::Possible,
                vec![SourceClaim::new(source.relative_path.clone())],
                vec![format!(
                    "Path matches configured planning pattern(s): {accepted}"
                )],
                vec![Capability::new("source", "Source")],
            ));
        }
        Ok(records)
    }

    fn summarize(
        &self,
        record: &DetectedRecord,
        capture: &RecordSourceCapture,
    ) -> Result<AdapterSummary, AdapterFailure> {
        Ok(generic_summary(record, capture))
    }

    fn build_detail(
        &self,
        record: &DetectedRecord,
        capture: &RecordSourceCapture,
    ) -> Result<Vec<CapabilityView>, AdapterFailure> {
        Ok(generic_source_detail(record, capture))
    }
}

fn generic_summary(record: &DetectedRecord, capture: &RecordSourceCapture) -> AdapterSummary {
    let captured_paths = capture
        .snapshots
        .iter()
        .map(|snapshot| snapshot.relative_path())
        .collect::<std::collections::BTreeSet<_>>();
    let complete = capture.failures.is_empty()
        && record.claims.len() == capture.snapshots.len()
        && record
            .claims
            .iter()
            .all(|claim| captured_paths.contains(claim.relative_path.as_str()));
    let (fingerprint, warnings) = if complete {
        (
            fingerprint_complete_snapshots(record.claims.len(), &capture.snapshots).ok(),
            vec![],
        )
    } else {
        (
            None,
            vec![WorkRecordWarning::without_source(
                "incomplete_source_snapshot",
                "Source fingerprint is unavailable because the captured record is incomplete",
            )],
        )
    };
    AdapterSummary::new(
        vec![],
        warnings,
        vec![Capability::new("source", "Source")],
        fingerprint,
    )
}

pub(crate) fn generic_source_detail(
    record: &DetectedRecord,
    capture: &RecordSourceCapture,
) -> Vec<CapabilityView> {
    let mut blocks = Vec::new();
    for claim in &record.claims {
        let Some(snapshot) = capture.snapshot(&claim.relative_path) else {
            continue;
        };
        match snapshot.text() {
            Some(markdown) => blocks.push(StructuredBlock::markdown_section(
                format!("source:{}", claim.relative_path),
                claim.relative_path.clone(),
                markdown,
                SourceReference::new(claim.relative_path.clone(), None),
            )),
            None => blocks.push(StructuredBlock::warning(
                format!("source-warning:{}", claim.relative_path),
                WorkRecordWarning::new(
                    "source_not_utf8",
                    "Source is not valid UTF-8",
                    Some(claim.relative_path.clone()),
                ),
            )),
        }
    }
    blocks.extend(capture.failures.iter().map(|failure| {
        StructuredBlock::warning(
            format!("source-warning:{}", failure.relative_path),
            WorkRecordWarning::new(
                failure.code.clone(),
                failure.message.clone(),
                Some(failure.relative_path.clone()),
            ),
        )
    }));
    if blocks.is_empty() {
        blocks.push(StructuredBlock::empty_state(
            "source-unavailable",
            "No safely captured source is available",
        ));
    }
    vec![CapabilityView::new(
        Capability::new("source", "Source"),
        blocks,
    )]
}

fn display_name(relative_path: &str) -> String {
    relative_path
        .rsplit('/')
        .next()
        .unwrap_or(relative_path)
        .to_owned()
}
