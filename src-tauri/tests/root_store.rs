mod support;

use backstage_app_lib::discovery::ProjectCandidate;
use backstage_app_lib::index::{IndexSnapshot, IndexedProject};
use backstage_app_lib::storage::{SqliteStore, StoreError};
use backstage_app_lib::{approve_root_path, list_approved_roots, remove_approved_root};
use backstage_core::{GeneratedResult, GenerationMode, SourceFingerprint};
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

fn empty_index(root_id: &str, bundle_ids: &[&str]) -> IndexSnapshot {
    let bundles = bundle_ids
        .iter()
        .map(|bundle_id| backstage_app_lib::index::IndexedBundle {
            bundle: backstage_core::ArtifactBundle {
                id: (*bundle_id).to_owned(),
                project_id: "project_shared".to_owned(),
                project_name: "Shared".to_owned(),
                name: format!("{bundle_id}.md"),
                kind: backstage_core::BundleKind::PossibleArtifact,
                recognition: backstage_core::ArtifactRecognition::Possible {
                    reason: "test fixture".to_owned(),
                },
                members: vec![],
                custody: None,
            },
            progress: backstage_core::OpenSpecProgress::Unavailable(
                backstage_core::ProgressFallback {
                    parser: backstage_core::ParserProvenance {
                        name: "test".to_owned(),
                        version: "1".to_owned(),
                    },
                    warnings: vec![],
                },
            ),
            primary_status: None,
            fingerprint: None,
            source_modified_unix_nanos: None,
            warnings: vec![],
        })
        .collect();
    IndexSnapshot {
        root_id: root_id.to_owned(),
        generation: 1,
        indexed_at: "2026-08-14T00:00:00Z".to_owned(),
        configuration_revision: 0,
        projects: vec![IndexedProject {
            project: ProjectCandidate {
                id: "project_shared".to_owned(),
                name: "Shared".to_owned(),
                root_path: "/tmp/shared".to_owned(),
                git: None,
            },
            bundles,
            markdown_documents: vec![],
        }],
        warnings: vec![],
    }
}

fn generated_result() -> GeneratedResult {
    GeneratedResult {
        text: "summary".to_owned(),
        mode: GenerationMode::Summary,
        source_fingerprint: SourceFingerprint::from_trusted("sha256:test"),
        included_paths: vec![],
        generated_at: "2026-08-14T00:00:00Z".to_owned(),
        model: None,
        prompt_version: "summary-v1".to_owned(),
    }
}

#[test]
fn coordinated_removal_returns_authoritative_inventory_and_prunes_only_unreachable_views() {
    let first_dir = TempDir::new().expect("first root");
    let second_dir = TempDir::new().expect("second root");
    let store = SqliteStore::in_memory().expect("store");
    let first = approve_root_path(&store, first_dir.path()).expect("first approval");
    let second = approve_root_path(&store, second_dir.path()).expect("second approval");
    store
        .save_index(&empty_index(
            first.id(),
            &["bundle_shared", "bundle_unique"],
        ))
        .expect("first index");
    store
        .save_index(&empty_index(second.id(), &["bundle_shared"]))
        .expect("second index");
    store
        .save_generated_view("bundle_shared", &generated_result())
        .expect("shared generated view");
    store
        .save_generated_view("bundle_unique", &generated_result())
        .expect("unique generated view");

    let inventory = store
        .remove_root_state(first.id())
        .expect("coordinated removal");

    assert_eq!(inventory.roots, vec![second.clone()]);
    assert_eq!(inventory.indexes.len(), 1);
    assert_eq!(inventory.indexes[0].root_id, second.id());
    assert!(
        store
            .load_index(first.id())
            .expect("removed index")
            .is_none()
    );
    assert!(
        store
            .find_latest_generated_view("bundle_shared", GenerationMode::Summary, "summary-v1")
            .expect("shared lookup")
            .is_some()
    );
    assert!(
        store
            .find_latest_generated_view("bundle_unique", GenerationMode::Summary, "summary-v1")
            .expect("unique lookup")
            .is_none()
    );
}

#[test]
fn removal_prefers_retained_current_indexes_when_pruning_generated_views() {
    let removed_dir = TempDir::new().expect("removed root");
    let retained_dir = TempDir::new().expect("retained root");
    let store = SqliteStore::in_memory().expect("store");
    let removed = approve_root_path(&store, removed_dir.path()).expect("removed approval");
    let retained = approve_root_path(&store, retained_dir.path()).expect("retained approval");
    store
        .save_index(&empty_index(removed.id(), &["bundle_removed"]))
        .expect("removed index");
    store
        .save_index(&empty_index(retained.id(), &[]))
        .expect("stale retained index");
    let mut current_retained = empty_index(retained.id(), &["bundle_memory_only"]);
    current_retained.generation = 2;
    store
        .save_generated_view("bundle_memory_only", &generated_result())
        .expect("generated view");

    let inventory = store
        .remove_root_state_with_retained_indexes(removed.id(), &[current_retained.clone()])
        .expect("coordinated removal");

    assert_eq!(inventory.indexes, vec![current_retained]);
    assert!(
        store
            .find_latest_generated_view("bundle_memory_only", GenerationMode::Summary, "summary-v1")
            .expect("generated lookup")
            .is_some()
    );
}

#[test]
fn removing_an_unknown_root_is_explicit_not_found_and_preserves_state() {
    let fixture = FixtureRepo::open_spec();
    let store = SqliteStore::in_memory().expect("store");
    let approved = approve_root_path(&store, fixture.path()).expect("approval");

    let error = store
        .remove_root_state("root_unknown")
        .expect_err("unknown root");

    assert!(matches!(error, StoreError::RootNotFound(id) if id == "root_unknown"));
    assert_eq!(list_approved_roots(&store).expect("roots"), vec![approved]);
}

#[test]
fn removal_transaction_rolls_back_root_index_and_generated_views_on_database_failure() {
    let root = TempDir::new().expect("root");
    let app_data = TempDir::new().expect("app data");
    let database = app_data.path().join("index.sqlite3");
    let store = SqliteStore::open(&database).expect("store");
    let approved = approve_root_path(&store, root.path()).expect("approval");
    store
        .save_index(&empty_index(approved.id(), &["bundle_unique"]))
        .expect("index");
    store
        .save_generated_view("bundle_unique", &generated_result())
        .expect("generated view");
    let connection = rusqlite::Connection::open(&database).expect("second connection");
    connection
        .execute_batch(
            "CREATE TRIGGER reject_root_delete BEFORE DELETE ON approved_roots
             BEGIN SELECT RAISE(ABORT, 'injected removal failure'); END;",
        )
        .expect("failure trigger");

    assert!(store.remove_root_state(approved.id()).is_err());

    assert_eq!(
        list_approved_roots(&store).expect("roots"),
        vec![approved.clone()]
    );
    assert!(store.load_index(approved.id()).expect("index").is_some());
    assert!(
        store
            .find_latest_generated_view("bundle_unique", GenerationMode::Summary, "summary-v1")
            .expect("generated lookup")
            .is_some()
    );
}
