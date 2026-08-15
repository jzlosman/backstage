use backstage_core::{
    FormatRegistry, MarkdownAdapter, PlanningFormatAdapter, PlanningPattern,
    PlanningPatternAdapter, ProjectSourceInventory, RecognitionLevel, RecordSourceCapture,
    SourceInventoryEntry, SourceObservation, SourceSnapshot, StructuredBlock,
};

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
                        byte_len: 8,
                        modified_unix_nanos: Some(index as u128 + 1),
                    },
                )
            })
            .collect(),
    )
}

fn snapshot(path: &str, text: &str) -> SourceSnapshot {
    let observation = SourceObservation {
        byte_len: text.len() as u64,
        modified_unix_nanos: Some(9),
    };
    SourceSnapshot::from_observations(path, text.as_bytes().to_vec(), observation, observation)
        .expect("stable source snapshot")
}

#[test]
fn markdown_adapter_emits_one_plain_record_for_every_source() {
    let adapter = MarkdownAdapter::new();
    let records = adapter
        .detect(&inventory(&["docs/Guide.md", "README.md"]))
        .expect("detect plain Markdown");

    assert_eq!(adapter.descriptor().adapter_id(), "markdown-v1");
    assert_eq!(adapter.descriptor().format_id(), "markdown");
    assert_eq!(records.len(), 2);
    assert!(
        records
            .iter()
            .all(|record| record.recognition_level == RecognitionLevel::Plain)
    );
    assert_eq!(records[0].adapter_record_key, "README.md");
    assert_eq!(records[1].display_name, "Guide.md");
    assert_eq!(records[1].claims[0].relative_path, "docs/Guide.md");
}

#[test]
fn planning_pattern_adapter_preserves_deterministic_pattern_evidence() {
    let broad = PlanningPattern::custom(r"(?:^|/)PLAN\.md$", 99).expect("broad pattern");
    let exact = PlanningPattern::custom(r"^docs/PLAN\.md$", 1).expect("exact pattern");
    let adapter = PlanningPatternAdapter::new(vec![broad, exact]);

    let records = adapter
        .detect(&inventory(&["docs/PLAN.md", "README.md"]))
        .expect("detect planning patterns");

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].recognition_level, RecognitionLevel::Possible);
    assert_eq!(records[0].claims.len(), 1);
    assert_eq!(records[0].claims[0].relative_path, "docs/PLAN.md");
    assert_eq!(records[0].evidence.len(), 1);
    assert!(records[0].evidence[0].starts_with("Path matches configured planning pattern(s): "));
    let first_evidence = records[0].evidence.clone();

    let reversed =
        PlanningPatternAdapter::new(adapter.patterns().iter().cloned().rev().collect::<Vec<_>>());
    assert_eq!(
        reversed
            .detect(&inventory(&["docs/PLAN.md"]))
            .expect("detect reversed patterns")[0]
            .evidence,
        first_evidence
    );
}

#[test]
fn planning_candidates_and_plain_fallback_compose_without_duplicate_sources() {
    let pattern = PlanningPattern::custom(r"^PLAN\.md$", 0).expect("pattern");
    let registry = FormatRegistry::new(vec![
        Box::new(MarkdownAdapter::new()),
        Box::new(PlanningPatternAdapter::new(vec![pattern])),
    ]);

    let result = registry.detect(&inventory(&["PLAN.md", "README.md"]));

    assert_eq!(result.source_count, 2);
    assert_eq!(result.records.len(), 2);
    assert!(result.records.iter().any(|record| {
        record.recognition_level == RecognitionLevel::Possible
            && record.claims[0].relative_path == "PLAN.md"
    }));
    assert!(result.records.iter().any(|record| {
        record.recognition_level == RecognitionLevel::Plain
            && record.claims[0].relative_path == "README.md"
    }));
    assert_eq!(
        result
            .records
            .iter()
            .map(|record| record.claims.len())
            .sum::<usize>(),
        result.source_count
    );
}

#[test]
fn generic_markdown_detail_keeps_exact_source_readable() {
    let adapter = MarkdownAdapter::new();
    let detected = adapter
        .detect(&inventory(&["notes.md"]))
        .expect("detect notes")
        .remove(0);
    let capture = RecordSourceCapture::complete(vec![snapshot(
        "notes.md",
        "# Notes\n\n<script>untrusted()</script>\n",
    )]);

    let views = adapter
        .build_detail(&detected, &capture)
        .expect("build generic source detail");

    assert_eq!(views.len(), 1);
    assert_eq!(views[0].capability.id, "source");
    match &views[0].blocks[0] {
        StructuredBlock::MarkdownSection {
            markdown, source, ..
        } => {
            assert_eq!(markdown, "# Notes\n\n<script>untrusted()</script>\n");
            assert_eq!(source.relative_path, "notes.md");
        }
        block => panic!("expected exact source block, got {block:?}"),
    }
}

#[test]
fn generic_scan_summary_fingerprints_only_complete_captures() {
    let adapter = MarkdownAdapter::new();
    let detected = adapter
        .detect(&inventory(&["notes.md"]))
        .expect("detect notes")
        .remove(0);

    let complete = adapter
        .summarize(
            &detected,
            &RecordSourceCapture::complete(vec![snapshot("notes.md", "notes")]),
        )
        .expect("summarize complete source");
    let partial = adapter
        .summarize(
            &detected,
            &RecordSourceCapture::partial(
                vec![],
                vec![backstage_core::SourceCaptureFailure::new(
                    "notes.md",
                    "source_unavailable",
                    "source unavailable",
                )],
            ),
        )
        .expect("summarize partial source");

    assert!(complete.fingerprint.is_some());
    assert!(partial.fingerprint.is_none());
    assert_eq!(partial.warnings[0].code, "incomplete_source_snapshot");
}
