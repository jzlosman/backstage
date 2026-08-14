use backstage_app_lib::storage::SqliteStore;
use backstage_core::{PlanningPatternProvenance, canonical_planning_patterns};
use rusqlite::Connection;
use tempfile::TempDir;

#[test]
fn legacy_database_gains_pattern_configuration_without_losing_existing_tables() {
    let app_data = TempDir::new().expect("app data");
    let database = app_data.path().join("legacy.sqlite3");
    let legacy = Connection::open(&database).expect("legacy database");
    legacy
        .execute_batch(
            "CREATE TABLE schema_version (version INTEGER NOT NULL);
             INSERT INTO schema_version(version) VALUES (1);
             CREATE TABLE approved_roots (id TEXT PRIMARY KEY, path TEXT NOT NULL UNIQUE);
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
             );
             CREATE TABLE preferences (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        )
        .expect("legacy schema");
    drop(legacy);

    let migrated = SqliteStore::open(&database).expect("migrate legacy database");

    assert_eq!(
        migrated
            .planning_configuration()
            .expect("pattern configuration")
            .patterns,
        canonical_planning_patterns()
    );
    assert!(
        migrated
            .list_roots()
            .expect("legacy roots remain readable")
            .is_empty()
    );
}

#[test]
fn migration_seeds_canonical_defaults_once_and_deleted_empty_state_survives_restart() {
    let app_data = TempDir::new().expect("app data");
    let database = app_data.path().join("index.sqlite3");
    let expected = canonical_planning_patterns();

    {
        let store = SqliteStore::open(&database).expect("migrate store");
        let initial = store
            .planning_configuration()
            .expect("initial configuration");
        assert_eq!(initial.revision, 0);
        assert_eq!(initial.patterns, expected);
        for pattern in initial.patterns {
            store
                .remove_planning_pattern(pattern.id())
                .expect("remove seeded default");
        }
        let empty = store.planning_configuration().expect("empty configuration");
        assert!(empty.patterns.is_empty());
        assert_eq!(empty.revision, 3);
    }

    let reopened = SqliteStore::open(&database).expect("reopen store");
    let persisted = reopened
        .planning_configuration()
        .expect("persisted empty configuration");
    assert!(persisted.patterns.is_empty());
    assert_eq!(persisted.revision, 3);
}

#[test]
fn restore_defaults_adds_only_missing_defaults_and_preserves_custom_patterns() {
    let store = SqliteStore::in_memory().expect("store");
    let defaults = canonical_planning_patterns();
    store
        .remove_planning_pattern(defaults[0].id())
        .expect("remove one default");
    let custom = store
        .add_planning_pattern("^docs/plans/.*\\.md$")
        .expect("add custom")
        .patterns
        .into_iter()
        .find(|pattern| pattern.provenance() == PlanningPatternProvenance::Custom)
        .expect("custom pattern");
    let before_restore = store.planning_configuration().expect("configuration");

    let restored = store
        .restore_default_planning_patterns()
        .expect("restore defaults");

    assert_eq!(restored.revision, before_restore.revision + 1);
    assert_eq!(restored.patterns.len(), 4);
    assert!(restored.patterns.iter().any(|pattern| pattern == &custom));
    for default in defaults {
        assert_eq!(
            restored
                .patterns
                .iter()
                .filter(|pattern| pattern.expression() == default.expression())
                .count(),
            1
        );
    }

    let unchanged = store
        .restore_default_planning_patterns()
        .expect("idempotent restore");
    assert_eq!(unchanged.revision, restored.revision);
    assert_eq!(unchanged.patterns, restored.patterns);
}

#[test]
fn invalid_or_over_limit_mutations_leave_configuration_unchanged() {
    let store = SqliteStore::in_memory().expect("store");
    let initial = store.planning_configuration().expect("initial");

    assert!(store.add_planning_pattern("(").is_err());
    assert_eq!(
        store.planning_configuration().expect("after invalid"),
        initial
    );

    for ordinal in initial.patterns.len()..64 {
        store
            .add_planning_pattern(&format!("^custom-{ordinal}\\.md$"))
            .expect("fill pattern capacity");
    }
    let full = store.planning_configuration().expect("full");
    assert_eq!(full.patterns.len(), 64);
    assert!(store.add_planning_pattern("^overflow\\.md$").is_err());
    assert_eq!(
        store.planning_configuration().expect("after overflow"),
        full
    );
}

#[test]
fn custom_patterns_and_revision_survive_restart() {
    let app_data = TempDir::new().expect("app data");
    let database = app_data.path().join("index.sqlite3");
    let expected = {
        let store = SqliteStore::open(&database).expect("store");
        store
            .add_planning_pattern("^architecture/.*\\.md$")
            .expect("add custom")
    };

    let reopened = SqliteStore::open(&database).expect("reopen");
    assert_eq!(
        reopened.planning_configuration().expect("configuration"),
        expected
    );
}
