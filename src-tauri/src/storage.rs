use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use backstage_core::{
    ApprovedRoot, Decision, Disposition, GeneratedResult, GenerationMode, PlanningPattern,
    PlanningPatternConfiguration, PlanningPatternError, PlanningPatternProvenance, Priority,
    SubjectId, WorkRecord, WorkRecordAnnotation, canonical_planning_patterns,
    validate_planning_pattern_count,
};
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, params};

use crate::index::{IndexPersistence, IndexSnapshot, migrate_legacy_snapshot};

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RootRemovalInventory {
    pub roots: Vec<ApprovedRoot>,
    pub indexes: Vec<IndexSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredWorkRecordSubject {
    pub subject_id: SubjectId,
    pub display_name: String,
    pub exact_locator_key: String,
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
        let known_subject_ids = {
            let mut statement =
                transaction.prepare("SELECT subject_id FROM work_record_subjects")?;
            statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<BTreeSet<_>, _>>()?
        };

        transaction.execute("DELETE FROM approved_roots WHERE id = ?1", [root_id])?;
        transaction.execute(
            "UPDATE work_record_annotations
             SET disposition = 'obsolete', superseded_by_subject_id = NULL
             WHERE superseded_by_subject_id IN (
               SELECT subject_id FROM work_record_subjects
               WHERE NOT EXISTS (
                 SELECT 1 FROM work_record_subject_roots
                 WHERE work_record_subject_roots.subject_id = work_record_subjects.subject_id
               )
             )",
            [],
        )?;
        transaction.execute(
            "DELETE FROM work_record_subjects
             WHERE NOT EXISTS (
               SELECT 1 FROM work_record_subject_roots
               WHERE work_record_subject_roots.subject_id = work_record_subjects.subject_id
             )",
            [],
        )?;
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
        let mut reachable_subject_ids = {
            let mut statement =
                transaction.prepare("SELECT DISTINCT subject_id FROM work_record_subject_roots")?;
            statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<BTreeSet<_>, _>>()?
        };
        reachable_subject_ids.extend(
            indexes
                .iter()
                .flat_map(|index| &index.projects)
                .flat_map(|project| &project.records)
                .map(|record| record.subject_id.as_str().to_owned())
                .filter(|subject_id| !known_subject_ids.contains(subject_id)),
        );
        let cached_subject_ids = {
            let mut statement =
                transaction.prepare("SELECT DISTINCT subject_id FROM generated_views")?;
            statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?
        };
        for subject_id in cached_subject_ids {
            if !reachable_subject_ids.contains(&subject_id) {
                transaction.execute(
                    "DELETE FROM generated_views WHERE subject_id = ?1",
                    [subject_id],
                )?;
            }
        }
        transaction.commit()?;
        Ok(RootRemovalInventory { roots, indexes })
    }

    pub fn refresh_work_record_subjects(
        &self,
        root_id: &str,
        records: &[WorkRecord],
        seen_at: &str,
    ) -> Result<(), StoreError> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        for record in records {
            transaction.execute(
                "INSERT INTO work_record_subjects (
                   subject_id, project_id, format_id, adapter_record_key,
                   last_known_name, last_seen_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(subject_id) DO UPDATE SET
                   project_id = excluded.project_id,
                   format_id = excluded.format_id,
                   adapter_record_key = excluded.adapter_record_key,
                   last_known_name = excluded.last_known_name,
                   last_seen_at = excluded.last_seen_at",
                params![
                    record.subject_id.as_str(),
                    record.locator.project_id,
                    record.locator.format_id,
                    record.locator.adapter_record_key,
                    record.display_name,
                    seen_at,
                ],
            )?;
            transaction.execute(
                "INSERT OR IGNORE INTO work_record_subject_roots (subject_id, root_id)
                 VALUES (?1, ?2)",
                params![record.subject_id.as_str(), root_id],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn list_work_record_subjects(&self) -> Result<Vec<StoredWorkRecordSubject>, StoreError> {
        let connection = self.connection.lock();
        let mut statement = connection
            .prepare(
                "SELECT subject_id, last_known_name,
                        format_id || ':' || project_id || ':' || adapter_record_key
                 FROM work_record_subjects ORDER BY last_known_name, subject_id",
            )
            .map_err(StoreError::Sqlite)?;
        let rows = statement
            .query_map([], |row| {
                let raw_id: String = row.get(0)?;
                Ok(StoredWorkRecordSubject {
                    subject_id: SubjectId::from_trusted(raw_id),
                    display_name: row.get(1)?,
                    exact_locator_key: row.get(2)?,
                })
            })
            .map_err(StoreError::Sqlite)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::Sqlite)
    }

    pub fn work_record_annotation(
        &self,
        subject_id: &SubjectId,
    ) -> Result<WorkRecordAnnotation, StoreError> {
        let row = self
            .connection
            .lock()
            .query_row(
                "SELECT decision, disposition, favorite, todo, priority,
                        superseded_by_subject_id
                 FROM work_record_annotations WHERE subject_id = ?1",
                [subject_id.as_str()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, bool>(2)?,
                        row.get::<_, bool>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                },
            )
            .optional()?;
        let Some((decision, disposition, favorite, todo, priority, replacement)) = row else {
            return Ok(WorkRecordAnnotation::default());
        };
        Ok(WorkRecordAnnotation {
            decision: parse_decision(&decision)?,
            disposition: parse_disposition(&disposition, replacement)?,
            favorite,
            todo,
            priority: priority.as_deref().map(parse_priority).transpose()?,
        })
    }

    pub fn save_work_record_annotation(
        &self,
        subject_id: &SubjectId,
        annotation: &WorkRecordAnnotation,
        updated_at: &str,
    ) -> Result<(), StoreError> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;
        if annotation == &WorkRecordAnnotation::default() {
            transaction.execute(
                "DELETE FROM work_record_annotations WHERE subject_id = ?1",
                [subject_id.as_str()],
            )?;
            transaction.commit()?;
            return Ok(());
        }
        let (disposition, replacement) = match &annotation.disposition {
            Disposition::Applicable => ("applicable", None),
            Disposition::Obsolete => ("obsolete", None),
            Disposition::Superseded { replacement } => ("superseded", Some(replacement.as_str())),
        };
        transaction.execute(
            "INSERT INTO work_record_annotations (
               subject_id, decision, disposition, favorite, todo, priority,
               superseded_by_subject_id, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(subject_id) DO UPDATE SET
               decision = excluded.decision,
               disposition = excluded.disposition,
               favorite = excluded.favorite,
               todo = excluded.todo,
               priority = excluded.priority,
               superseded_by_subject_id = excluded.superseded_by_subject_id,
               updated_at = excluded.updated_at",
            params![
                subject_id.as_str(),
                decision_value(annotation.decision),
                disposition,
                annotation.favorite,
                annotation.todo,
                annotation.priority.map(priority_value),
                replacement,
                updated_at,
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn supersession_edges(&self) -> Result<Vec<(SubjectId, SubjectId)>, StoreError> {
        let connection = self.connection.lock();
        let mut statement = connection.prepare(
            "SELECT subject_id, superseded_by_subject_id
             FROM work_record_annotations
             WHERE disposition = 'superseded' AND superseded_by_subject_id IS NOT NULL
             ORDER BY subject_id",
        )?;
        statement
            .query_map([], |row| {
                Ok((
                    SubjectId::from_trusted(row.get::<_, String>(0)?),
                    SubjectId::from_trusted(row.get::<_, String>(1)?),
                ))
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
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
            .and_then(|payload| serde_json::from_str::<IndexSnapshot>(&payload).ok());
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
        match serde_json::from_str::<IndexSnapshot>(&payload) {
            Ok(mut snapshot) => {
                let prior_version = snapshot.schema_version;
                migrate_legacy_snapshot(&mut snapshot);
                if snapshot.schema_version != prior_version {
                    connection.execute(
                        "UPDATE index_snapshots SET payload = ?1 WHERE root_id = ?2",
                        params![serde_json::to_string(&snapshot)?, root_id],
                    )?;
                }
                Ok(Some(snapshot))
            }
            Err(_) => {
                connection.execute("DELETE FROM index_snapshots WHERE root_id = ?1", [root_id])?;
                Ok(None)
            }
        }
    }

    pub fn save_generated_view(
        &self,
        subject_id: &SubjectId,
        result: &GeneratedResult,
    ) -> Result<(), StoreError> {
        let mode = generation_mode(result.mode);
        let included_paths = serde_json::to_string(&result.included_paths)?;
        let cache_key = generated_cache_key(
            subject_id,
            mode,
            result.source_fingerprint.as_str(),
            &result.prompt_version,
        );
        self.connection.lock().execute(
            "INSERT INTO generated_views (
               cache_key, subject_id, mode, source_fingerprint, prompt_version,
               included_paths, generated_text, generated_at, model
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(cache_key) DO UPDATE SET
               included_paths = excluded.included_paths,
               generated_text = excluded.generated_text,
               generated_at = excluded.generated_at,
               model = excluded.model",
            params![
                cache_key,
                subject_id.as_str(),
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
        subject_id: &SubjectId,
        mode: GenerationMode,
        source_fingerprint: &str,
        prompt_version: &str,
    ) -> Result<Option<GeneratedResult>, StoreError> {
        let mode_name = generation_mode(mode);
        let cache_key =
            generated_cache_key(subject_id, mode_name, source_fingerprint, prompt_version);
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
        subject_id: &SubjectId,
        mode: GenerationMode,
        prompt_version: &str,
    ) -> Result<Option<GeneratedResult>, StoreError> {
        let connection = self.connection.lock();
        let row = connection
            .query_row(
                "SELECT source_fingerprint, included_paths, generated_text, generated_at, model
                 FROM generated_views
                 WHERE subject_id = ?1 AND mode = ?2 AND prompt_version = ?3
                 ORDER BY generated_at DESC LIMIT 1",
                params![subject_id.as_str(), generation_mode(mode), prompt_version],
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
               subject_id TEXT NOT NULL,
               mode TEXT NOT NULL,
               source_fingerprint TEXT NOT NULL,
               prompt_version TEXT NOT NULL,
               included_paths TEXT NOT NULL,
               generated_text TEXT NOT NULL,
               generated_at TEXT NOT NULL,
               model TEXT
             );
             CREATE TABLE IF NOT EXISTS work_record_subjects (
               subject_id TEXT PRIMARY KEY,
               project_id TEXT NOT NULL,
               format_id TEXT NOT NULL,
               adapter_record_key TEXT NOT NULL,
               last_known_name TEXT NOT NULL,
               last_seen_at TEXT NOT NULL,
               UNIQUE(project_id, format_id, adapter_record_key)
             );
             CREATE TABLE IF NOT EXISTS work_record_subject_roots (
               subject_id TEXT NOT NULL,
               root_id TEXT NOT NULL,
               PRIMARY KEY(subject_id, root_id),
               FOREIGN KEY(subject_id) REFERENCES work_record_subjects(subject_id) ON DELETE CASCADE,
               FOREIGN KEY(root_id) REFERENCES approved_roots(id) ON DELETE CASCADE
             );
             CREATE TABLE IF NOT EXISTS work_record_annotations (
               subject_id TEXT PRIMARY KEY,
               decision TEXT NOT NULL CHECK(decision IN ('undecided', 'approved', 'rejected')),
               disposition TEXT NOT NULL CHECK(disposition IN ('applicable', 'obsolete', 'superseded')),
               favorite INTEGER NOT NULL CHECK(favorite IN (0, 1)),
               todo INTEGER NOT NULL CHECK(todo IN (0, 1)),
               priority TEXT CHECK(priority IN ('low', 'medium', 'high')),
               superseded_by_subject_id TEXT,
               updated_at TEXT NOT NULL,
               CHECK(
                 (disposition = 'superseded' AND superseded_by_subject_id IS NOT NULL)
                 OR (disposition != 'superseded' AND superseded_by_subject_id IS NULL)
               ),
               FOREIGN KEY(subject_id) REFERENCES work_record_subjects(subject_id) ON DELETE CASCADE,
               FOREIGN KEY(superseded_by_subject_id) REFERENCES work_record_subjects(subject_id)
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
        migrate_generated_view_owners(&transaction)?;
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

fn migrate_generated_view_owners(connection: &Connection) -> Result<(), StoreError> {
    let columns = {
        let mut statement = connection.prepare("PRAGMA table_info(generated_views)")?;
        statement
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<BTreeSet<_>, _>>()?
    };
    if columns.contains("subject_id") {
        return Ok(());
    }
    if !columns.contains("bundle_id") {
        return Err(StoreError::InvalidStoredConfiguration(
            "generated view owner column is unavailable".to_owned(),
        ));
    }

    let indexes = load_all_indexes(connection)?;
    let mut owners = BTreeMap::new();
    for index in &indexes {
        for project in &index.projects {
            for bundle in &project.bundles {
                let member_paths = bundle
                    .bundle
                    .members
                    .iter()
                    .map(|member| member.relative_path.as_str())
                    .collect::<BTreeSet<_>>();
                let expected_format = match bundle.bundle.kind {
                    backstage_core::BundleKind::OpenSpecChange => "openspec",
                    backstage_core::BundleKind::PossibleArtifact => "planning-pattern",
                };
                if let Some(record) = project.records.iter().find(|record| {
                    record.locator.format_id == expected_format
                        && record
                            .sources
                            .iter()
                            .map(|source| source.relative_path.as_str())
                            .collect::<BTreeSet<_>>()
                            == member_paths
                }) {
                    owners.insert(bundle.bundle.id.clone(), record.subject_id.clone());
                }
            }
        }
    }

    let legacy_rows = {
        let mut statement = connection.prepare(
            "SELECT bundle_id, mode, source_fingerprint, prompt_version, included_paths,
                    generated_text, generated_at, model
             FROM generated_views",
        )?;
        statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?
    };
    connection.execute_batch(
        "CREATE TABLE generated_views_v2 (
           cache_key TEXT PRIMARY KEY,
           subject_id TEXT NOT NULL,
           mode TEXT NOT NULL,
           source_fingerprint TEXT NOT NULL,
           prompt_version TEXT NOT NULL,
           included_paths TEXT NOT NULL,
           generated_text TEXT NOT NULL,
           generated_at TEXT NOT NULL,
           model TEXT
         );",
    )?;
    for (
        bundle_id,
        mode,
        source_fingerprint,
        prompt_version,
        included_paths,
        generated_text,
        generated_at,
        model,
    ) in legacy_rows
    {
        let Some(subject_id) = owners.get(&bundle_id) else {
            continue;
        };
        connection.execute(
            "INSERT OR REPLACE INTO generated_views_v2 (
               cache_key, subject_id, mode, source_fingerprint, prompt_version,
               included_paths, generated_text, generated_at, model
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                generated_cache_key(subject_id, &mode, &source_fingerprint, &prompt_version),
                subject_id.as_str(),
                mode,
                source_fingerprint,
                prompt_version,
                included_paths,
                generated_text,
                generated_at,
                model,
            ],
        )?;
    }
    connection.execute_batch(
        "DROP TABLE generated_views;
         ALTER TABLE generated_views_v2 RENAME TO generated_views;",
    )?;
    Ok(())
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
    let rows = {
        let mut statement =
            connection.prepare("SELECT root_id, payload FROM index_snapshots ORDER BY root_id")?;
        statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?
    };
    let mut snapshots = Vec::new();
    for (root_id, payload) in rows {
        match serde_json::from_str::<IndexSnapshot>(&payload) {
            Ok(mut snapshot) => {
                let prior_version = snapshot.schema_version;
                migrate_legacy_snapshot(&mut snapshot);
                if snapshot.schema_version != prior_version {
                    connection.execute(
                        "UPDATE index_snapshots SET payload = ?1 WHERE root_id = ?2",
                        params![serde_json::to_string(&snapshot)?, root_id],
                    )?;
                }
                snapshots.push(snapshot);
            }
            Err(_) => {
                connection.execute("DELETE FROM index_snapshots WHERE root_id = ?1", [root_id])?;
            }
        }
    }
    Ok(snapshots)
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

fn decision_value(decision: Decision) -> &'static str {
    match decision {
        Decision::Undecided => "undecided",
        Decision::Approved => "approved",
        Decision::Rejected => "rejected",
    }
}

fn priority_value(priority: Priority) -> &'static str {
    match priority {
        Priority::Low => "low",
        Priority::Medium => "medium",
        Priority::High => "high",
    }
}

fn parse_decision(value: &str) -> Result<Decision, StoreError> {
    match value {
        "undecided" => Ok(Decision::Undecided),
        "approved" => Ok(Decision::Approved),
        "rejected" => Ok(Decision::Rejected),
        _ => Err(StoreError::InvalidStoredConfiguration(format!(
            "annotation decision is invalid: {value}"
        ))),
    }
}

fn parse_priority(value: &str) -> Result<Priority, StoreError> {
    match value {
        "low" => Ok(Priority::Low),
        "medium" => Ok(Priority::Medium),
        "high" => Ok(Priority::High),
        _ => Err(StoreError::InvalidStoredConfiguration(format!(
            "annotation priority is invalid: {value}"
        ))),
    }
}

fn parse_disposition(value: &str, replacement: Option<String>) -> Result<Disposition, StoreError> {
    match (value, replacement) {
        ("applicable", None) => Ok(Disposition::Applicable),
        ("obsolete", None) => Ok(Disposition::Obsolete),
        ("superseded", Some(replacement)) => Ok(Disposition::Superseded {
            replacement: SubjectId::from_trusted(replacement),
        }),
        _ => Err(StoreError::InvalidStoredConfiguration(format!(
            "annotation disposition is invalid: {value}"
        ))),
    }
}

fn generation_mode(mode: GenerationMode) -> &'static str {
    match mode {
        GenerationMode::Summary => "summary",
    }
}

fn generated_cache_key(
    subject_id: &SubjectId,
    mode: &str,
    fingerprint: &str,
    prompt_version: &str,
) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(
        format!(
            "{}\0{mode}\0{fingerprint}\0{prompt_version}",
            subject_id.as_str()
        )
        .as_bytes(),
    );
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
