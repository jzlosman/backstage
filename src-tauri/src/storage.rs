use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use backstage_core::{
    ApprovedRoot, GeneratedResult, GenerationMode, PlanningPattern, PlanningPatternConfiguration,
    PlanningPatternError, PlanningPatternProvenance, canonical_planning_patterns,
    validate_planning_pattern_count,
};
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, params};

use crate::index::{IndexPersistence, IndexSnapshot};

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RootRemovalInventory {
    pub roots: Vec<ApprovedRoot>,
    pub indexes: Vec<IndexSnapshot>,
}

pub struct SqliteStore {
    connection: Mutex<Connection>,
    #[cfg(test)]
    fail_next_index_save: std::sync::atomic::AtomicBool,
}

impl SqliteStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(path)?;
        let store = Self {
            connection: Mutex::new(connection),
            #[cfg(test)]
            fail_next_index_save: std::sync::atomic::AtomicBool::new(false),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn in_memory() -> Result<Self, StoreError> {
        let store = Self {
            connection: Mutex::new(Connection::open_in_memory()?),
            #[cfg(test)]
            fail_next_index_save: std::sync::atomic::AtomicBool::new(false),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn upsert_root(&self, root: &ApprovedRoot) -> Result<(), StoreError> {
        self.connection.lock().execute(
            "INSERT INTO approved_roots (id, path) VALUES (?1, ?2)
             ON CONFLICT(id) DO UPDATE SET path = excluded.path",
            params![root.id(), root.path()],
        )?;
        Ok(())
    }

    pub fn list_roots(&self) -> Result<Vec<ApprovedRoot>, StoreError> {
        let connection = self.connection.lock();
        let mut statement = connection.prepare("SELECT path FROM approved_roots ORDER BY path")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.map(|path| {
            ApprovedRoot::new(path?, true).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(StoreError::from)
    }

    pub fn remove_root(&self, root_id: &str) -> Result<(), StoreError> {
        self.remove_root_state(root_id).map(|_| ())
    }

    pub fn remove_root_state(&self, root_id: &str) -> Result<RootRemovalInventory, StoreError> {
        self.remove_root_state_with_retained_indexes(root_id, &[])
    }

    pub fn remove_root_state_with_retained_indexes(
        &self,
        root_id: &str,
        retained_indexes: &[IndexSnapshot],
    ) -> Result<RootRemovalInventory, StoreError> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let exists = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM approved_roots WHERE id = ?1)",
            [root_id],
            |row| row.get::<_, bool>(0),
        )?;
        if !exists {
            return Err(StoreError::RootNotFound(root_id.to_owned()));
        }

        transaction.execute("DELETE FROM approved_roots WHERE id = ?1", [root_id])?;
        let roots = load_roots(&transaction)?;
        let retained_root_ids = roots
            .iter()
            .map(|root| root.id().to_owned())
            .collect::<BTreeSet<_>>();
        let mut indexes = load_all_indexes(&transaction)?
            .into_iter()
            .map(|index| (index.root_id.clone(), index))
            .collect::<BTreeMap<_, _>>();
        for index in retained_indexes {
            if retained_root_ids.contains(&index.root_id) {
                indexes.insert(index.root_id.clone(), index.clone());
            }
        }
        let indexes = indexes.into_values().collect::<Vec<_>>();
        let reachable_bundle_ids = indexes
            .iter()
            .flat_map(|index| &index.projects)
            .flat_map(|project| &project.bundles)
            .map(|bundle| bundle.bundle.id.clone())
            .collect::<BTreeSet<_>>();
        let cached_bundle_ids = {
            let mut statement =
                transaction.prepare("SELECT DISTINCT bundle_id FROM generated_views")?;
            statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?
        };
        for bundle_id in cached_bundle_ids {
            if !reachable_bundle_ids.contains(&bundle_id) {
                transaction.execute(
                    "DELETE FROM generated_views WHERE bundle_id = ?1",
                    [bundle_id],
                )?;
            }
        }
        transaction.commit()?;
        Ok(RootRemovalInventory { roots, indexes })
    }

    pub fn planning_configuration(&self) -> Result<PlanningPatternConfiguration, StoreError> {
        load_planning_configuration(&self.connection.lock())
    }

    pub fn add_planning_pattern(
        &self,
        expression: &str,
    ) -> Result<PlanningPatternConfiguration, StoreError> {
        let expression = expression.trim();
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let count = transaction.query_row("SELECT COUNT(*) FROM planning_patterns", [], |row| {
            row.get::<_, usize>(0)
        })?;
        validate_planning_pattern_count(count + 1)?;
        let ordinal = next_pattern_ordinal(&transaction)?;
        let pattern = PlanningPattern::custom(expression, ordinal)?;
        let duplicate = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM planning_patterns WHERE expression = ?1)",
            [pattern.expression()],
            |row| row.get::<_, bool>(0),
        )?;
        if duplicate {
            return Err(StoreError::PlanningPatternAlreadyExists(
                pattern.expression().to_owned(),
            ));
        }
        insert_planning_pattern(&transaction, &pattern)?;
        increment_configuration_revision(&transaction)?;
        let configuration = load_planning_configuration(&transaction)?;
        transaction.commit()?;
        Ok(configuration)
    }

    pub fn remove_planning_pattern(
        &self,
        pattern_id: &str,
    ) -> Result<PlanningPatternConfiguration, StoreError> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let removed =
            transaction.execute("DELETE FROM planning_patterns WHERE id = ?1", [pattern_id])?;
        if removed == 0 {
            return Err(StoreError::PlanningPatternNotFound(pattern_id.to_owned()));
        }
        increment_configuration_revision(&transaction)?;
        let configuration = load_planning_configuration(&transaction)?;
        transaction.commit()?;
        Ok(configuration)
    }

    pub fn restore_default_planning_patterns(
        &self,
    ) -> Result<PlanningPatternConfiguration, StoreError> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let existing = {
            let mut statement = transaction.prepare("SELECT expression FROM planning_patterns")?;
            statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<BTreeSet<_>, _>>()?
        };
        let mut ordinal = next_pattern_ordinal(&transaction)?;
        let mut inserted = 0;
        for default in canonical_planning_patterns() {
            if existing.contains(default.expression()) {
                continue;
            }
            let pattern = PlanningPattern::persisted(
                default.expression(),
                ordinal,
                PlanningPatternProvenance::Default,
            )?;
            insert_planning_pattern(&transaction, &pattern)?;
            ordinal += 1;
            inserted += 1;
        }
        if inserted > 0 {
            increment_configuration_revision(&transaction)?;
        }
        let configuration = load_planning_configuration(&transaction)?;
        transaction.commit()?;
        Ok(configuration)
    }

    #[cfg(test)]
    pub fn fail_next_index_save(&self) {
        self.fail_next_index_save
            .store(true, std::sync::atomic::Ordering::Release);
    }

    pub fn save_index(&self, snapshot: &IndexSnapshot) -> Result<(), StoreError> {
        #[cfg(test)]
        if self
            .fail_next_index_save
            .swap(false, std::sync::atomic::Ordering::AcqRel)
        {
            return Err(StoreError::Io(std::io::Error::other(
                "injected index cache save failure",
            )));
        }
        let payload = serde_json::to_string(snapshot)?;
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        let stored = transaction
            .query_row(
                "SELECT payload FROM index_snapshots WHERE root_id = ?1",
                [&snapshot.root_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .map(|payload| serde_json::from_str::<IndexSnapshot>(&payload))
            .transpose()?;
        if stored.as_ref().is_some_and(|stored| {
            (stored.configuration_revision, stored.generation)
                > (snapshot.configuration_revision, snapshot.generation)
        }) {
            transaction.commit()?;
            return Ok(());
        }
        transaction.execute(
            "INSERT INTO index_snapshots (root_id, generation, payload, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(root_id) DO UPDATE SET
               generation = excluded.generation,
               payload = excluded.payload,
               updated_at = excluded.updated_at",
            params![
                snapshot.root_id,
                snapshot.generation,
                payload,
                snapshot.indexed_at
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn load_index(&self, root_id: &str) -> Result<Option<IndexSnapshot>, StoreError> {
        let connection = self.connection.lock();
        let payload = connection
            .query_row(
                "SELECT payload FROM index_snapshots WHERE root_id = ?1",
                [root_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(payload) = payload else {
            return Ok(None);
        };
        match serde_json::from_str(&payload) {
            Ok(snapshot) => Ok(Some(snapshot)),
            Err(_) => {
                connection.execute("DELETE FROM index_snapshots WHERE root_id = ?1", [root_id])?;
                Ok(None)
            }
        }
    }

    pub fn save_generated_view(
        &self,
        bundle_id: &str,
        result: &GeneratedResult,
    ) -> Result<(), StoreError> {
        let mode = generation_mode(result.mode);
        let included_paths = serde_json::to_string(&result.included_paths)?;
        let cache_key = generated_cache_key(
            bundle_id,
            mode,
            result.source_fingerprint.as_str(),
            &result.prompt_version,
        );
        self.connection.lock().execute(
            "INSERT INTO generated_views (
               cache_key, bundle_id, mode, source_fingerprint, prompt_version,
               included_paths, generated_text, generated_at, model
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(cache_key) DO UPDATE SET
               included_paths = excluded.included_paths,
               generated_text = excluded.generated_text,
               generated_at = excluded.generated_at,
               model = excluded.model",
            params![
                cache_key,
                bundle_id,
                mode,
                result.source_fingerprint.as_str(),
                result.prompt_version,
                included_paths,
                result.text,
                result.generated_at,
                result.model,
            ],
        )?;
        Ok(())
    }

    pub fn find_generated_view(
        &self,
        bundle_id: &str,
        mode: GenerationMode,
        source_fingerprint: &str,
        prompt_version: &str,
    ) -> Result<Option<GeneratedResult>, StoreError> {
        let mode_name = generation_mode(mode);
        let cache_key =
            generated_cache_key(bundle_id, mode_name, source_fingerprint, prompt_version);
        let connection = self.connection.lock();
        let row = connection
            .query_row(
                "SELECT source_fingerprint, included_paths, generated_text, generated_at, model
                 FROM generated_views WHERE cache_key = ?1",
                [cache_key],
                generated_view_row,
            )
            .optional()?;
        deserialize_generated_view(row, mode, prompt_version)
    }

    pub fn find_latest_generated_view(
        &self,
        bundle_id: &str,
        mode: GenerationMode,
        prompt_version: &str,
    ) -> Result<Option<GeneratedResult>, StoreError> {
        let connection = self.connection.lock();
        let row = connection
            .query_row(
                "SELECT source_fingerprint, included_paths, generated_text, generated_at, model
                 FROM generated_views
                 WHERE bundle_id = ?1 AND mode = ?2 AND prompt_version = ?3
                 ORDER BY generated_at DESC LIMIT 1",
                params![bundle_id, generation_mode(mode), prompt_version],
                generated_view_row,
            )
            .optional()?;
        deserialize_generated_view(row, mode, prompt_version)
    }

    fn migrate(&self) -> Result<(), StoreError> {
        let mut connection = self.connection.lock();
        connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS schema_version (
               version INTEGER NOT NULL
             );
             INSERT INTO schema_version(version)
               SELECT 1 WHERE NOT EXISTS (SELECT 1 FROM schema_version);
             CREATE TABLE IF NOT EXISTS approved_roots (
               id TEXT PRIMARY KEY,
               path TEXT NOT NULL UNIQUE
             );
             CREATE TABLE IF NOT EXISTS index_snapshots (
               root_id TEXT PRIMARY KEY,
               generation INTEGER NOT NULL,
               payload TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               FOREIGN KEY(root_id) REFERENCES approved_roots(id) ON DELETE CASCADE
             );
             CREATE TABLE IF NOT EXISTS generated_views (
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
             CREATE TABLE IF NOT EXISTS preferences (
               key TEXT PRIMARY KEY,
               value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS planning_patterns (
               id TEXT PRIMARY KEY,
               expression TEXT NOT NULL UNIQUE,
               ordinal INTEGER NOT NULL,
               provenance TEXT NOT NULL CHECK(provenance IN ('default', 'custom'))
             );
             CREATE TABLE IF NOT EXISTS configuration_state (
               singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
               revision INTEGER NOT NULL
             );
             INSERT OR IGNORE INTO configuration_state(singleton, revision) VALUES (1, 0);
             CREATE TABLE IF NOT EXISTS migration_markers (
               name TEXT PRIMARY KEY
             );",
        )?;

        let transaction = connection.transaction()?;
        let seeded = transaction.query_row(
            "SELECT EXISTS(
               SELECT 1 FROM migration_markers WHERE name = 'planning-pattern-defaults-v1'
             )",
            [],
            |row| row.get::<_, bool>(0),
        )?;
        if !seeded {
            for pattern in canonical_planning_patterns() {
                insert_planning_pattern(&transaction, &pattern)?;
            }
            transaction.execute(
                "INSERT INTO migration_markers(name) VALUES ('planning-pattern-defaults-v1')",
                [],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }
}

fn load_roots(connection: &Connection) -> Result<Vec<ApprovedRoot>, StoreError> {
    let mut statement = connection.prepare("SELECT path FROM approved_roots ORDER BY path")?;
    let paths = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    paths
        .into_iter()
        .map(|path| {
            ApprovedRoot::new(path, true)
                .map_err(|error| StoreError::InvalidStoredConfiguration(error.to_string()))
        })
        .collect()
}

fn load_all_indexes(connection: &Connection) -> Result<Vec<IndexSnapshot>, StoreError> {
    let mut statement =
        connection.prepare("SELECT payload FROM index_snapshots ORDER BY root_id")?;
    let payloads = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    payloads
        .into_iter()
        .map(|payload| serde_json::from_str(&payload).map_err(StoreError::from))
        .collect()
}

fn load_planning_configuration(
    connection: &Connection,
) -> Result<PlanningPatternConfiguration, StoreError> {
    let revision = connection.query_row(
        "SELECT revision FROM configuration_state WHERE singleton = 1",
        [],
        |row| row.get::<_, u64>(0),
    )?;
    let mut statement = connection.prepare(
        "SELECT id, expression, ordinal, provenance
         FROM planning_patterns ORDER BY ordinal, id",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, u32>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    validate_planning_pattern_count(rows.len())?;
    let patterns = rows
        .into_iter()
        .map(|(stored_id, expression, ordinal, provenance)| {
            let provenance = PlanningPatternProvenance::parse(&provenance)?;
            let pattern = PlanningPattern::persisted(expression, ordinal, provenance)?;
            if pattern.id() != stored_id {
                return Err(StoreError::InvalidStoredConfiguration(format!(
                    "planning pattern ID {stored_id} does not match its expression"
                )));
            }
            Ok(pattern)
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(PlanningPatternConfiguration { revision, patterns })
}

fn next_pattern_ordinal(connection: &Connection) -> Result<u32, StoreError> {
    connection
        .query_row(
            "SELECT COALESCE(MAX(ordinal) + 1, 0) FROM planning_patterns",
            [],
            |row| row.get(0),
        )
        .map_err(StoreError::from)
}

fn insert_planning_pattern(
    connection: &Connection,
    pattern: &PlanningPattern,
) -> Result<(), StoreError> {
    connection.execute(
        "INSERT INTO planning_patterns (id, expression, ordinal, provenance)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            pattern.id(),
            pattern.expression(),
            pattern.ordinal(),
            pattern.provenance().as_str(),
        ],
    )?;
    Ok(())
}

fn increment_configuration_revision(connection: &Connection) -> Result<(), StoreError> {
    connection.execute(
        "UPDATE configuration_state SET revision = revision + 1 WHERE singleton = 1",
        [],
    )?;
    Ok(())
}

type GeneratedViewRow = (String, String, String, String, Option<String>);

fn generated_view_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<GeneratedViewRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
    ))
}

fn deserialize_generated_view(
    row: Option<GeneratedViewRow>,
    mode: GenerationMode,
    prompt_version: &str,
) -> Result<Option<GeneratedResult>, StoreError> {
    row.map(
        |(source_fingerprint, included_paths, text, generated_at, model)| {
            Ok(GeneratedResult {
                text,
                mode,
                source_fingerprint: backstage_core::SourceFingerprint::from_trusted(
                    source_fingerprint,
                ),
                included_paths: serde_json::from_str(&included_paths)?,
                generated_at,
                model,
                prompt_version: prompt_version.to_owned(),
            })
        },
    )
    .transpose()
}

fn generation_mode(mode: GenerationMode) -> &'static str {
    match mode {
        GenerationMode::Summary => "summary",
    }
}

fn generated_cache_key(
    bundle_id: &str,
    mode: &str,
    fingerprint: &str,
    prompt_version: &str,
) -> String {
    use sha2::{Digest, Sha256};
    let digest =
        Sha256::digest(format!("{bundle_id}\0{mode}\0{fingerprint}\0{prompt_version}").as_bytes());
    format!("generated_{digest:x}")
}

impl IndexPersistence for SqliteStore {
    fn save_index(&self, snapshot: &IndexSnapshot) -> Result<(), String> {
        SqliteStore::save_index(self, snapshot).map_err(|error| error.to_string())
    }

    fn load_index(&self, root_id: &str) -> Result<Option<IndexSnapshot>, String> {
        SqliteStore::load_index(self, root_id).map_err(|error| error.to_string())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("storage I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("SQLite operation failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("stored index JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    PlanningPattern(#[from] PlanningPatternError),
    #[error("approved root was not found: {0}")]
    RootNotFound(String),
    #[error("planning pattern already exists: {0}")]
    PlanningPatternAlreadyExists(String),
    #[error("planning pattern was not found: {0}")]
    PlanningPatternNotFound(String),
    #[error("stored app configuration is invalid: {0}")]
    InvalidStoredConfiguration(String),
}
