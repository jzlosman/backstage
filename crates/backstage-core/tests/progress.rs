use backstage_core::{OpenSpecProgress, parse_openspec_tasks};

#[test]
fn parses_exact_mixed_task_facts_and_source_locations() {
    let source = "# Tasks\n\n- [x] Parse query\n  - [ ] Filter bundles\n* [ ] Restore focus\n";

    let OpenSpecProgress::Available(progress) = parse_openspec_tasks(source) else {
        panic!("progress should be available");
    };

    assert_eq!(progress.total, 3);
    assert_eq!(progress.completed, 1);
    assert_eq!(progress.remaining.len(), 2);
    assert_eq!(progress.remaining[0].text, "Filter bundles");
    assert_eq!(progress.remaining[0].location.line, 4);
    assert_eq!(progress.remaining[0].location.column, 5);
    assert_eq!(progress.parser.name, "openspec-task-markers");
    assert_eq!(progress.parser.version, "1");
}

#[test]
fn ignores_task_markers_inside_fenced_code() {
    let source = "# Tasks\n\n```md\n- [x] Example only\n```\n\n- [ ] Real task\n";

    let OpenSpecProgress::Available(progress) = parse_openspec_tasks(source) else {
        panic!("progress should be available");
    };

    assert_eq!(progress.total, 1);
    assert_eq!(progress.completed, 0);
    assert_eq!(progress.remaining[0].text, "Real task");
}

#[test]
fn returns_unavailable_instead_of_inventing_progress() {
    let OpenSpecProgress::Unavailable(fallback) =
        parse_openspec_tasks("# Tasks\n\nTasks are tracked elsewhere.\n")
    else {
        panic!("progress should be unavailable");
    };

    assert_eq!(fallback.parser.name, "openspec-task-markers");
    assert!(fallback.warnings.is_empty());
}

#[test]
fn malformed_markers_degrade_to_a_parse_warning() {
    let OpenSpecProgress::Unavailable(fallback) =
        parse_openspec_tasks("# Tasks\n\n- [~] Ambiguous state\n")
    else {
        panic!("progress should be unavailable");
    };

    assert_eq!(fallback.warnings.len(), 1);
    assert_eq!(fallback.warnings[0].line, 3);
    assert!(
        fallback.warnings[0]
            .message
            .contains("unsupported task marker")
    );
}
