use backstage_app_lib::index::{
    CURRENT_INDEX_SCHEMA_VERSION, CompletionDisposition, IndexSnapshot, ScanCoordinator,
};

fn snapshot(root_id: &str, generation: u64) -> IndexSnapshot {
    IndexSnapshot {
        schema_version: CURRENT_INDEX_SCHEMA_VERSION,
        root_id: root_id.to_owned(),
        generation,
        indexed_at: format!("generation-{generation}"),
        configuration_revision: 0,
        projects: vec![],
        warnings: vec![],
    }
}

#[test]
fn a_superseded_scan_cannot_replace_a_newer_generation() {
    let coordinator = ScanCoordinator::default();
    let first = coordinator.begin("root_1");
    let second = coordinator.begin("root_1");

    assert_eq!(
        coordinator.complete(&first, snapshot("root_1", first.generation)),
        CompletionDisposition::Superseded
    );
    assert!(coordinator.current("root_1").is_none());

    assert_eq!(
        coordinator.complete(&second, snapshot("root_1", second.generation)),
        CompletionDisposition::Accepted
    );
    assert_eq!(
        coordinator.current("root_1").expect("current").generation,
        2
    );
}

#[test]
fn scan_failure_preserves_the_prior_usable_index() {
    let coordinator = ScanCoordinator::default();
    let first = coordinator.begin("root_1");
    coordinator.complete(&first, snapshot("root_1", first.generation));
    let refresh = coordinator.begin("root_1");

    coordinator.fail(&refresh, "root unavailable");

    let current = coordinator.current("root_1").expect("prior index");
    assert_eq!(current.generation, 1);
    assert_eq!(
        coordinator.failure("root_1"),
        Some("root unavailable".to_owned())
    );
}

#[test]
fn configuration_revision_supersedes_older_scans_and_is_published_with_the_snapshot() {
    let coordinator = ScanCoordinator::default();
    let old = coordinator.begin_for_revision("root_1", 4);
    let current = coordinator.begin_for_revision("root_1", 5);
    let mut old_snapshot = snapshot("root_1", old.generation);
    old_snapshot.configuration_revision = old.configuration_revision;
    let mut current_snapshot = snapshot("root_1", current.generation);
    current_snapshot.configuration_revision = current.configuration_revision;

    assert_eq!(
        coordinator.complete(&old, old_snapshot),
        CompletionDisposition::Superseded
    );
    assert_eq!(
        coordinator.complete(&current, current_snapshot),
        CompletionDisposition::Accepted
    );
    assert_eq!(
        coordinator
            .current("root_1")
            .expect("current revision")
            .configuration_revision,
        5
    );
}

#[test]
fn begin_reports_when_a_revision_was_not_admitted() {
    let coordinator = ScanCoordinator::default();
    let current = coordinator.begin_for_revision("root_1", 5);
    let stale = coordinator.begin_for_revision("root_1", 4);

    assert!(current.admitted);
    assert!(!stale.admitted);
}

#[test]
fn an_older_revision_cannot_supersede_a_newer_revision_even_if_it_begins_later() {
    let coordinator = ScanCoordinator::default();
    let current = coordinator.begin_for_revision("root_1", 5);
    let delayed_old = coordinator.begin_for_revision("root_1", 4);
    let mut current_snapshot = snapshot("root_1", current.generation);
    current_snapshot.configuration_revision = 5;
    let mut delayed_old_snapshot = snapshot("root_1", delayed_old.generation);
    delayed_old_snapshot.configuration_revision = 4;

    assert_eq!(
        coordinator.complete(&current, current_snapshot),
        CompletionDisposition::Accepted
    );
    assert_eq!(
        coordinator.complete(&delayed_old, delayed_old_snapshot),
        CompletionDisposition::Superseded
    );
    assert_eq!(
        coordinator
            .current("root_1")
            .expect("newer revision remains")
            .configuration_revision,
        5
    );
}

#[test]
fn hydrating_an_older_cached_snapshot_cannot_replace_newer_runtime_state() {
    let coordinator = ScanCoordinator::default();
    let current = coordinator.begin_for_revision("root_1", 5);
    let mut current_snapshot = snapshot("root_1", current.generation);
    current_snapshot.configuration_revision = 5;
    assert_eq!(
        coordinator.complete(&current, current_snapshot.clone()),
        CompletionDisposition::Accepted
    );
    let mut stale = snapshot("root_1", current.generation + 10);
    stale.configuration_revision = 4;

    coordinator.hydrate(stale);

    assert_eq!(coordinator.current("root_1"), Some(current_snapshot));
}

#[test]
fn forgetting_a_root_discards_current_state_and_rejects_delayed_completion() {
    let coordinator = ScanCoordinator::default();
    let published = coordinator.begin_for_revision("root_1", 1);
    let mut published_snapshot = snapshot("root_1", published.generation);
    published_snapshot.configuration_revision = 1;
    coordinator.complete(&published, published_snapshot);
    let delayed = coordinator.begin_for_revision("root_1", 1);

    coordinator.forget("root_1");

    assert!(coordinator.current("root_1").is_none());
    assert!(coordinator.failure("root_1").is_none());
    let mut delayed_snapshot = snapshot("root_1", delayed.generation);
    delayed_snapshot.configuration_revision = 1;
    assert_eq!(
        coordinator.complete(&delayed, delayed_snapshot),
        CompletionDisposition::Superseded
    );
    assert!(coordinator.current("root_1").is_none());
}

#[test]
fn a_successful_recovery_replaces_unavailable_state() {
    let coordinator = ScanCoordinator::default();
    let first = coordinator.begin("root_1");
    coordinator.fail(&first, "root unavailable");
    let recovery = coordinator.begin("root_1");

    coordinator.complete(&recovery, snapshot("root_1", recovery.generation));

    assert!(coordinator.failure("root_1").is_none());
    assert_eq!(
        coordinator.current("root_1").expect("recovered").generation,
        2
    );
}
