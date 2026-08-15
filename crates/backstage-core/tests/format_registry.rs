use backstage_core::{
    AdapterDescriptor, AdapterFailure, AdapterSummary, Capability, DetectedRecord, FactProvenance,
    FactValue, FormatRegistry, PlanningFormatAdapter, ProjectSourceInventory, RecognitionLevel,
    RecordSourceCapture, SourceClaim, SourceInventoryEntry, SourceObservation, SourceSnapshot,
    SummaryFact,
};

#[derive(Clone)]
struct FakeAdapter {
    descriptor: AdapterDescriptor,
    detection: Result<Vec<DetectedRecord>, AdapterFailure>,
    summary: Result<AdapterSummary, AdapterFailure>,
}

impl FakeAdapter {
    fn successful(descriptor: AdapterDescriptor, records: Vec<DetectedRecord>) -> Self {
        Self {
            descriptor,
            detection: Ok(records),
            summary: Ok(AdapterSummary::empty()),
        }
    }

    fn failing(descriptor: AdapterDescriptor, code: &str) -> Self {
        Self {
            descriptor,
            detection: Err(AdapterFailure::new(code, "adapter failed")),
            summary: Ok(AdapterSummary::empty()),
        }
    }

    fn with_summary(mut self, summary: AdapterSummary) -> Self {
        self.summary = Ok(summary);
        self
    }
}

impl PlanningFormatAdapter for FakeAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    fn detect(
        &self,
        _inventory: &ProjectSourceInventory,
    ) -> Result<Vec<DetectedRecord>, AdapterFailure> {
        self.detection.clone()
    }

    fn summarize(
        &self,
        _record: &DetectedRecord,
        _capture: &RecordSourceCapture,
    ) -> Result<AdapterSummary, AdapterFailure> {
        self.summary.clone()
    }
}

fn descriptor(id: &str, precedence: u16) -> AdapterDescriptor {
    AdapterDescriptor::new(id, id.trim_end_matches("-v1"), 1, precedence)
}

fn record(key: &str, name: &str, level: RecognitionLevel, paths: &[&str]) -> DetectedRecord {
    DetectedRecord::new(
        key,
        name,
        level,
        paths.iter().copied().map(SourceClaim::new).collect(),
        vec![format!("{name} detected")],
        vec![Capability::new("source", "Source")],
    )
}

fn inventory(paths: &[&str]) -> ProjectSourceInventory {
    ProjectSourceInventory::new(
        "project_1",
        "Workbench",
        paths
            .iter()
            .enumerate()
            .map(|(index, path)| {
                SourceInventoryEntry::new(
                    *path,
                    SourceObservation {
                        byte_len: 10,
                        modified_unix_nanos: Some(index as u128 + 1),
                    },
                )
            })
            .collect(),
    )
}

#[test]
fn explicit_precedence_resolves_specialized_overlap_and_names_competitors() {
    let slower = FakeAdapter::successful(
        descriptor("slower-v1", 20),
        vec![record(
            "slower",
            "Slower",
            RecognitionLevel::Recognized,
            &["PLAN.md"],
        )],
    );
    let winner = FakeAdapter::successful(
        descriptor("winner-v1", 10),
        vec![record(
            "winner",
            "Winner",
            RecognitionLevel::Recognized,
            &["PLAN.md"],
        )],
    );

    let result = FormatRegistry::new(vec![Box::new(slower), Box::new(winner)])
        .detect(&inventory(&["PLAN.md"]));

    assert_eq!(result.records.len(), 1);
    assert_eq!(result.records[0].descriptor.adapter_id(), "winner-v1");
    assert_eq!(result.records[0].claims[0].relative_path, "PLAN.md");
    assert_eq!(result.records[0].warnings.len(), 1);
    assert_eq!(result.records[0].warnings[0].code, "adapter_claim_overlap");
    assert!(result.records[0].warnings[0].message.contains("slower-v1"));
    assert!(result.records[0].warnings[0].message.contains("winner-v1"));
}

#[test]
fn recognized_claims_beat_possible_and_plain_claims_regardless_of_precedence() {
    let possible = FakeAdapter::successful(
        descriptor("possible-v1", 1),
        vec![record(
            "possible",
            "Possible",
            RecognitionLevel::Possible,
            &["PLAN.md"],
        )],
    );
    let plain = FakeAdapter::successful(
        descriptor("markdown-v1", 2),
        vec![record(
            "PLAN.md",
            "PLAN.md",
            RecognitionLevel::Plain,
            &["PLAN.md"],
        )],
    );
    let recognized = FakeAdapter::successful(
        descriptor("recognized-v1", 99),
        vec![record(
            "recognized",
            "Recognized",
            RecognitionLevel::Recognized,
            &["PLAN.md"],
        )],
    );

    let result = FormatRegistry::new(vec![
        Box::new(possible),
        Box::new(plain),
        Box::new(recognized),
    ])
    .detect(&inventory(&["PLAN.md"]));

    assert_eq!(result.records.len(), 1);
    assert_eq!(result.records[0].descriptor.adapter_id(), "recognized-v1");
}

#[test]
fn registry_membership_and_order_are_deterministic_and_unique() {
    let specialized = FakeAdapter::successful(
        descriptor("specialized-v1", 10),
        vec![record(
            "group",
            "Grouped",
            RecognitionLevel::Recognized,
            &["b.md", "a.md"],
        )],
    );
    let possible = FakeAdapter::successful(
        descriptor("planning-pattern-v1", 20),
        vec![record(
            "c.md",
            "Candidate",
            RecognitionLevel::Possible,
            &["c.md"],
        )],
    );
    let plain_records = ["d.md", "c.md", "b.md", "a.md"]
        .into_iter()
        .map(|path| record(path, path, RecognitionLevel::Plain, &[path]))
        .collect();
    let plain = FakeAdapter::successful(descriptor("markdown-v1", 30), plain_records);
    let source_inventory = inventory(&["d.md", "b.md", "a.md", "c.md"]);

    let first = FormatRegistry::new(vec![
        Box::new(plain.clone()),
        Box::new(possible.clone()),
        Box::new(specialized.clone()),
    ])
    .detect(&source_inventory);
    let second = FormatRegistry::new(vec![
        Box::new(specialized),
        Box::new(possible),
        Box::new(plain),
    ])
    .detect(&source_inventory);

    assert_eq!(first, second);
    assert_eq!(first.source_count, 4);
    assert_eq!(first.records.len(), 3);
    assert_eq!(first.records[0].display_name, "Candidate");
    assert_eq!(first.records[1].display_name, "d.md");
    assert_eq!(first.records[2].display_name, "Grouped");
    let represented = first
        .records
        .iter()
        .flat_map(|record| {
            record
                .claims
                .iter()
                .map(|claim| claim.relative_path.as_str())
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        represented,
        std::collections::BTreeSet::from(["a.md", "b.md", "c.md", "d.md"])
    );
    assert_eq!(
        first
            .records
            .iter()
            .map(|record| record.claims.len())
            .sum::<usize>(),
        first.source_count
    );
}

#[test]
fn a_failed_specialized_adapter_does_not_hide_plain_markdown() {
    let broken = FakeAdapter::failing(descriptor("broken-v1", 10), "parse_failed");
    let plain = FakeAdapter::successful(
        descriptor("markdown-v1", 30),
        vec![
            record("a.md", "a.md", RecognitionLevel::Plain, &["a.md"]),
            record("b.md", "b.md", RecognitionLevel::Plain, &["b.md"]),
        ],
    );

    let result = FormatRegistry::new(vec![Box::new(broken), Box::new(plain)])
        .detect(&inventory(&["a.md", "b.md"]));

    assert_eq!(result.records.len(), 2);
    assert!(
        result
            .records
            .iter()
            .all(|record| record.descriptor.adapter_id() == "markdown-v1")
    );
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(result.warnings[0].code, "adapter_detection_failed");
    assert!(result.warnings[0].message.contains("broken-v1"));
}

#[test]
fn scan_summarization_is_pure_and_builds_a_neutral_record() {
    let summary = AdapterSummary::new(
        vec![SummaryFact::new(
            "fake.count",
            "Count",
            FactValue::Count(1),
            FactProvenance::new("fake-v1", vec!["PLAN.md".to_owned()]),
        )],
        vec![],
        vec![Capability::new("overview", "Overview")],
        None,
    );
    let fake = FakeAdapter::successful(
        descriptor("fake-v1", 10),
        vec![record(
            "PLAN.md",
            "Plan",
            RecognitionLevel::Recognized,
            &["PLAN.md"],
        )],
    )
    .with_summary(summary);
    let registry = FormatRegistry::new(vec![Box::new(fake)]);
    let source_inventory = inventory(&["PLAN.md"]);
    let detected = registry.detect(&source_inventory).records.remove(0);
    let snapshot = SourceSnapshot::from_observations(
        "PLAN.md",
        b"# Plan\n".to_vec(),
        SourceObservation {
            byte_len: 7,
            modified_unix_nanos: Some(9),
        },
        SourceObservation {
            byte_len: 7,
            modified_unix_nanos: Some(9),
        },
    )
    .expect("stable snapshot");

    let work_record = registry
        .summarize(
            &source_inventory,
            &detected,
            &RecordSourceCapture::complete(vec![snapshot]),
        )
        .expect("summarize fake record");

    assert_eq!(work_record.locator.project_id, "project_1");
    assert_eq!(work_record.locator.format_id, "fake");
    assert_eq!(work_record.locator.adapter_record_key, "PLAN.md");
    assert_eq!(work_record.recognition.adapter_id, "fake-v1");
    assert_eq!(work_record.facts[0].key, "fake.count");
    assert_eq!(
        work_record
            .capabilities
            .iter()
            .map(|capability| capability.id.as_str())
            .collect::<Vec<_>>(),
        vec!["overview", "source"]
    );
}
