mod support;

use std::fs;
use std::path::Path;

use backstage_app_lib::filesystem::{ContainedReader, ReadError};
use backstage_core::fingerprint_snapshots;
use tempfile::TempDir;

use support::FixtureRepo;

#[test]
fn contained_reader_rejects_relative_and_traversal_paths() {
    let fixture = FixtureRepo::open_spec();
    let before = fixture.manifest();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");

    assert!(matches!(
        reader.read_text(Path::new("openspec/changes/ship-search/tasks.md")),
        Err(ReadError::PathMustBeAbsolute)
    ));
    assert!(matches!(
        reader.read_text(fixture.path().join("../outside.md")),
        Err(ReadError::Unavailable { .. }) | Err(ReadError::OutsideApprovedRoot { .. })
    ));

    fixture.assert_unchanged(&before);
}

#[cfg(unix)]
#[test]
fn contained_reader_rejects_symlinks_that_resolve_outside_root() {
    use std::os::unix::fs::symlink;

    let fixture = FixtureRepo::open_spec();
    let outside = TempDir::new().expect("outside tempdir");
    let secret = outside.path().join("secret.md");
    fs::write(&secret, "must not be read").expect("outside fixture write");
    let escaped = fixture.path().join("escaped.md");
    symlink(&secret, &escaped).expect("create escape symlink");
    let before = fixture.manifest();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");

    let error = reader
        .read_text(&escaped)
        .expect_err("escape must be rejected");
    let metadata_error = reader
        .regular_file_modified_unix_nanos(&escaped)
        .expect_err("escaped metadata must be rejected");

    assert!(matches!(error, ReadError::OutsideApprovedRoot { .. }));
    assert!(matches!(
        metadata_error,
        ReadError::NotAFile { .. } | ReadError::OutsideApprovedRoot { .. }
    ));
    fixture.assert_unchanged(&before);
}

#[test]
fn contained_reader_returns_bounded_utf8_without_mutation() {
    let fixture = FixtureRepo::open_spec();
    let before = fixture.manifest();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");
    let tasks = fixture.path().join("openspec/changes/ship-search/tasks.md");

    let content = reader.read_text(&tasks).expect("read tasks");
    let modified = reader
        .regular_file_modified_unix_nanos(&tasks)
        .expect("observe tasks metadata");

    assert!(content.contains("Filter bundles"));
    assert!(modified.is_some());
    fixture.assert_unchanged(&before);
}

#[test]
fn contained_reader_creates_a_relative_immutable_snapshot() {
    let fixture = FixtureRepo::open_spec();
    let before = fixture.manifest();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");
    let tasks = fixture.path().join("openspec/changes/ship-search/tasks.md");

    let snapshot = reader.read_snapshot(&tasks).expect("snapshot tasks");

    assert_eq!(
        snapshot.relative_path(),
        "openspec/changes/ship-search/tasks.md"
    );
    assert!(
        snapshot
            .text()
            .expect("UTF-8 snapshot")
            .contains("Filter bundles")
    );
    assert!(
        fingerprint_snapshots(&[snapshot])
            .as_str()
            .starts_with("sha256:")
    );
    fixture.assert_unchanged(&before);
}

#[test]
fn contained_reader_rejects_oversized_files_before_reading_content() {
    let fixture = FixtureRepo::open_spec();
    let before = fixture.manifest();
    let reader = ContainedReader::approve(fixture.path(), 8).expect("approve fixture");
    let tasks = fixture.path().join("openspec/changes/ship-search/tasks.md");

    assert!(matches!(
        reader.read_text(&tasks),
        Err(ReadError::FileTooLarge { .. })
    ));
    fixture.assert_unchanged(&before);
}
