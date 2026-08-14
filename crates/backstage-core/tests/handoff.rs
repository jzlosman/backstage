use backstage_core::{
    ArtifactRecognition, BundleKind, HandoffContext, OpenSpecCustody, OpenSpecProgress,
    ParserProvenance, ProgressFallback, SourceLocation, TaskFact, TaskProgress,
    continuation_prompt,
};

fn context(progress: OpenSpecProgress) -> HandoffContext {
    HandoffContext {
        project_path: "/Users/dev/workbench".to_owned(),
        project_name: "Workbench".to_owned(),
        bundle_name: "ship-search".to_owned(),
        artifact_path: "/Users/dev/workbench/openspec/changes/ship-search/tasks.md".to_owned(),
        bundle_kind: BundleKind::OpenSpecChange,
        recognition: ArtifactRecognition::Recognized {
            detector: "openspec-v1".to_owned(),
        },
        custody: Some(OpenSpecCustody::Current),
        progress,
        warnings: vec!["Git metadata unavailable".to_owned()],
    }
}

#[test]
fn prompt_includes_paths_deterministic_status_and_explicit_continuation() {
    let task = TaskFact {
        text: "Filter bundles".to_owned(),
        completed: false,
        location: SourceLocation { line: 8, column: 3 },
    };
    let prompt = continuation_prompt(&context(OpenSpecProgress::Available(TaskProgress {
        total: 2,
        completed: 1,
        remaining_count: 1,
        tasks: vec![task.clone()],
        remaining: vec![task],
        parser: ParserProvenance {
            name: "openspec-task-markers".to_owned(),
            version: "1".to_owned(),
        },
        warnings: vec![],
    })));

    assert!(prompt.contains("/Users/dev/workbench"));
    assert!(prompt.contains("openspec/changes/ship-search/tasks.md"));
    assert!(prompt.contains("1 of 2 tasks complete"));
    assert!(prompt.contains("Filter bundles (tasks.md:8)"));
    assert!(prompt.contains("Inspect the source files before continuing"));
    assert!(prompt.contains("Do not modify repository content unless the user explicitly asks"));
}

#[test]
fn archived_prompt_identifies_custody_and_exact_source_path() {
    let mut archived = context(OpenSpecProgress::Unavailable(ProgressFallback {
        parser: ParserProvenance {
            name: "openspec-task-markers".to_owned(),
            version: "1".to_owned(),
        },
        warnings: vec![],
    }));
    archived.artifact_path =
        "/Users/dev/workbench/openspec/changes/archive/2026-08-13-ship-search/tasks.md".to_owned();
    archived.custody = Some(OpenSpecCustody::Archived {
        archived_on: Some("2026-08-13".to_owned()),
    });

    let prompt = continuation_prompt(&archived);

    assert!(prompt.contains("Custody: Archived on 2026-08-13"));
    assert!(prompt.contains(&archived.artifact_path));
    assert!(!prompt.contains("Custody: Current"));
}

#[test]
fn prompt_omits_generated_claims_and_labels_progress_unavailable() {
    let prompt = continuation_prompt(&context(OpenSpecProgress::Unavailable(ProgressFallback {
        parser: ParserProvenance {
            name: "openspec-task-markers".to_owned(),
            version: "1".to_owned(),
        },
        warnings: vec![],
    })));

    assert!(prompt.contains("Progress unavailable"));
    assert!(!prompt.contains("Pi-generated"));
    assert!(!prompt.contains("Summary"));
}
