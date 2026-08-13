mod support;

use std::fs;

use backstage_app_lib::discovery::{CancellationToken, ScanPolicy, discover_projects};
use backstage_app_lib::filesystem::ContainedReader;

use support::FixtureRepo;

#[test]
fn discovery_finds_git_working_trees_without_mutating_them() {
    let fixture = FixtureRepo::open_spec();
    let before = fixture.manifest();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");

    let result = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());

    assert_eq!(result.projects.len(), 1);
    assert_eq!(
        result.projects[0].root_path,
        reader.root().to_string_lossy()
    );
    assert!(!result.cancelled);
    fixture.assert_unchanged(&before);
}

#[test]
fn discovery_honors_cancellation_and_preserves_repository() {
    let fixture = FixtureRepo::open_spec();
    let before = fixture.manifest();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");
    let cancellation = CancellationToken::new();
    cancellation.cancel();

    let result = discover_projects(&reader, &ScanPolicy::default(), &cancellation);

    assert!(result.cancelled);
    assert!(result.projects.is_empty());
    fixture.assert_unchanged(&before);
}

#[test]
fn discovery_reports_bounds_and_symlink_escapes_as_partial_warnings() {
    let fixture = FixtureRepo::open_spec();
    fs::create_dir_all(fixture.path().join("node_modules/ignored/.git")).expect("excluded tree");
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let outside = tempfile::TempDir::new().expect("outside tempdir");
        symlink(outside.path(), fixture.path().join("outside-link")).expect("escape symlink");
        let before = fixture.manifest();
        let reader =
            ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");
        let policy = ScanPolicy {
            max_entries: 3,
            ..ScanPolicy::default()
        };

        let result = discover_projects(&reader, &policy, &CancellationToken::new());

        assert!(result.warnings.iter().any(|warning| {
            warning.code == "entry_limit_reached" || warning.code == "symlink_escape"
        }));
        fixture.assert_unchanged(&before);
    }
}

#[test]
fn malformed_git_metadata_does_not_hide_a_readable_project() {
    let fixture = FixtureRepo::open_spec();
    fs::remove_dir_all(fixture.path().join(".git")).expect("remove fixture metadata");
    fs::write(fixture.path().join(".git"), "malformed gitdir").expect("malformed git marker");
    let before = fixture.manifest();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");

    let result = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());

    assert_eq!(result.projects.len(), 1);
    assert!(result.projects[0].git.is_none());
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.code == "git_unavailable")
    );
    fixture.assert_unchanged(&before);
}
