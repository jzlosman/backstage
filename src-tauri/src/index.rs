use std::collections::BTreeMap;

use backstage_core::{
    ArtifactBundle, MarkdownDocument, OpenSpecPrimaryStatus, OpenSpecProgress, SourceFingerprint,
    assess_openspec_status,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::discovery::{ProjectCandidate, ScanWarning};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexSnapshot {
    pub root_id: String,
    pub generation: u64,
    pub indexed_at: String,
    #[serde(default)]
    pub configuration_revision: u64,
    pub projects: Vec<IndexedProject>,
    pub warnings: Vec<ScanWarning>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexedProject {
    pub project: ProjectCandidate,
    pub bundles: Vec<IndexedBundle>,
    #[serde(default)]
    pub markdown_documents: Vec<MarkdownDocument>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexedBundle {
    pub bundle: ArtifactBundle,
    pub progress: OpenSpecProgress,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_status: Option<OpenSpecPrimaryStatus>,
    pub fingerprint: Option<SourceFingerprint>,
    #[serde(with = "backstage_core::optional_u128_decimal_string")]
    pub source_modified_unix_nanos: Option<u128>,
    pub warnings: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexedBundleData {
    bundle: ArtifactBundle,
    progress: OpenSpecProgress,
    fingerprint: Option<SourceFingerprint>,
    #[serde(with = "backstage_core::optional_u128_decimal_string")]
    source_modified_unix_nanos: Option<u128>,
    warnings: Vec<String>,
}

impl<'de> Deserialize<'de> for IndexedBundle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data = IndexedBundleData::deserialize(deserializer)?;
        let primary_status = data
            .bundle
            .custody
            .as_ref()
            .map(|custody| assess_openspec_status(custody, &data.progress));
        Ok(Self {
            bundle: data.bundle,
            progress: data.progress,
            primary_status,
            fingerprint: data.fingerprint,
            source_modified_unix_nanos: data.source_modified_unix_nanos,
            warnings: data.warnings,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanPermit {
    pub root_id: String,
    pub generation: u64,
    pub configuration_revision: u64,
    pub admitted: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionDisposition {
    Accepted,
    Superseded,
}

#[derive(Default)]
pub struct ScanCoordinator {
    roots: Mutex<BTreeMap<String, RootScanState>>,
    next_generation: Mutex<u64>,
}

#[derive(Default)]
struct RootScanState {
    latest_generation: u64,
    configuration_revision: u64,
    current: Option<IndexSnapshot>,
    failure: Option<String>,
}

impl ScanCoordinator {
    pub fn begin(&self, root_id: impl Into<String>) -> ScanPermit {
        self.begin_for_revision(root_id, 0)
    }

    pub fn begin_for_revision(
        &self,
        root_id: impl Into<String>,
        configuration_revision: u64,
    ) -> ScanPermit {
        let root_id = root_id.into();
        let generation = {
            let mut next_generation = self.next_generation.lock();
            *next_generation += 1;
            *next_generation
        };
        let mut roots = self.roots.lock();
        let state = roots.entry(root_id.clone()).or_default();
        let admitted = configuration_revision >= state.configuration_revision;
        if admitted {
            state.latest_generation = generation;
            state.configuration_revision = configuration_revision;
            state.failure = None;
        }
        ScanPermit {
            root_id,
            generation,
            configuration_revision,
            admitted,
        }
    }

    pub fn complete(&self, permit: &ScanPermit, snapshot: IndexSnapshot) -> CompletionDisposition {
        let mut roots = self.roots.lock();
        let Some(state) = roots.get_mut(&permit.root_id) else {
            return CompletionDisposition::Superseded;
        };
        if state.latest_generation != permit.generation
            || state.configuration_revision != permit.configuration_revision
            || snapshot.generation != permit.generation
            || snapshot.configuration_revision != permit.configuration_revision
            || snapshot.root_id != permit.root_id
        {
            return CompletionDisposition::Superseded;
        }
        state.current = Some(snapshot);
        state.failure = None;
        CompletionDisposition::Accepted
    }

    pub fn fail(&self, permit: &ScanPermit, message: impl Into<String>) {
        let mut roots = self.roots.lock();
        if let Some(state) = roots.get_mut(&permit.root_id)
            && state.latest_generation == permit.generation
            && state.configuration_revision == permit.configuration_revision
        {
            state.failure = Some(message.into());
        }
    }

    pub fn cancel(&self, permit: &ScanPermit) {
        let mut roots = self.roots.lock();
        if let Some(state) = roots.get_mut(&permit.root_id)
            && state.latest_generation == permit.generation
        {
            state.latest_generation = state.latest_generation.saturating_add(1);
        }
    }

    pub fn forget(&self, root_id: &str) {
        self.roots.lock().remove(root_id);
    }

    pub fn current(&self, root_id: &str) -> Option<IndexSnapshot> {
        self.roots
            .lock()
            .get(root_id)
            .and_then(|state| state.current.clone())
    }

    pub fn failure(&self, root_id: &str) -> Option<String> {
        self.roots
            .lock()
            .get(root_id)
            .and_then(|state| state.failure.clone())
    }

    pub fn hydrate(&self, snapshot: IndexSnapshot) {
        {
            let mut next_generation = self.next_generation.lock();
            *next_generation = (*next_generation).max(snapshot.generation);
        }
        let mut roots = self.roots.lock();
        let state = roots.entry(snapshot.root_id.clone()).or_default();
        if snapshot.configuration_revision < state.configuration_revision
            || (snapshot.configuration_revision == state.configuration_revision
                && snapshot.generation < state.latest_generation)
        {
            return;
        }
        state.latest_generation = snapshot.generation;
        state.configuration_revision = snapshot.configuration_revision;
        state.current = Some(snapshot);
    }
}

pub trait IndexPersistence {
    fn save_index(&self, snapshot: &IndexSnapshot) -> Result<(), String>;
    fn load_index(&self, root_id: &str) -> Result<Option<IndexSnapshot>, String>;
}

#[derive(Default)]
pub struct IndexSession {
    current: Option<IndexSnapshot>,
    operational_warnings: Vec<String>,
}

impl IndexSession {
    pub fn publish(&mut self, snapshot: IndexSnapshot, persistence: &dyn IndexPersistence) {
        self.current = Some(snapshot);
        if let Some(current) = &self.current
            && let Err(error) = persistence.save_index(current)
        {
            self.operational_warnings.push(format!(
                "Index is usable in memory but could not be cached: {error}"
            ));
        }
    }

    pub fn current(&self) -> Option<&IndexSnapshot> {
        self.current.as_ref()
    }

    pub fn operational_warnings(&self) -> &[String] {
        &self.operational_warnings
    }
}
