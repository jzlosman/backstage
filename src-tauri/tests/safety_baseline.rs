mod support;

use backstage_app_lib::app_paths::AppPaths;
use tempfile::TempDir;

use support::FixtureRepo;

#[test]
fn startup_paths_are_created_only_under_app_owned_base() {
    let fixture = FixtureRepo::open_spec();
    let app_base = TempDir::new().expect("app tempdir");
    let before = fixture.manifest();

    let paths = AppPaths::under(app_base.path());
    paths.ensure_exists().expect("app paths should initialize");

    assert!(paths.config_dir().is_dir());
    assert!(paths.cache_dir().is_dir());
    assert!(paths.database_path().starts_with(app_base.path()));
    fixture.assert_unchanged(&before);
}

#[test]
fn fixture_contains_git_and_representative_openspec_material() {
    let fixture = FixtureRepo::open_spec();
    let before = fixture.manifest();

    assert!(fixture.path().join(".git").is_dir());
    assert!(
        fixture
            .path()
            .join("openspec/changes/ship-search/tasks.md")
            .is_file()
    );
    assert!(fixture.path().join("PLAN.md").is_file());

    fixture.assert_unchanged(&before);
}
