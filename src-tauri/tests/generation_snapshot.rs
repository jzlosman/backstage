mod support;

use backstage_app_lib::filesystem::ContainedReader;
use backstage_app_lib::generation::{GenerationLimits, build_generation_snapshot};
use backstage_core::{GenerationMode, SnapshotError};
use tempfile::TempDir;

use support::FixtureRepo;

#[test]
fn bounded_snapshot_contains_relative_untrusted_source_and_provenance() {
    let fixture = FixtureRepo::open_spec();
    let before = fixture.manifest();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");
    let paths = vec![
        fixture
            .path()
            .join("openspec/changes/ship-search/proposal.md"),
        fixture.path().join("openspec/changes/ship-search/tasks.md"),
    ];

    let snapshot = build_generation_snapshot(
        &reader,
        &paths,
        GenerationMode::Summary,
        "summary-v1",
        &GenerationLimits {
            max_files: 4,
            max_bytes: 64 * 1024,
        },
    )
    .expect("bounded snapshot");

    assert_eq!(snapshot.included_paths.len(), 2);
    assert!(snapshot.envelope.contains("untrusted quoted source"));
    assert!(
        snapshot
            .envelope
            .contains("path=\"openspec/changes/ship-search/tasks.md\"")
    );
    assert!(
        !snapshot
            .envelope
            .contains(fixture.path().to_string_lossy().as_ref())
    );
    assert_eq!(snapshot.prompt_version, "summary-v1");
    fixture.assert_unchanged(&before);
}

#[test]
fn over_limit_scope_spawns_nothing_and_returns_scope_error() {
    let fixture = FixtureRepo::open_spec();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");
    let paths = vec![
        fixture
            .path()
            .join("openspec/changes/ship-search/proposal.md"),
        fixture.path().join("openspec/changes/ship-search/tasks.md"),
    ];

    let error = build_generation_snapshot(
        &reader,
        &paths,
        GenerationMode::Summary,
        "summary-v1",
        &GenerationLimits {
            max_files: 1,
            max_bytes: 64 * 1024,
        },
    )
    .expect_err("file limit");

    assert_eq!(error, SnapshotError::IncompleteManifest);
}

#[cfg(unix)]
#[test]
fn escaped_scope_is_rejected_before_outside_content_is_read() {
    use std::fs;
    use std::os::unix::fs::symlink;

    let fixture = FixtureRepo::open_spec();
    let outside = TempDir::new().expect("outside tempdir");
    let secret = outside.path().join("secret.md");
    fs::write(&secret, "outside secret").expect("outside file");
    let link = fixture.path().join("escaped.md");
    symlink(&secret, &link).expect("escape link");
    let before = fixture.manifest();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");

    assert!(
        build_generation_snapshot(
            &reader,
            &[link],
            GenerationMode::Summary,
            "summary-v1",
            &GenerationLimits::default(),
        )
        .is_err()
    );
    fixture.assert_unchanged(&before);
}
