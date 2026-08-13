use backstage_core::{SnapshotError, SourceObservation, SourceSnapshot, fingerprint_snapshots};

fn snapshot(path: &str, content: &str, modified: u128) -> SourceSnapshot {
    SourceSnapshot::from_observations(
        path,
        content.as_bytes().to_vec(),
        SourceObservation {
            byte_len: content.len() as u64,
            modified_unix_nanos: Some(modified),
        },
        SourceObservation {
            byte_len: content.len() as u64,
            modified_unix_nanos: Some(modified),
        },
    )
    .expect("stable snapshot")
}

#[test]
fn snapshot_rejects_absolute_and_traversing_relative_paths() {
    let observation = SourceObservation {
        byte_len: 4,
        modified_unix_nanos: None,
    };

    assert!(matches!(
        SourceSnapshot::from_observations(
            "/tmp/file.md",
            b"text".to_vec(),
            observation,
            observation
        ),
        Err(SnapshotError::InvalidRelativePath)
    ));
    assert!(matches!(
        SourceSnapshot::from_observations("../file.md", b"text".to_vec(), observation, observation),
        Err(SnapshotError::InvalidRelativePath)
    ));
}

#[test]
fn snapshot_detects_a_source_change_race() {
    let error = SourceSnapshot::from_observations(
        "tasks.md",
        b"- [ ] task".to_vec(),
        SourceObservation {
            byte_len: 10,
            modified_unix_nanos: Some(1),
        },
        SourceObservation {
            byte_len: 11,
            modified_unix_nanos: Some(2),
        },
    )
    .expect_err("raced source must fail");

    assert_eq!(error, SnapshotError::SourceChangedDuringRead);
}

#[test]
fn bundle_fingerprint_is_ordered_by_normalized_path_not_input_order() {
    let proposal = snapshot("proposal.md", "proposal", 1);
    let tasks = snapshot("tasks.md", "tasks", 1);

    assert_eq!(
        fingerprint_snapshots(&[proposal.clone(), tasks.clone()]),
        fingerprint_snapshots(&[tasks, proposal])
    );
}

#[test]
fn content_and_membership_change_the_bundle_fingerprint() {
    let proposal = snapshot("proposal.md", "proposal", 1);
    let tasks = snapshot("tasks.md", "tasks", 1);
    let changed_tasks = snapshot("tasks.md", "changed tasks", 2);

    let original = fingerprint_snapshots(&[proposal.clone(), tasks]);
    assert_ne!(
        original,
        fingerprint_snapshots(&[proposal.clone(), changed_tasks])
    );
    assert_ne!(original, fingerprint_snapshots(&[proposal]));
}

#[test]
fn timestamp_only_change_keeps_the_fingerprint_current() {
    let first = snapshot("tasks.md", "same content", 1);
    let touched = snapshot("tasks.md", "same content", 9_999);

    assert_eq!(
        fingerprint_snapshots(&[first]),
        fingerprint_snapshots(&[touched])
    );
}
