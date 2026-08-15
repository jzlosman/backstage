use backstage_core::{
    AdapterDescriptor, Capability, CapabilityView, FactProvenance, FactValue, RecognitionLevel,
    RecordLocator, SourceClaim, SourceReference, StructuredBlock, StructuredItem, SubjectId,
    SummaryFact, WorkRecord, WorkRecordRecognition, WorkRecordSource, WorkRecordWarning,
};

#[test]
fn record_locators_produce_stable_project_scoped_subjects() {
    let locator = RecordLocator::new("project_a", "openspec", r"openspec\changes\search");
    let normalized = RecordLocator::new("project_a", "openspec", "openspec/changes/search");
    let other_project = RecordLocator::new("project_b", "openspec", "openspec/changes/search");

    assert_eq!(locator, normalized);
    assert_eq!(locator.subject_id(), normalized.subject_id());
    assert_ne!(locator.subject_id(), other_project.subject_id());
    assert!(locator.subject_id().as_str().starts_with("subject_"));

    let serialized = serde_json::to_string(&locator.subject_id()).expect("serialize subject id");
    assert_eq!(serialized, format!("\"{}\"", locator.subject_id().as_str()));
}

#[test]
fn subject_hashing_frames_locator_fields_instead_of_concatenating_them() {
    let left = RecordLocator::new("project", "format:with", "separator").subject_id();
    let right = RecordLocator::new("project:format", "with", "separator").subject_id();

    assert_ne!(left, right);
}

#[test]
fn adapter_versions_are_provenance_not_subject_identity() {
    let locator = RecordLocator::new("project", "openspec", "openspec/changes/search");
    let v1 = AdapterDescriptor::new("openspec-v1", "openspec", 1, 10);
    let v2 = AdapterDescriptor::new("openspec-v2", "openspec", 2, 10);

    assert_eq!(SubjectId::for_locator(&locator), locator.subject_id());
    assert_ne!(v1, v2);
    assert_eq!(locator.subject_id(), locator.subject_id());
    assert_eq!(v1.format_id(), v2.format_id());
}

#[test]
fn source_claims_and_neutral_records_serialize_deterministically() {
    let descriptor = AdapterDescriptor::new("openspec-v1", "openspec", 1, 10);
    let locator = RecordLocator::new("project", descriptor.format_id(), "openspec/changes/search");
    let recognition = WorkRecordRecognition::new(
        RecognitionLevel::Recognized,
        &descriptor,
        vec!["tasks detected".to_owned(), "proposal detected".to_owned()],
    );
    let tasks_fact = SummaryFact::new(
        "openspec.task.done_count",
        "Done",
        FactValue::Count(2),
        FactProvenance::new("openspec-v1", vec!["tasks.md".to_owned()]),
    );
    let status_fact = SummaryFact::new(
        "openspec.primary_status",
        "Status",
        FactValue::Text("active".to_owned()),
        FactProvenance::new("openspec-v1", vec!["tasks.md".to_owned()]),
    );

    let first = WorkRecord::new(
        locator.clone(),
        "Search",
        recognition.clone(),
        vec![
            WorkRecordSource::new("tasks.md", Some(20)),
            WorkRecordSource::new("proposal.md", Some(10)),
        ],
        vec![tasks_fact.clone(), status_fact.clone()],
        vec![
            WorkRecordWarning::new(
                "tasks_unavailable",
                "Tasks are unavailable",
                Some("tasks.md"),
            ),
            WorkRecordWarning::new(
                "proposal_partial",
                "Proposal is partial",
                Some("proposal.md"),
            ),
        ],
        vec![
            Capability::new("source", "Source"),
            Capability::new("overview", "Overview"),
        ],
    );
    let second = WorkRecord::new(
        locator,
        "Search",
        recognition,
        vec![
            WorkRecordSource::new("proposal.md", Some(10)),
            WorkRecordSource::new("tasks.md", Some(20)),
        ],
        vec![status_fact, tasks_fact],
        vec![
            WorkRecordWarning::new(
                "proposal_partial",
                "Proposal is partial",
                Some("proposal.md"),
            ),
            WorkRecordWarning::new(
                "tasks_unavailable",
                "Tasks are unavailable",
                Some("tasks.md"),
            ),
        ],
        vec![
            Capability::new("overview", "Overview"),
            Capability::new("source", "Source"),
        ],
    );

    assert_eq!(first, second);
    assert_eq!(
        serde_json::to_string(&first).expect("serialize first record"),
        serde_json::to_string(&second).expect("serialize second record")
    );
    assert_eq!(first.subject_id, first.locator.subject_id());
    assert_eq!(first.sources[0].relative_path, "proposal.md");
    assert_eq!(first.facts[0].key, "openspec.primary_status");
    assert_eq!(first.capabilities[0].id, "overview");

    let mut claims = [
        SourceClaim::new("tasks.md"),
        SourceClaim::new("proposal.md"),
    ];
    claims.sort();
    assert_eq!(claims[0].relative_path, "proposal.md");
}

#[test]
fn neutral_structured_blocks_have_stable_tagged_serialization() {
    let source = SourceReference::new("tasks.md", Some(7));
    let fact = SummaryFact::new(
        "openspec.task.open_count",
        "Open",
        FactValue::Count(1),
        FactProvenance::new("openspec-v1", vec!["tasks.md".to_owned()]),
    );
    let warning =
        WorkRecordWarning::new("partial", "Some content is unavailable", Some("tasks.md"));
    let blocks = vec![
        StructuredBlock::markdown_section("destination", "Destination", "Build it", source.clone()),
        StructuredBlock::fact_register("facts", "Facts", vec![fact]),
        StructuredBlock::progress("progress", "Tasks", 2, 3),
        StructuredBlock::item_collection(
            "items",
            "Tasks",
            vec![StructuredItem::new("task-1", "Ship", None, source.clone())],
        ),
        StructuredBlock::relationship_list("relations", "Replaces", vec![]),
        StructuredBlock::empty_state("empty", "Nothing here"),
        StructuredBlock::warning("warning", warning),
        StructuredBlock::source_list("sources", "Source", vec![source]),
    ];
    let view = CapabilityView::new(Capability::new("overview", "Overview"), blocks);

    let payload = serde_json::to_value(view).expect("serialize capability view");
    let kinds = payload["blocks"]
        .as_array()
        .expect("block array")
        .iter()
        .map(|block| block["kind"].as_str().expect("block kind"))
        .collect::<Vec<_>>();

    assert_eq!(
        kinds,
        vec![
            "markdown_section",
            "fact_register",
            "progress",
            "item_collection",
            "relationship_list",
            "empty_state",
            "warning",
            "source_list",
        ]
    );
    assert_eq!(payload["blocks"][0]["source"]["line"], 7);
}
