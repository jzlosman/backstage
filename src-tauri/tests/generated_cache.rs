mod support;

use backstage_app_lib::catalog::build_index;
use backstage_app_lib::discovery::{CancellationToken, ScanPolicy, discover_projects};
use backstage_app_lib::filesystem::ContainedReader;
use backstage_app_lib::storage::SqliteStore;
use backstage_core::{GeneratedResult, GenerationMode, SourceFingerprint, SubjectId};
use support::FixtureRepo;

fn result(fingerprint: &str, prompt_version: &str) -> GeneratedResult {
    GeneratedResult {
        text: "Generated summary".to_owned(),
        mode: GenerationMode::Summary,
        source_fingerprint: SourceFingerprint::from_trusted(fingerprint),
        included_paths: vec!["proposal.md".to_owned(), "tasks.md".to_owned()],
        generated_at: "2026-08-13T12:00:00Z".to_owned(),
        model: Some("openai-codex/gpt-5.6-sol".to_owned()),
        prompt_version: prompt_version.to_owned(),
    }
}

#[test]
fn generated_cache_round_trips_text_paths_time_model_mode_and_prompt_version() {
    let store = SqliteStore::in_memory().expect("memory store");
    let expected = result("sha256:a", "summary-v1");
    let subject = SubjectId::from_trusted("subject_1");

    store
        .save_generated_view(&subject, &expected)
        .expect("save generated view");

    assert_eq!(
        store
            .find_generated_view(&subject, GenerationMode::Summary, "sha256:a", "summary-v1",)
            .expect("find generated view"),
        Some(expected.clone())
    );
    assert_eq!(
        store
            .find_latest_generated_view(&subject, GenerationMode::Summary, "summary-v1")
            .expect("find latest generated view"),
        Some(expected)
    );
}

#[test]
fn cache_reuse_requires_same_subject_mode_fingerprint_and_prompt_version() {
    let store = SqliteStore::in_memory().expect("memory store");
    let subject = SubjectId::from_trusted("subject_1");
    store
        .save_generated_view(&subject, &result("sha256:a", "summary-v1"))
        .expect("save generated view");

    assert!(
        store
            .find_generated_view(&subject, GenerationMode::Summary, "sha256:b", "summary-v1",)
            .expect("query")
            .is_none()
    );
    assert!(
        store
            .find_generated_view(&subject, GenerationMode::Summary, "sha256:a", "summary-v2",)
            .expect("query")
            .is_none()
    );
    assert!(
        store
            .find_generated_view(
                &SubjectId::from_trusted("subject_2"),
                GenerationMode::Summary,
                "sha256:a",
                "summary-v1",
            )
            .expect("query")
            .is_none()
    );
}

#[test]
fn reachable_legacy_bundle_cache_migrates_to_subject_and_unmappable_rows_are_deleted() {
    let fixture = FixtureRepo::open_spec();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());
    let index = build_index(
        &reader,
        discovered.projects,
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
    );
    let bundle = index.projects[0]
        .bundles
        .iter()
        .find(|bundle| bundle.bundle.name == "ship-search")
        .expect("legacy OpenSpec bundle");
    let subject = index.projects[0]
        .records
        .iter()
        .find(|record| record.locator.format_id == "openspec")
        .expect("neutral OpenSpec owner")
        .subject_id
        .clone();
    let expected = result("sha256:legacy", "summary-v1");
    let app_data = tempfile::TempDir::new().expect("app data");
    let database = app_data.path().join("backstage.sqlite3");
    let connection = rusqlite::Connection::open(&database).expect("legacy database");
    connection
        .execute_batch(
            "CREATE TABLE approved_roots (id TEXT PRIMARY KEY, path TEXT NOT NULL UNIQUE);
             CREATE TABLE index_snapshots (
               root_id TEXT PRIMARY KEY,
               generation INTEGER NOT NULL,
               payload TEXT NOT NULL,
               updated_at TEXT NOT NULL
             );
             CREATE TABLE generated_views (
               cache_key TEXT PRIMARY KEY,
               bundle_id TEXT NOT NULL,
               mode TEXT NOT NULL,
               source_fingerprint TEXT NOT NULL,
               prompt_version TEXT NOT NULL,
               included_paths TEXT NOT NULL,
               generated_text TEXT NOT NULL,
               generated_at TEXT NOT NULL,
               model TEXT
             );",
        )
        .expect("legacy schema");
    connection
        .execute(
            "INSERT INTO approved_roots (id, path) VALUES (?1, ?2)",
            rusqlite::params![index.root_id, fixture.path().to_string_lossy()],
        )
        .expect("legacy root");
    connection
        .execute(
            "INSERT INTO index_snapshots (root_id, generation, payload, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                index.root_id,
                index.generation,
                serde_json::to_string(&index).expect("index payload"),
                index.indexed_at
            ],
        )
        .expect("legacy index");
    for (cache_key, bundle_id) in [
        ("legacy-mappable", bundle.bundle.id.as_str()),
        ("legacy-unmappable", "bundle_missing"),
    ] {
        connection
            .execute(
                "INSERT INTO generated_views (
                   cache_key, bundle_id, mode, source_fingerprint, prompt_version,
                   included_paths, generated_text, generated_at, model
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    cache_key,
                    bundle_id,
                    "summary",
                    expected.source_fingerprint.as_str(),
                    expected.prompt_version,
                    serde_json::to_string(&expected.included_paths).expect("paths"),
                    expected.text,
                    expected.generated_at,
                    expected.model,
                ],
            )
            .expect("legacy generated row");
    }
    drop(connection);

    let store = SqliteStore::open(&database).expect("migrate store");

    assert_eq!(
        store
            .find_latest_generated_view(&subject, GenerationMode::Summary, "summary-v1")
            .expect("migrated result"),
        Some(expected)
    );
    drop(store);
    let connection = rusqlite::Connection::open(&database).expect("inspect migration");
    let rows = connection
        .query_row("SELECT COUNT(*) FROM generated_views", [], |row| {
            row.get::<_, usize>(0)
        })
        .expect("generated row count");
    let migrated_subject = connection
        .query_row("SELECT subject_id FROM generated_views", [], |row| {
            row.get::<_, String>(0)
        })
        .expect("subject owner");
    assert_eq!(rows, 1);
    assert_eq!(migrated_subject, subject.as_str());
}
