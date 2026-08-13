use std::collections::BTreeMap;

use backstage_core::{ArtifactBundle, MarkdownDocument, OpenSpecProgress, SourceFingerprint};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::discovery::{ProjectCandidate, ScanWarning};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexSnapshot {
    pub root_id: String,
    pub generation: u64,
    pub indexed_at: String,
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexedBundle {
    pub bundle: ArtifactBundle,
    pub progress: OpenSpecProgress,
    pub fingerprint: Option<SourceFingerprint>,
    pub source_modified_unix_nanos: Option<u128>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanPermit {
    pub root_id: String,
    pub generation: u64,
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
}

#[derive(Default)]
struct RootScanState {
    latest_generation: u64,
    current: Option<IndexSnapshot>,
    failure: Option<String>,
}

impl ScanCoordinator {
    pub fn begin(&self, root_id: impl Into<String>) -> ScanPermit {
        let root_id = root_id.into();
        let mut roots = self.roots.lock();
        let state = roots.entry(root_id.clone()).or_default();
        state.latest_generation += 1;
        state.failure = None;
        ScanPermit {
            root_id,
            generation: state.latest_generation,
        }
    }

    pub fn complete(&self, permit: &ScanPermit, snapshot: IndexSnapshot) -> CompletionDisposition {
        let mut roots = self.roots.lock();
        let state = roots.entry(permit.root_id.clone()).or_default();
        if state.latest_generation != permit.generation
            || snapshot.generation != permit.generation
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
        let state = roots.entry(permit.root_id.clone()).or_default();
        if state.latest_generation == permit.generation {
            state.failure = Some(message.into());
        }
    }

    pub fn cancel(&self, permit: &ScanPermit) {
        let mut roots = self.roots.lock();
        let state = roots.entry(permit.root_id.clone()).or_default();
        if state.latest_generation == permit.generation {
            state.latest_generation += 1;
        }
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
        let mut roots = self.roots.lock();
        let state = roots.entry(snapshot.root_id.clone()).or_default();
        state.latest_generation = state.latest_generation.max(snapshot.generation);
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
