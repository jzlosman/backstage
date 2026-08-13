use backstage_app_lib::index::{CompletionDisposition, IndexSnapshot, ScanCoordinator};

fn snapshot(root_id: &str, generation: u64) -> IndexSnapshot {
    IndexSnapshot {
        root_id: root_id.to_owned(),
        generation,
        indexed_at: format!("generation-{generation}"),
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
