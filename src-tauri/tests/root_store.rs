mod support;

use backstage_app_lib::storage::SqliteStore;
use backstage_app_lib::{approve_root_path, list_approved_roots, remove_approved_root};
use tempfile::TempDir;

use support::FixtureRepo;

#[test]
fn root_approval_is_persisted_by_stable_id_in_app_owned_storage() {
    let fixture = FixtureRepo::open_spec();
    let app_data = TempDir::new().expect("app data tempdir");
    let database = app_data.path().join("index.sqlite3");
    let before = fixture.manifest();
    let store = SqliteStore::open(&database).expect("open app store");

    let approved = approve_root_path(&store, fixture.path()).expect("approve root");
    let listed = list_approved_roots(&store).expect("list roots");

    assert_eq!(listed, vec![approved.clone()]);
    assert!(database.is_file());
    assert!(!database.starts_with(fixture.path()));

    remove_approved_root(&store, approved.id()).expect("remove root");
    assert!(list_approved_roots(&store).expect("list roots").is_empty());
    fixture.assert_unchanged(&before);
}

#[test]
fn root_approval_rejects_files_and_relative_paths_without_persisting() {
    let fixture = FixtureRepo::open_spec();
    let store = SqliteStore::in_memory().expect("open memory store");
    let before = fixture.manifest();

    assert!(approve_root_path(&store, "relative").is_err());
    assert!(approve_root_path(&store, fixture.path().join("README.md")).is_err());
    assert!(list_approved_roots(&store).expect("list roots").is_empty());
    fixture.assert_unchanged(&before);
}
