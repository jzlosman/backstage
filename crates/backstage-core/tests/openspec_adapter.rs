use backstage_core::{
    AdapterDescriptor, FactValue, OpenSpecAdapter, PlanningFormatAdapter, ProjectSourceInventory,
    RecognitionLevel, RecordLocator, RecordSourceCapture, SourceCaptureFailure,
    SourceInventoryEntry, SourceObservation, SourceSnapshot, StructuredBlock,
};

const CURRENT: &str = "openspec/changes/search";
const ARCHIVED: &str = "openspec/changes/archive/2026-08-14-search";

fn observation(text: &str, modified: u128) -> SourceObservation {
    SourceObservation {
        byte_len: text.len() as u64,
        modified_unix_nanos: Some(modified),
    }
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
                        byte_len: 100,
                        modified_unix_nanos: Some(index as u128 + 1),
                    },
                )
            })
            .collect(),
    )
}

fn snapshot(path: &str, text: &str, modified: u128) -> SourceSnapshot {
    let observed = observation(text, modified);
    SourceSnapshot::from_observations(path, text.as_bytes().to_vec(), observed, observed)
        .expect("stable OpenSpec snapshot")
}

fn current_inventory() -> ProjectSourceInventory {
    inventory(&[
        "README.md",
        "openspec/changes/search/design.md",
        "openspec/changes/search/proposal.md",
        "openspec/changes/search/specs/catalog/spec.md",
        "openspec/changes/search/tasks.md",
    ])
}

fn current_capture(tasks: &str) -> RecordSourceCapture {
    RecordSourceCapture::complete(vec![
        snapshot(
            "openspec/changes/search/design.md",
            "## Goals / Non-Goals\n\nKeep facts local.\n\n## Decisions\n\nUse adapters.\n",
            1,
        ),
        snapshot(
            "openspec/changes/search/proposal.md",
            "## Why\n\nSearch needs context.\n\n## What Changes\n\n- Add records\n",
            2,
        ),
        snapshot(
            "openspec/changes/search/specs/catalog/spec.md",
            "## ADDED Requirements\n\nCatalog records.\n",
            3,
        ),
        snapshot("openspec/changes/search/tasks.md", tasks, 4),
    ])
}

fn fact_text<'a>(summary: &'a backstage_core::AdapterSummary, key: &str) -> Option<&'a str> {
    summary
        .facts
        .iter()
        .find(|fact| fact.key == key)
        .and_then(|fact| match &fact.value {
            FactValue::Text(value) => Some(value.as_str()),
            _ => None,
        })
}

fn fact_count(summary: &backstage_core::AdapterSummary, key: &str) -> Option<u64> {
    summary
        .facts
        .iter()
        .find(|fact| fact.key == key)
        .and_then(|fact| match fact.value {
            FactValue::Count(value) => Some(value),
            _ => None,
        })
}

#[test]
fn current_mixed_tasks_keep_recognition_progress_views_source_order_and_handoff() {
    let adapter = OpenSpecAdapter::new();
    let records = adapter
        .detect(&current_inventory())
        .expect("detect current OpenSpec change");

    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(adapter.descriptor().adapter_id(), "openspec-v1");
    assert_eq!(adapter.descriptor().format_id(), "openspec");
    assert_eq!(adapter.descriptor().version(), 1);
    assert_eq!(record.adapter_record_key, CURRENT);
    assert_eq!(record.display_name, "search");
    assert_eq!(record.recognition_level, RecognitionLevel::Recognized);
    assert_eq!(
        record
            .claims
            .iter()
            .map(|claim| claim.relative_path.as_str())
            .collect::<Vec<_>>(),
        vec![
            "openspec/changes/search/design.md",
            "openspec/changes/search/proposal.md",
            "openspec/changes/search/specs/catalog/spec.md",
            "openspec/changes/search/tasks.md",
        ]
    );

    let capture = current_capture(
        "# Tasks\n\n- [x] Existing behavior\n- [ ] Adapter parity\n- [?] malformed marker\n",
    );
    let summary = adapter
        .summarize(record, &capture)
        .expect("summarize current OpenSpec change");

    assert_eq!(fact_text(&summary, "openspec.custody"), Some("current"));
    assert_eq!(
        fact_text(&summary, "openspec.primary_status"),
        Some("active")
    );
    assert_eq!(fact_count(&summary, "openspec.task.total_count"), Some(2));
    assert_eq!(fact_count(&summary, "openspec.task.done_count"), Some(1));
    assert_eq!(fact_count(&summary, "openspec.task.open_count"), Some(1));
    assert_eq!(fact_count(&summary, "work_record.source_count"), Some(4));
    assert!(summary.fingerprint.is_some());
    assert!(
        summary
            .warnings
            .iter()
            .any(|warning| warning.code == "openspec_task_parse_warning" && warning.line == Some(5))
    );

    let views = adapter
        .build_detail(record, &capture)
        .expect("build neutral OpenSpec detail");
    assert_eq!(
        views
            .iter()
            .map(|view| view.capability.id.as_str())
            .collect::<Vec<_>>(),
        vec!["overview", "tasks", "source"]
    );
    assert!(views[0].blocks.iter().any(|block| matches!(
        block,
        StructuredBlock::MarkdownSection { title, markdown, .. }
            if title == "Why" && markdown == "Search needs context."
    )));
    assert!(views[1].blocks.iter().any(|block| matches!(
        block,
        StructuredBlock::Progress {
            completed: 1,
            total: 2,
            ..
        }
    )));
    assert!(views[1].blocks.iter().any(|block| matches!(
        block,
        StructuredBlock::ItemCollection { items, .. }
            if items.iter().any(|item| item.title == "Adapter parity")
    )));
    assert_eq!(
        views[2]
            .blocks
            .iter()
            .filter(|block| matches!(block, StructuredBlock::MarkdownSection { .. }))
            .count(),
        4
    );

    let handoff = adapter
        .build_handoff(record, &capture)
        .expect("build OpenSpec handoff");
    assert_eq!(
        handoff.primary_source_path.as_deref(),
        Some("openspec/changes/search/tasks.md")
    );
    assert!(
        handoff
            .continuation_prompt
            .contains("OpenSpec change: search")
    );
    assert!(
        handoff
            .continuation_prompt
            .contains("1 of 2 tasks complete")
    );
    assert!(
        handoff
            .continuation_prompt
            .contains("Adapter parity (tasks.md:4)")
    );
}

#[test]
fn current_done_archived_and_malformed_progress_keep_legacy_status_semantics() {
    let adapter = OpenSpecAdapter::new();
    let current = adapter
        .detect(&current_inventory())
        .expect("detect current")
        .remove(0);
    let done = adapter
        .summarize(&current, &current_capture("# Tasks\n\n- [x] Complete\n"))
        .expect("summarize done current change");
    assert_eq!(fact_text(&done, "openspec.primary_status"), Some("done"));

    let archived_inventory = inventory(&[
        "openspec/changes/archive/2026-08-14-search/proposal.md",
        "openspec/changes/archive/2026-08-14-search/tasks.md",
    ]);
    let archived = adapter
        .detect(&archived_inventory)
        .expect("detect archived")
        .remove(0);
    let archived_capture = RecordSourceCapture::complete(vec![
        snapshot(
            "openspec/changes/archive/2026-08-14-search/proposal.md",
            "## Why\n\nArchived context.\n",
            1,
        ),
        snapshot(
            "openspec/changes/archive/2026-08-14-search/tasks.md",
            "# Tasks\n\n- [ ] Still open\n",
            2,
        ),
    ]);
    let archived_summary = adapter
        .summarize(&archived, &archived_capture)
        .expect("summarize archive");
    assert_eq!(archived.adapter_record_key, ARCHIVED);
    assert_eq!(archived.display_name, "search");
    assert_eq!(
        fact_text(&archived_summary, "openspec.custody"),
        Some("archived")
    );
    assert_eq!(
        fact_text(&archived_summary, "openspec.primary_status"),
        Some("archived")
    );

    let malformed = adapter
        .summarize(
            &current,
            &current_capture("# Tasks\n\n- [?] Unknown\nTasks elsewhere.\n"),
        )
        .expect("summarize malformed tasks");
    assert_eq!(
        fact_text(&malformed, "openspec.primary_status"),
        Some("active")
    );
    assert_eq!(fact_count(&malformed, "openspec.task.total_count"), None);
    assert!(
        malformed
            .warnings
            .iter()
            .any(|warning| warning.code == "openspec_progress_unavailable")
    );
}

#[test]
fn current_archive_movement_and_adapter_upgrades_follow_exact_locator_identity() {
    let current = RecordLocator::new("project_1", "openspec", CURRENT);
    let archived = RecordLocator::new("project_1", "openspec", ARCHIVED);
    let duplicate_archive = RecordLocator::new(
        "project_1",
        "openspec",
        "openspec/changes/archive/2026-08-15-search",
    );
    let upgraded = AdapterDescriptor::new("openspec-v2", "openspec", 2, 10);

    assert_ne!(current.subject_id(), archived.subject_id());
    assert_ne!(archived.subject_id(), duplicate_archive.subject_id());
    assert_eq!(
        RecordLocator::new("project_1", upgraded.format_id(), CURRENT).subject_id(),
        current.subject_id()
    );
}

#[test]
fn incomplete_scan_snapshot_exposes_supported_status_without_inventing_progress() {
    let adapter = OpenSpecAdapter::new();
    let record = adapter
        .detect(&current_inventory())
        .expect("detect current")
        .remove(0);
    let capture = RecordSourceCapture::partial(
        vec![snapshot(
            "openspec/changes/search/proposal.md",
            "## Why\n\nReadable.\n",
            1,
        )],
        vec![SourceCaptureFailure::new(
            "openspec/changes/search/tasks.md",
            "source_unavailable",
            "tasks could not be captured",
        )],
    );

    let summary = adapter
        .summarize(&record, &capture)
        .expect("summarize partial scan");

    assert_eq!(fact_text(&summary, "openspec.custody"), Some("current"));
    assert_eq!(
        fact_text(&summary, "openspec.primary_status"),
        Some("active")
    );
    assert_eq!(fact_count(&summary, "openspec.task.total_count"), None);
    assert!(summary.fingerprint.is_none());
    assert!(
        summary
            .warnings
            .iter()
            .any(|warning| warning.code == "incomplete_source_snapshot")
    );
    assert!(
        summary
            .warnings
            .iter()
            .any(|warning| warning.code == "openspec_progress_unavailable")
    );
}

#[test]
fn malformed_or_partial_details_keep_every_safe_source_available() {
    let adapter = OpenSpecAdapter::new();
    let record = adapter
        .detect(&current_inventory())
        .expect("detect current")
        .remove(0);
    let capture = RecordSourceCapture::partial(
        vec![
            snapshot(
                "openspec/changes/search/proposal.md",
                "## Unsupported\n\nNo canonical overview.\n",
                1,
            ),
            snapshot(
                "openspec/changes/search/tasks.md",
                "# Tasks\n\n- [?] Unknown\n",
                2,
            ),
        ],
        vec![SourceCaptureFailure::new(
            "openspec/changes/search/design.md",
            "source_unavailable",
            "design unavailable",
        )],
    );

    let views = adapter
        .build_detail(&record, &capture)
        .expect("build partial detail");
    let overview = views
        .iter()
        .find(|view| view.capability.id == "overview")
        .expect("overview view");
    let tasks = views
        .iter()
        .find(|view| view.capability.id == "tasks")
        .expect("tasks view");
    let source = views
        .iter()
        .find(|view| view.capability.id == "source")
        .expect("source view");

    assert!(
        overview
            .blocks
            .iter()
            .any(|block| matches!(block, StructuredBlock::EmptyState { .. }))
    );
    assert!(
        tasks
            .blocks
            .iter()
            .any(|block| matches!(block, StructuredBlock::EmptyState { .. }))
    );
    assert_eq!(
        source
            .blocks
            .iter()
            .filter(|block| matches!(block, StructuredBlock::MarkdownSection { .. }))
            .count(),
        2
    );
    assert!(source.blocks.iter().any(|block| matches!(
        block,
        StructuredBlock::Warning { warning, .. }
            if warning.source_path.as_deref() == Some("openspec/changes/search/design.md")
    )));
}
