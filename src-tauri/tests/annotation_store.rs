use std::sync::{Arc, Barrier};

use backstage_app_lib::storage::SqliteStore;
use backstage_core::{
    AdapterDescriptor, Capability, Decision, Disposition, GeneratedResult, GenerationMode,
    RecognitionLevel, RecordLocator, SourceFingerprint, WorkRecord, WorkRecordAnnotation,
    WorkRecordRecognition, WorkRecordSource,
};

fn record(project_id: &str, path: &str, name: &str) -> WorkRecord {
    let descriptor = AdapterDescriptor::new("markdown-v1", "markdown", 1, 40);
    WorkRecord::new(
        RecordLocator::new(project_id, descriptor.format_id(), path),
        name,
        WorkRecordRecognition::new(
            RecognitionLevel::Plain,
            &descriptor,
            vec!["plain Markdown".to_owned()],
        ),
        vec![WorkRecordSource::new(path, Some(1))],
        vec![],
        vec![],
        vec![Capability::new("source", "Source")],
    )
}

#[test]
fn subjects_routes_and_sparse_defaults_are_durable_across_restart_and_absence() {
    let app_data = tempfile::TempDir::new().expect("app data");
    let database = app_data.path().join("backstage.sqlite3");
    let root = backstage_core::ApprovedRoot::new("/tmp", true).expect("root");
    let plan = record("project_1", "PLAN.md", "PLAN.md");

    {
        let store = SqliteStore::open(&database).expect("store");
        store.upsert_root(&root).expect("root");
        store
            .refresh_work_record_subjects(root.id(), std::slice::from_ref(&plan), "seen-1")
            .expect("refresh subjects");
        assert_eq!(
            store
                .work_record_annotation(&plan.subject_id)
                .expect("default annotation"),
            WorkRecordAnnotation::default()
        );
        store
            .save_work_record_annotation(
                &plan.subject_id,
                &WorkRecordAnnotation {
                    decision: Decision::Approved,
                    todo: true,
                    ..WorkRecordAnnotation::default()
                },
                "updated-1",
            )
            .expect("save annotation");
        store
            .refresh_work_record_subjects(root.id(), &[], "seen-2")
            .expect("empty accepted scan must not remove route");
    }

    let restarted = SqliteStore::open(&database).expect("restart store");
    assert_eq!(
        restarted
            .work_record_annotation(&plan.subject_id)
            .expect("durable annotation"),
        WorkRecordAnnotation {
            decision: Decision::Approved,
            todo: true,
            ..WorkRecordAnnotation::default()
        }
    );
    let connection = rusqlite::Connection::open(&database).expect("inspect database");
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM work_record_subject_roots WHERE subject_id = ?1",
                [plan.subject_id.as_str()],
                |row| row.get::<_, usize>(0),
            )
            .expect("route count"),
        1
    );
}

#[test]
fn default_annotations_remain_sparse_and_updates_are_atomic() {
    let app_data = tempfile::TempDir::new().expect("app data");
    let database = app_data.path().join("backstage.sqlite3");
    let root = backstage_core::ApprovedRoot::new("/tmp", true).expect("root");
    let old = record("project_1", "old.md", "Old");
    let replacement = record("project_1", "new.md", "New");
    let store = SqliteStore::open(&database).expect("store");
    store.upsert_root(&root).expect("root");
    store
        .refresh_work_record_subjects(root.id(), &[old.clone(), replacement.clone()], "seen")
        .expect("subjects");

    store
        .save_work_record_annotation(&old.subject_id, &WorkRecordAnnotation::default(), "default")
        .expect("sparse default");
    let connection = rusqlite::Connection::open(&database).expect("inspect database");
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM work_record_annotations", [], |row| {
                row.get::<_, usize>(0)
            })
            .expect("annotation count"),
        0
    );

    let superseded = WorkRecordAnnotation {
        decision: Decision::Approved,
        disposition: Disposition::Superseded {
            replacement: replacement.subject_id.clone(),
        },
        favorite: true,
        ..WorkRecordAnnotation::default()
    };
    store
        .save_work_record_annotation(&old.subject_id, &superseded, "updated")
        .expect("save supersession");
    connection
        .execute_batch(
            "CREATE TRIGGER reject_annotation_update
             BEFORE UPDATE ON work_record_annotations
             BEGIN SELECT RAISE(ABORT, 'injected annotation failure'); END;",
        )
        .expect("failure trigger");
    let attempted = WorkRecordAnnotation {
        favorite: false,
        ..superseded.clone()
    };

    assert!(
        store
            .save_work_record_annotation(&old.subject_id, &attempted, "failed")
            .is_err()
    );
    assert_eq!(
        store
            .work_record_annotation(&old.subject_id)
            .expect("authoritative state after failure"),
        superseded
    );
    assert_eq!(
        store.supersession_edges().expect("supersession graph"),
        vec![(old.subject_id, replacement.subject_id)]
    );
}

#[test]
fn removing_an_unrelated_root_preserves_historical_subject_routes() {
    let first_dir = tempfile::TempDir::new().expect("first root");
    let second_dir = tempfile::TempDir::new().expect("second root");
    let first = backstage_core::ApprovedRoot::new(first_dir.path(), true).expect("first root");
    let second = backstage_core::ApprovedRoot::new(second_dir.path(), true).expect("second root");
    let plan = record("project_1", "PLAN.md", "Plan");
    let store = SqliteStore::in_memory().expect("store");
    store.upsert_root(&first).expect("first approval");
    store.upsert_root(&second).expect("second approval");
    store
        .refresh_work_record_subjects(first.id(), std::slice::from_ref(&plan), "seen")
        .expect("subject route");
    let annotation = WorkRecordAnnotation {
        todo: true,
        ..WorkRecordAnnotation::default()
    };
    store
        .save_work_record_annotation(&plan.subject_id, &annotation, "updated")
        .expect("annotation");
    store
        .save_generated_view(
            &plan.subject_id,
            &GeneratedResult {
                text: "summary".to_owned(),
                mode: GenerationMode::Summary,
                source_fingerprint: SourceFingerprint::from_trusted("sha256:plan"),
                included_paths: vec!["PLAN.md".to_owned()],
                generated_at: "generated".to_owned(),
                model: None,
                prompt_version: "summary-v1".to_owned(),
            },
        )
        .expect("generated view");

    store
        .remove_root_state(second.id())
        .expect("remove unrelated root");

    assert_eq!(
        store
            .work_record_annotation(&plan.subject_id)
            .expect("retained annotation"),
        annotation
    );
    assert!(
        store
            .find_latest_generated_view(&plan.subject_id, GenerationMode::Summary, "summary-v1",)
            .expect("retained generated view")
            .is_some()
    );
}

#[test]
fn root_removal_deletes_only_unrouted_subjects_and_reconciles_supersession() {
    let first_dir = tempfile::TempDir::new().expect("first root");
    let second_dir = tempfile::TempDir::new().expect("second root");
    let first = backstage_core::ApprovedRoot::new(first_dir.path(), true).expect("first root");
    let second = backstage_core::ApprovedRoot::new(second_dir.path(), true).expect("second root");
    let shared = record("project_shared", "SHARED.md", "Shared plan");
    let unique = record("project_unique", "UNIQUE.md", "Unique plan");
    let store = SqliteStore::in_memory().expect("store");
    store.upsert_root(&first).expect("first approval");
    store.upsert_root(&second).expect("second approval");
    store
        .refresh_work_record_subjects(first.id(), &[shared.clone(), unique.clone()], "seen")
        .expect("first routes");
    store
        .refresh_work_record_subjects(second.id(), std::slice::from_ref(&shared), "seen")
        .expect("shared route");
    let annotation = WorkRecordAnnotation {
        disposition: Disposition::Superseded {
            replacement: unique.subject_id.clone(),
        },
        favorite: true,
        ..WorkRecordAnnotation::default()
    };
    store
        .save_work_record_annotation(&shared.subject_id, &annotation, "updated")
        .expect("annotation");
    store
        .save_work_record_annotation(
            &unique.subject_id,
            &WorkRecordAnnotation {
                todo: true,
                ..WorkRecordAnnotation::default()
            },
            "updated",
        )
        .expect("unique annotation");
    store
        .save_generated_view(
            &unique.subject_id,
            &GeneratedResult {
                text: "unique summary".to_owned(),
                mode: GenerationMode::Summary,
                source_fingerprint: SourceFingerprint::from_trusted("sha256:unique"),
                included_paths: vec!["UNIQUE.md".to_owned()],
                generated_at: "generated".to_owned(),
                model: None,
                prompt_version: "summary-v1".to_owned(),
            },
        )
        .expect("unique generated view");

    store
        .remove_root_state(first.id())
        .expect("remove first root");

    let subjects = store.list_work_record_subjects().expect("subjects");
    assert_eq!(subjects.len(), 1);
    assert_eq!(subjects[0].subject_id, shared.subject_id);
    assert_eq!(subjects[0].display_name, "Shared plan");
    assert!(subjects[0].exact_locator_key.contains("project_shared"));
    assert_eq!(
        store
            .work_record_annotation(&shared.subject_id)
            .expect("shared annotation"),
        WorkRecordAnnotation {
            disposition: Disposition::Obsolete,
            favorite: true,
            ..WorkRecordAnnotation::default()
        }
    );
    assert!(
        store
            .find_latest_generated_view(&unique.subject_id, GenerationMode::Summary, "summary-v1",)
            .expect("pruned generated view")
            .is_none()
    );

    store
        .remove_root_state(second.id())
        .expect("remove final root");
    assert!(
        store
            .list_work_record_subjects()
            .expect("empty subjects")
            .is_empty()
    );
}

#[test]
fn annotation_write_racing_root_removal_cannot_resurrect_a_forgotten_subject() {
    let root_dir = tempfile::TempDir::new().expect("root");
    let root = backstage_core::ApprovedRoot::new(root_dir.path(), true).expect("root");
    let plan = record("project_1", "PLAN.md", "Plan");
    let store = Arc::new(SqliteStore::in_memory().expect("store"));
    store.upsert_root(&root).expect("approval");
    store
        .refresh_work_record_subjects(root.id(), std::slice::from_ref(&plan), "seen")
        .expect("route");
    let barrier = Arc::new(Barrier::new(3));
    let writer_store = Arc::clone(&store);
    let writer_barrier = Arc::clone(&barrier);
    let subject_id = plan.subject_id.clone();
    let writer = std::thread::spawn(move || {
        writer_barrier.wait();
        writer_store.save_work_record_annotation(
            &subject_id,
            &WorkRecordAnnotation {
                favorite: true,
                ..WorkRecordAnnotation::default()
            },
            "racing-write",
        )
    });
    let remover_store = Arc::clone(&store);
    let remover_barrier = Arc::clone(&barrier);
    let root_id = root.id().to_owned();
    let remover = std::thread::spawn(move || {
        remover_barrier.wait();
        remover_store.remove_root_state(&root_id)
    });

    barrier.wait();
    let _write_result = writer.join().expect("writer thread");
    remover
        .join()
        .expect("remover thread")
        .expect("root removal");

    assert!(store.list_roots().expect("roots").is_empty());
    assert!(
        store
            .list_work_record_subjects()
            .expect("subjects")
            .is_empty()
    );
}

#[test]
fn exact_locator_changes_create_distinct_subjects_without_annotation_transfer() {
    let before = record("project_1", "PLAN.md", "Plan");
    let moved = record("project_1", "docs/PLAN.md", "Plan");
    let reformatted = {
        let descriptor = AdapterDescriptor::new("planning-pattern-v1", "planning-pattern", 1, 30);
        WorkRecord::new(
            RecordLocator::new("project_1", descriptor.format_id(), "PLAN.md"),
            "Plan",
            WorkRecordRecognition::new(
                RecognitionLevel::Possible,
                &descriptor,
                vec!["pattern".to_owned()],
            ),
            vec![WorkRecordSource::new("PLAN.md", Some(1))],
            vec![],
            vec![],
            vec![Capability::new("source", "Source")],
        )
    };

    assert_ne!(before.subject_id, moved.subject_id);
    assert_ne!(before.subject_id, reformatted.subject_id);
}
