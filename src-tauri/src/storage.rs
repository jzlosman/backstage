use std::fs;
use std::path::Path;

use backstage_core::{ApprovedRoot, GeneratedResult, GenerationMode};
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, params};

use crate::index::{IndexPersistence, IndexSnapshot};

pub struct SqliteStore {
    connection: Mutex<Connection>,
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
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn in_memory() -> Result<Self, StoreError> {
        let store = Self {
            connection: Mutex::new(Connection::open_in_memory()?),
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
        self.connection
            .lock()
            .execute("DELETE FROM approved_roots WHERE id = ?1", [root_id])?;
        Ok(())
    }

    pub fn save_index(&self, snapshot: &IndexSnapshot) -> Result<(), StoreError> {
        let payload = serde_json::to_string(snapshot)?;
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
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
        self.connection.lock().execute_batch(
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
             );",
        )?;
        Ok(())
    }
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
}
