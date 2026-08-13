use backstage_app_lib::discovery::{GitContext, ProjectCandidate, ScanWarning};
use backstage_app_lib::index::{
    IndexPersistence, IndexSession, IndexSnapshot, IndexedBundle, IndexedProject,
};
use backstage_app_lib::storage::SqliteStore;
use backstage_core::{
    DetectorEvidence, EvidenceKind, SourceObservation, SourceSnapshot, classify_project,
    fingerprint_snapshots, parse_openspec_tasks,
};

fn index(generation: u64) -> IndexSnapshot {
    let bundles = classify_project(
        "project_1",
        "Workbench",
        vec![DetectorEvidence::new(
            "openspec/changes/search/tasks.md",
            EvidenceKind::OpenSpecMember,
            "OpenSpec change material",
        )],
    );
    let content = "- [x] Parse\n- [ ] Render\n";
    let observation = SourceObservation {
        byte_len: content.len() as u64,
        modified_unix_nanos: Some(42),
    };
    let source = SourceSnapshot::from_observations(
        "openspec/changes/search/tasks.md",
        content.as_bytes().to_vec(),
        observation,
        observation,
    )
    .expect("snapshot");

    IndexSnapshot {
        root_id: "root_1".to_owned(),
        generation,
        indexed_at: "2026-08-13T12:00:00Z".to_owned(),
        projects: vec![IndexedProject {
            project: ProjectCandidate {
                id: "project_1".to_owned(),
                name: "Workbench".to_owned(),
                root_path: "/tmp/workbench".to_owned(),
                git: Some(GitContext {
                    branch: "main".to_owned(),
                }),
            },
            bundles: vec![IndexedBundle {
                bundle: bundles[0].clone(),
                progress: parse_openspec_tasks(content),
                fingerprint: Some(fingerprint_snapshots(&[source])),
                source_modified_unix_nanos: Some(42),
                warnings: vec!["Parser retained readable source".to_owned()],
            }],
            markdown_documents: vec![backstage_core::MarkdownDocument::new(
                "project_1",
                "Workbench",
                "README.md",
                Some(41),
            )],
        }],
        warnings: vec![ScanWarning {
            code: "partial".to_owned(),
            path: "/tmp/workbench".to_owned(),
            message: "One path was unreadable".to_owned(),
        }],
    }
}

#[test]
fn sqlite_round_trips_parsed_facts_fingerprints_and_warnings() {
    let store = SqliteStore::in_memory().expect("memory store");
    store
        .upsert_root(&backstage_core::ApprovedRoot::new("/tmp", true).expect("root"))
        .expect("store root");
    let mut expected = index(1);
    expected.root_id = backstage_core::ApprovedRoot::new("/tmp", true)
        .expect("root")
        .id()
        .to_owned();

    store.save_index(&expected).expect("save index");

    assert_eq!(
        store.load_index(&expected.root_id).expect("load index"),
        Some(expected)
    );
}

#[test]
fn legacy_index_without_markdown_documents_loads_with_an_empty_manifest() {
    let mut payload = serde_json::to_value(index(1)).expect("serialize index");
    payload["projects"][0]
        .as_object_mut()
        .expect("project object")
        .remove("markdownDocuments");

    let restored: IndexSnapshot = serde_json::from_value(payload).expect("load legacy index");

    assert!(restored.projects[0].markdown_documents.is_empty());
}

#[test]
fn sqlite_replaces_an_index_snapshot_atomically_by_root() {
    let store = SqliteStore::in_memory().expect("memory store");
    let root = backstage_core::ApprovedRoot::new("/tmp", true).expect("root");
    store.upsert_root(&root).expect("store root");
    let mut first = index(1);
    first.root_id = root.id().to_owned();
    let mut second = index(2);
    second.root_id = root.id().to_owned();
    second.warnings.clear();

    store.save_index(&first).expect("save first");
    store.save_index(&second).expect("replace index");

    assert_eq!(
        store.load_index(root.id()).expect("load index"),
        Some(second)
    );
}

struct FailingPersistence;

impl IndexPersistence for FailingPersistence {
    fn save_index(&self, _snapshot: &IndexSnapshot) -> Result<(), String> {
        Err("disk full".to_owned())
    }

    fn load_index(&self, _root_id: &str) -> Result<Option<IndexSnapshot>, String> {
        Ok(None)
    }
}

#[test]
fn cache_write_failure_keeps_the_new_in_memory_index_usable() {
    let mut session = IndexSession::default();
    let expected = index(7);

    session.publish(expected.clone(), &FailingPersistence);

    assert_eq!(session.current(), Some(&expected));
    assert_eq!(session.operational_warnings().len(), 1);
    assert!(session.operational_warnings()[0].contains("disk full"));
}
