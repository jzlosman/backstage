use std::collections::BTreeMap;

use backstage_core::{
    AdapterDescriptor, ArtifactBundle, ArtifactRecognition, BundleKind, Capability, FactProvenance,
    FactValue, MarkdownDocument, OpenSpecCustody, OpenSpecPrimaryStatus, OpenSpecProgress,
    RecognitionLevel, RecordLocator, SourceFingerprint, SummaryFact, WorkRecord,
    WorkRecordRecognition, WorkRecordSource, WorkRecordWarning, assess_openspec_status,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::discovery::{ProjectCandidate, ScanWarning};

pub const CURRENT_INDEX_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexSnapshot {
    #[serde(default)]
    pub schema_version: u32,
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
    #[serde(default)]
    pub records: Vec<WorkRecord>,
    #[serde(default)]
    pub source_count: usize,
    #[serde(default)]
    pub registry_warnings: Vec<WorkRecordWarning>,
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

pub fn migrate_legacy_snapshot(snapshot: &mut IndexSnapshot) {
    if snapshot.schema_version >= CURRENT_INDEX_SCHEMA_VERSION {
        return;
    }
    for project in &mut snapshot.projects {
        if project.records.is_empty() {
            project.records = translate_legacy_project(project);
        }
        if project.source_count == 0 {
            project.source_count = project
                .markdown_documents
                .iter()
                .map(|document| document.relative_path.as_str())
                .chain(
                    project
                        .bundles
                        .iter()
                        .flat_map(|bundle| &bundle.bundle.members)
                        .map(|member| member.relative_path.as_str()),
                )
                .collect::<std::collections::BTreeSet<_>>()
                .len();
        }
    }
    snapshot.schema_version = CURRENT_INDEX_SCHEMA_VERSION;
}

fn translate_legacy_project(project: &IndexedProject) -> Vec<WorkRecord> {
    let mut records = Vec::new();
    let mut claimed = std::collections::BTreeSet::new();
    for indexed in &project.bundles {
        let Some((descriptor, record_key, level, capabilities)) = legacy_bundle_contract(indexed)
        else {
            continue;
        };
        let sources = indexed
            .bundle
            .members
            .iter()
            .map(|member| {
                claimed.insert(member.relative_path.clone());
                WorkRecordSource::new(
                    member.relative_path.clone(),
                    project
                        .markdown_documents
                        .iter()
                        .find(|document| document.relative_path == member.relative_path)
                        .and_then(|document| document.source_modified_unix_nanos),
                )
            })
            .collect::<Vec<_>>();
        let evidence = match &indexed.bundle.recognition {
            ArtifactRecognition::Recognized { detector } => {
                vec![format!("Translated from legacy recognition by {detector}")]
            }
            ArtifactRecognition::Possible { reason } => vec![reason.clone()],
        };
        let recognition = WorkRecordRecognition::new(level, &descriptor, evidence);
        let mut facts = vec![SummaryFact::new(
            "work_record.source_count",
            "Sources",
            FactValue::Count(sources.len() as u64),
            FactProvenance::new(
                descriptor.adapter_id(),
                sources
                    .iter()
                    .map(|source| source.relative_path.clone())
                    .collect(),
            ),
        )];
        if indexed.bundle.kind == BundleKind::OpenSpecChange {
            let provenance = indexed
                .bundle
                .members
                .iter()
                .filter(|member| member.relative_path.ends_with("/tasks.md"))
                .map(|member| member.relative_path.clone())
                .collect::<Vec<_>>();
            if let Some(custody) = &indexed.bundle.custody {
                facts.push(SummaryFact::new(
                    "openspec.custody",
                    "Custody",
                    FactValue::Text(
                        match custody {
                            OpenSpecCustody::Current => "current",
                            OpenSpecCustody::Archived { .. } => "archived",
                        }
                        .to_owned(),
                    ),
                    FactProvenance::new(descriptor.adapter_id(), vec![]),
                ));
            }
            if let Some(status) = indexed.primary_status {
                facts.push(SummaryFact::new(
                    "openspec.primary_status",
                    "Status",
                    FactValue::Text(
                        match status {
                            OpenSpecPrimaryStatus::Active => "active",
                            OpenSpecPrimaryStatus::Done => "done",
                            OpenSpecPrimaryStatus::Archived => "archived",
                        }
                        .to_owned(),
                    ),
                    FactProvenance::new(descriptor.adapter_id(), provenance.clone()),
                ));
            }
            if let OpenSpecProgress::Available(progress) = &indexed.progress {
                facts.extend([
                    SummaryFact::new(
                        "openspec.task.total_count",
                        "Tasks",
                        FactValue::Count(progress.total as u64),
                        FactProvenance::new(descriptor.adapter_id(), provenance.clone()),
                    ),
                    SummaryFact::new(
                        "openspec.task.done_count",
                        "Done",
                        FactValue::Count(progress.completed as u64),
                        FactProvenance::new(descriptor.adapter_id(), provenance.clone()),
                    ),
                    SummaryFact::new(
                        "openspec.task.open_count",
                        "Open",
                        FactValue::Count(progress.remaining_count as u64),
                        FactProvenance::new(descriptor.adapter_id(), provenance),
                    ),
                ]);
            }
        }
        let warnings = indexed
            .warnings
            .iter()
            .map(|warning| WorkRecordWarning::without_source("legacy_index_warning", warning))
            .collect();
        let mut record = WorkRecord::new(
            RecordLocator::new(&project.project.id, descriptor.format_id(), record_key),
            indexed.bundle.name.clone(),
            recognition,
            sources,
            facts,
            warnings,
            capabilities,
        );
        if record.source_modified_unix_nanos.is_none() {
            record.source_modified_unix_nanos = indexed.source_modified_unix_nanos;
        }
        record.fingerprint = indexed.fingerprint.clone();
        records.push(record);
    }

    let markdown_descriptor = AdapterDescriptor::new("markdown-v1", "markdown", 1, 40);
    for document in &project.markdown_documents {
        if claimed.contains(&document.relative_path) {
            continue;
        }
        records.push(WorkRecord::new(
            RecordLocator::new(
                &project.project.id,
                markdown_descriptor.format_id(),
                &document.relative_path,
            ),
            document
                .relative_path
                .rsplit('/')
                .next()
                .unwrap_or(&document.relative_path),
            WorkRecordRecognition::new(
                RecognitionLevel::Plain,
                &markdown_descriptor,
                vec!["Translated plain Markdown fallback".to_owned()],
            ),
            vec![WorkRecordSource::new(
                document.relative_path.clone(),
                document.source_modified_unix_nanos,
            )],
            vec![],
            vec![],
            vec![Capability::new("source", "Source")],
        ));
    }
    records.sort_by(|left, right| {
        left.display_name
            .to_lowercase()
            .cmp(&right.display_name.to_lowercase())
            .then_with(|| left.subject_id.cmp(&right.subject_id))
    });
    records
}

fn legacy_bundle_contract(
    indexed: &IndexedBundle,
) -> Option<(AdapterDescriptor, String, RecognitionLevel, Vec<Capability>)> {
    match indexed.bundle.kind {
        BundleKind::OpenSpecChange => Some((
            AdapterDescriptor::new("openspec-v1", "openspec", 1, 10),
            openspec_record_key(&indexed.bundle.members.first()?.relative_path)?,
            RecognitionLevel::Recognized,
            vec![
                Capability::new("overview", "Overview"),
                Capability::new("tasks", "Tasks"),
                Capability::new("source", "Source"),
            ],
        )),
        BundleKind::PossibleArtifact => Some((
            AdapterDescriptor::new("planning-pattern-v1", "planning-pattern", 1, 30),
            indexed.bundle.members.first()?.relative_path.clone(),
            RecognitionLevel::Possible,
            vec![Capability::new("source", "Source")],
        )),
    }
}

fn openspec_record_key(relative_path: &str) -> Option<String> {
    let parts = relative_path.split('/').collect::<Vec<_>>();
    if parts.first() != Some(&"openspec") || parts.get(1) != Some(&"changes") {
        return None;
    }
    let length = if parts.get(2) == Some(&"archive") {
        4
    } else {
        3
    };
    (parts.len() > length).then(|| parts[..length].join("/"))
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
