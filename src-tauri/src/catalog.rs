use std::path::{Path, PathBuf};

use backstage_core::{
    AdapterHandoff, AdapterSummary, ArtifactBundle, ArtifactRecognition, BundleKind,
    CapabilityView, DetectedRecord, DetectedWorkRecord, DetectorEvidence, EvidenceKind,
    FormatRegistry, MarkdownAdapter, MarkdownDocument, OpenSpecAdapter, OpenSpecCustody,
    OpenSpecPrimaryStatus, OpenSpecProgress, OpenSpecSource, OpenSpecView, ParseWarning,
    ParserProvenance, PlanningFormatAdapter, PlanningPattern, PlanningPatternAdapter,
    ProgressFallback, ProjectSourceInventory, RecordSourceCapture, SnapshotError,
    SourceCaptureFailure, SourceClaim, SourceInventoryEntry, SourceSnapshot, SubjectId,
    WayfinderLocalAdapter, WorkRecord, WorkRecordWarning, assess_openspec_status,
    build_openspec_view, canonical_planning_patterns, classify_project,
    fingerprint_complete_snapshots, is_supported_openspec_member, matching_planning_patterns,
    parse_openspec_tasks,
};
use serde::Serialize;

use crate::discovery::{CancellationToken, ProjectCandidate, ScanPolicy, ScanWarning};
use crate::filesystem::ContainedReader;
use crate::index::{CURRENT_INDEX_SCHEMA_VERSION, IndexSnapshot, IndexedBundle, IndexedProject};

pub fn build_index(
    reader: &ContainedReader,
    projects: Vec<ProjectCandidate>,
    generation: u64,
    indexed_at: String,
    warnings: Vec<ScanWarning>,
) -> IndexSnapshot {
    build_index_with_patterns(
        reader,
        projects,
        generation,
        0,
        indexed_at,
        warnings,
        &canonical_planning_patterns(),
    )
}

pub fn build_index_with_patterns(
    reader: &ContainedReader,
    projects: Vec<ProjectCandidate>,
    generation: u64,
    configuration_revision: u64,
    indexed_at: String,
    warnings: Vec<ScanWarning>,
    patterns: &[PlanningPattern],
) -> IndexSnapshot {
    build_index_controlled_with_patterns(
        reader,
        projects,
        generation,
        configuration_revision,
        indexed_at,
        warnings,
        patterns,
        &ScanPolicy::default(),
        &CancellationToken::new(),
    )
}

pub fn build_index_controlled(
    reader: &ContainedReader,
    projects: Vec<ProjectCandidate>,
    generation: u64,
    indexed_at: String,
    warnings: Vec<ScanWarning>,
    policy: &ScanPolicy,
    cancellation: &CancellationToken,
) -> IndexSnapshot {
    build_index_controlled_with_patterns(
        reader,
        projects,
        generation,
        0,
        indexed_at,
        warnings,
        &canonical_planning_patterns(),
        policy,
        cancellation,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_index_controlled_with_patterns(
    reader: &ContainedReader,
    projects: Vec<ProjectCandidate>,
    generation: u64,
    configuration_revision: u64,
    indexed_at: String,
    warnings: Vec<ScanWarning>,
    patterns: &[PlanningPattern],
    policy: &ScanPolicy,
    cancellation: &CancellationToken,
) -> IndexSnapshot {
    build_index_controlled_with_patterns_and_checkpoint(
        reader,
        projects,
        generation,
        configuration_revision,
        indexed_at,
        warnings,
        patterns,
        policy,
        cancellation,
        |_| {},
    )
}

#[allow(clippy::too_many_arguments)]
fn build_index_controlled_with_patterns_and_checkpoint(
    reader: &ContainedReader,
    projects: Vec<ProjectCandidate>,
    generation: u64,
    configuration_revision: u64,
    indexed_at: String,
    mut warnings: Vec<ScanWarning>,
    patterns: &[PlanningPattern],
    policy: &ScanPolicy,
    cancellation: &CancellationToken,
    mut checkpoint: impl FnMut(CatalogCheckpoint),
) -> IndexSnapshot {
    let mut indexed_projects = Vec::new();
    for project in projects {
        if cancellation.is_cancelled() {
            warnings.push(partial_warning(
                ScanStopReason::Cancelled,
                Path::new(&project.root_path),
                policy,
            ));
            break;
        }
        let project_path = project.root_path.clone();
        let (indexed, stopped) = index_project(
            reader,
            project,
            patterns,
            policy,
            cancellation,
            &mut checkpoint,
        );
        indexed_projects.push(indexed);
        if let Some(reason) = stopped {
            warnings.push(partial_warning(reason, Path::new(&project_path), policy));
            if reason == ScanStopReason::Cancelled {
                break;
            }
        }
    }
    for project in &indexed_projects {
        for bundle in &project.bundles {
            for message in &bundle.warnings {
                warnings.push(ScanWarning {
                    code: "artifact_warning".to_owned(),
                    path: bundle.bundle.name.clone(),
                    message: message.clone(),
                });
            }
        }
    }
    IndexSnapshot {
        schema_version: CURRENT_INDEX_SCHEMA_VERSION,
        root_id: backstage_core::ApprovedRoot::new(reader.root(), true)
            .expect("contained reader root is a canonical directory")
            .id()
            .to_owned(),
        generation,
        indexed_at,
        configuration_revision,
        projects: indexed_projects,
        warnings,
    }
}

fn index_project(
    reader: &ContainedReader,
    project: ProjectCandidate,
    patterns: &[PlanningPattern],
    policy: &ScanPolicy,
    cancellation: &CancellationToken,
    checkpoint: &mut impl FnMut(CatalogCheckpoint),
) -> (IndexedProject, Option<ScanStopReason>) {
    let budget = ProjectScanBudget::new(policy, cancellation);
    let detected = detect_project_content(
        reader,
        &project,
        Path::new(&project.root_path),
        patterns,
        policy,
        &budget,
    );
    let mut bundles = Vec::new();
    if budget.stop_reason().is_none() {
        for bundle in classify_project(&project.id, &project.name, detected.evidence.clone()) {
            checkpoint(CatalogCheckpoint::BeforeBundle);
            if budget.stop_reason().is_some() {
                break;
            }
            let (indexed, complete) = index_bundle(
                reader,
                &project,
                bundle,
                &detected.markdown_documents,
                &budget,
                checkpoint,
            );
            bundles.push(indexed);
            if !complete {
                break;
            }
        }
    }

    let registry = FormatRegistry::new(vec![
        Box::new(OpenSpecAdapter::new()),
        Box::new(WayfinderLocalAdapter::new()),
        Box::new(PlanningPatternAdapter::new(patterns.to_vec())),
        Box::new(MarkdownAdapter::new()),
    ]);
    let registry_detection = registry.detect(&detected.inventory);
    let source_count = registry_detection.source_count;
    let registry_warnings = registry_detection.warnings;
    let mut records = Vec::new();
    for record in registry_detection.records {
        checkpoint(CatalogCheckpoint::BeforeBundle);
        let (indexed, _) = index_work_record(
            reader,
            &project,
            &detected.inventory,
            &registry,
            record,
            &budget,
            checkpoint,
        );
        if let Some(indexed) = indexed {
            records.push(indexed);
        }
    }
    let stopped = budget.stop_reason();
    (
        IndexedProject {
            project,
            bundles,
            markdown_documents: detected.markdown_documents,
            records,
            source_count,
            registry_warnings,
        },
        stopped,
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScanStopReason {
    Cancelled,
    TimedOut,
}

struct ProjectScanBudget<'a> {
    started: std::time::Instant,
    timeout: std::time::Duration,
    cancellation: &'a CancellationToken,
}

impl<'a> ProjectScanBudget<'a> {
    fn new(policy: &ScanPolicy, cancellation: &'a CancellationToken) -> Self {
        Self {
            started: std::time::Instant::now(),
            timeout: std::time::Duration::from_millis(policy.timeout_ms),
            cancellation,
        }
    }

    fn stop_reason(&self) -> Option<ScanStopReason> {
        if self.cancellation.is_cancelled() {
            Some(ScanStopReason::Cancelled)
        } else if self.started.elapsed() >= self.timeout {
            Some(ScanStopReason::TimedOut)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
enum CatalogCheckpoint {
    BeforeBundle,
    BeforeMember,
}

fn partial_warning(reason: ScanStopReason, path: &Path, policy: &ScanPolicy) -> ScanWarning {
    match reason {
        ScanStopReason::Cancelled => ScanWarning {
            code: "artifact_index_cancelled".to_owned(),
            path: path_string(path),
            message: "Artifact indexing was cancelled; the index contains bounded partial results"
                .to_owned(),
        },
        ScanStopReason::TimedOut => ScanWarning {
            code: "artifact_index_timeout".to_owned(),
            path: path_string(path),
            message: format!(
                "Artifact indexing stopped after {} ms; the index contains bounded partial results",
                policy.timeout_ms
            ),
        },
    }
}

struct DetectedProjectContent {
    evidence: Vec<DetectorEvidence>,
    markdown_documents: Vec<MarkdownDocument>,
    inventory: ProjectSourceInventory,
}

fn detect_project_content(
    reader: &ContainedReader,
    project: &ProjectCandidate,
    project_root: &Path,
    patterns: &[PlanningPattern],
    policy: &ScanPolicy,
    budget: &ProjectScanBudget<'_>,
) -> DetectedProjectContent {
    let mut evidence = Vec::new();
    let mut markdown_documents = Vec::new();
    let mut inventory_sources = Vec::new();
    let excluded = policy.exclusions.iter().cloned().collect();
    for entry in reader.walk_from(
        project_root,
        policy.max_depth,
        policy.max_entries,
        &excluded,
        || budget.stop_reason().is_some(),
    ) {
        if budget.stop_reason().is_some() {
            break;
        }
        let Ok((path, file_type)) = entry else {
            continue;
        };
        if !file_type.is_file() || !path.starts_with(project_root) {
            continue;
        }
        let Ok(relative) = path.strip_prefix(project_root) else {
            continue;
        };
        let relative = path_string(relative);
        if !is_markdown(&relative) {
            continue;
        }
        let Ok(observation) = reader.regular_file_observation(&path) else {
            continue;
        };
        markdown_documents.push(MarkdownDocument::new(
            &project.id,
            &project.name,
            &relative,
            observation.modified_unix_nanos,
        ));
        inventory_sources.push(SourceInventoryEntry::new(relative.clone(), observation));
        if is_supported_openspec_member(&relative) {
            evidence.push(DetectorEvidence::new(
                relative,
                EvidenceKind::OpenSpecMember,
                "Path is supported OpenSpec change material",
            ));
        } else {
            let matches = matching_planning_patterns(&relative, patterns);
            if !matches.is_empty() {
                let accepted = matches
                    .into_iter()
                    .map(|pattern| format!("{} ({})", pattern.id(), pattern.expression()))
                    .collect::<Vec<_>>()
                    .join(", ");
                evidence.push(DetectorEvidence::new(
                    relative.clone(),
                    EvidenceKind::PlanningPatternMatch,
                    format!("Path matches configured planning pattern(s): {accepted}"),
                ));
            }
        }
    }
    markdown_documents.sort_by(|left, right| {
        left.relative_path
            .cmp(&right.relative_path)
            .then_with(|| left.id.cmp(&right.id))
    });
    DetectedProjectContent {
        evidence,
        markdown_documents,
        inventory: ProjectSourceInventory::new(
            project.id.clone(),
            project.name.clone(),
            inventory_sources,
        ),
    }
}

fn is_markdown(relative: &str) -> bool {
    Path::new(relative)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
}

fn index_work_record(
    reader: &ContainedReader,
    project: &ProjectCandidate,
    inventory: &ProjectSourceInventory,
    registry: &FormatRegistry,
    record: DetectedWorkRecord,
    budget: &ProjectScanBudget<'_>,
    checkpoint: &mut impl FnMut(CatalogCheckpoint),
) -> (Option<WorkRecord>, bool) {
    let mut snapshots = Vec::new();
    let mut failures = Vec::new();
    let mut complete = true;
    for claim in &record.claims {
        checkpoint(CatalogCheckpoint::BeforeMember);
        if budget.stop_reason().is_some() {
            complete = false;
            break;
        }
        let absolute = Path::new(&project.root_path).join(&claim.relative_path);
        match capture_project_snapshot(reader, &absolute, &claim.relative_path) {
            Ok(snapshot) => snapshots.push(snapshot),
            Err(failure) => failures.push(failure),
        }
    }
    let capture = RecordSourceCapture::partial(snapshots, failures);
    (
        registry.summarize(inventory, &record, &capture).ok(),
        complete,
    )
}

fn capture_project_snapshot(
    reader: &ContainedReader,
    absolute_path: &Path,
    project_relative_path: &str,
) -> Result<SourceSnapshot, SourceCaptureFailure> {
    let snapshot = reader.read_snapshot(absolute_path).map_err(|error| {
        SourceCaptureFailure::new(
            project_relative_path,
            "source_unavailable",
            format!("Source could not be captured safely: {error}"),
        )
    })?;
    snapshot
        .with_relative_path(project_relative_path)
        .map_err(|error| {
            SourceCaptureFailure::new(
                project_relative_path,
                "source_path_invalid",
                format!("Source path could not be normalized safely: {error}"),
            )
        })
}

fn index_bundle(
    reader: &ContainedReader,
    project: &ProjectCandidate,
    bundle: ArtifactBundle,
    markdown_documents: &[MarkdownDocument],
    budget: &ProjectScanBudget<'_>,
    checkpoint: &mut impl FnMut(CatalogCheckpoint),
) -> (IndexedBundle, bool) {
    let mut snapshots = Vec::new();
    let mut warnings = Vec::new();
    let mut progress = OpenSpecProgress::Unavailable(backstage_core::ProgressFallback {
        parser: backstage_core::ParserProvenance {
            name: "openspec-task-markers".to_owned(),
            version: "1".to_owned(),
        },
        warnings: vec![],
    });

    let mut complete = true;
    for member in &bundle.members {
        checkpoint(CatalogCheckpoint::BeforeMember);
        if budget.stop_reason().is_some() {
            complete = false;
            break;
        }
        let absolute = Path::new(&project.root_path).join(&member.relative_path);
        match reader.read_snapshot(&absolute) {
            Ok(snapshot) => {
                if member.relative_path.ends_with("/tasks.md") || member.relative_path == "tasks.md"
                {
                    if let Some(text) = snapshot.text() {
                        progress = parse_openspec_tasks(text);
                    } else {
                        warnings.push(format!(
                            "{} is not UTF-8; deterministic progress is unavailable",
                            member.relative_path
                        ));
                    }
                }
                snapshots.push(snapshot);
            }
            Err(error) => warnings.push(format!(
                "{} remained indexed but could not be read: {error}",
                member.relative_path
            )),
        }
    }

    let member_paths = bundle
        .members
        .iter()
        .map(|member| member.relative_path.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let source_modified_unix_nanos = markdown_documents
        .iter()
        .filter(|document| member_paths.contains(document.relative_path.as_str()))
        .filter_map(|document| document.source_modified_unix_nanos)
        .max();
    let fingerprint = match fingerprint_complete_snapshots(bundle.members.len(), &snapshots) {
        Ok(fingerprint) => Some(fingerprint),
        Err(SnapshotError::IncompleteManifest) => {
            warnings.push(
                "Source fingerprint unavailable because the manifest is incomplete".to_owned(),
            );
            None
        }
        Err(error) => {
            warnings.push(format!("Source fingerprint unavailable: {error}"));
            None
        }
    };

    let primary_status = bundle
        .custody
        .as_ref()
        .map(|custody| assess_openspec_status(custody, &progress));
    (
        IndexedBundle {
            bundle,
            progress,
            primary_status,
            fingerprint,
            source_modified_unix_nanos,
            warnings,
        },
        complete,
    )
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkRecordDetail {
    pub root_id: String,
    pub subject_id: SubjectId,
    pub index_generation: u64,
    pub project_id: String,
    pub project_name: String,
    pub project_root: String,
    pub git: Option<crate::discovery::GitContext>,
    pub record: WorkRecord,
    pub capabilities: Vec<CapabilityView>,
    pub fingerprint: Option<backstage_core::SourceFingerprint>,
    pub warnings: Vec<WorkRecordWarning>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkRecordHandoffDetail {
    pub root_id: String,
    pub subject_id: SubjectId,
    pub index_generation: u64,
    pub handoff: AdapterHandoff,
}

pub fn work_record_detail(
    reader: &ContainedReader,
    index: &IndexSnapshot,
    subject_id: &str,
) -> Result<WorkRecordDetail, CatalogError> {
    let (project, record) = find_work_record(index, subject_id)?;
    let adapter = adapter_for_record(record)?;
    let detected = detected_record(record);
    let capture = capture_work_record_sources(reader, project, record);
    let capabilities = adapter
        .build_detail(&detected, &capture)
        .map_err(|error| CatalogError::Read(error.to_string()))?;
    let AdapterSummary {
        facts,
        warnings: summary_warnings,
        capabilities: summary_capabilities,
        fingerprint,
    } = adapter
        .summarize(&detected, &capture)
        .map_err(|error| CatalogError::Read(error.to_string()))?;
    let mut warnings = record
        .warnings
        .iter()
        .filter(|warning| warning.code == "adapter_claim_overlap")
        .cloned()
        .collect::<Vec<_>>();
    warnings.extend(summary_warnings);
    warnings.extend(capture.failures.iter().map(|failure| {
        WorkRecordWarning::new(
            failure.code.clone(),
            failure.message.clone(),
            Some(failure.relative_path.clone()),
        )
    }));
    warnings.sort();
    warnings.dedup();
    let mut fresh_record = record.clone();
    fresh_record.facts = facts;
    fresh_record.capabilities = summary_capabilities;
    fresh_record.warnings = warnings.clone();
    fresh_record.fingerprint = fingerprint.clone();
    for source in &mut fresh_record.sources {
        if let Some(snapshot) = capture.snapshot(&source.relative_path) {
            source.source_modified_unix_nanos = snapshot.observation().modified_unix_nanos;
        }
    }
    fresh_record.source_modified_unix_nanos = fresh_record
        .sources
        .iter()
        .filter_map(|source| source.source_modified_unix_nanos)
        .max();
    Ok(WorkRecordDetail {
        root_id: index.root_id.clone(),
        subject_id: record.subject_id.clone(),
        index_generation: index.generation,
        project_id: project.project.id.clone(),
        project_name: project.project.name.clone(),
        project_root: project.project.root_path.clone(),
        git: project.project.git.clone(),
        record: fresh_record,
        capabilities,
        fingerprint,
        warnings,
    })
}

pub fn work_record_handoff(
    reader: &ContainedReader,
    index: &IndexSnapshot,
    subject_id: &str,
) -> Result<WorkRecordHandoffDetail, CatalogError> {
    let (project, record) = find_work_record(index, subject_id)?;
    let adapter = adapter_for_record(record)?;
    let detected = detected_record(record);
    let capture = capture_work_record_sources(reader, project, record);
    let handoff = adapter
        .build_handoff(&detected, &capture)
        .map_err(|error| CatalogError::Read(error.to_string()))?;
    Ok(WorkRecordHandoffDetail {
        root_id: index.root_id.clone(),
        subject_id: record.subject_id.clone(),
        index_generation: index.generation,
        handoff,
    })
}

fn find_work_record<'a>(
    index: &'a IndexSnapshot,
    subject_id: &str,
) -> Result<(&'a IndexedProject, &'a WorkRecord), CatalogError> {
    index
        .projects
        .iter()
        .find_map(|project| {
            project
                .records
                .iter()
                .find(|record| record.subject_id.as_str() == subject_id)
                .map(|record| (project, record))
        })
        .ok_or(CatalogError::NotFound)
}

fn detected_record(record: &WorkRecord) -> DetectedRecord {
    DetectedRecord::new(
        record.locator.adapter_record_key.clone(),
        record.display_name.clone(),
        record.recognition.level,
        record
            .sources
            .iter()
            .map(|source| SourceClaim::new(source.relative_path.clone()))
            .collect(),
        record.recognition.evidence.clone(),
        record.capabilities.clone(),
    )
}

fn capture_work_record_sources(
    reader: &ContainedReader,
    project: &IndexedProject,
    record: &WorkRecord,
) -> RecordSourceCapture {
    let mut snapshots = Vec::new();
    let mut failures = Vec::new();
    for source in &record.sources {
        let absolute = Path::new(&project.project.root_path).join(&source.relative_path);
        match capture_project_snapshot(reader, &absolute, &source.relative_path) {
            Ok(snapshot) => snapshots.push(snapshot),
            Err(failure) => failures.push(failure),
        }
    }
    RecordSourceCapture::partial(snapshots, failures)
}

fn adapter_for_record(record: &WorkRecord) -> Result<Box<dyn PlanningFormatAdapter>, CatalogError> {
    match record.locator.format_id.as_str() {
        "openspec" => Ok(Box::new(OpenSpecAdapter::new())),
        "wayfinder-local" => Ok(Box::new(WayfinderLocalAdapter::new())),
        "planning-pattern" => Ok(Box::new(PlanningPatternAdapter::new(vec![]))),
        "markdown" => Ok(Box::new(MarkdownAdapter::new())),
        format_id => Err(CatalogError::Read(format!(
            "No compiled adapter is available for format {format_id}"
        ))),
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownDetail {
    pub root_id: String,
    pub document_id: String,
    pub project_id: String,
    pub project_name: String,
    pub project_root: String,
    pub git: Option<crate::discovery::GitContext>,
    pub relative_path: String,
    pub absolute_path: String,
    #[serde(serialize_with = "backstage_core::optional_u128_decimal_string::serialize")]
    pub source_modified_unix_nanos: Option<u128>,
    pub markdown: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactDetail {
    pub root_id: String,
    pub artifact_id: String,
    pub bundle_id: String,
    pub project_id: String,
    pub project_name: String,
    pub project_root: String,
    pub git: Option<crate::discovery::GitContext>,
    pub bundle_name: String,
    pub members: Vec<backstage_core::ArtifactMember>,
    pub bundle_kind: BundleKind,
    pub recognition: ArtifactRecognition,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custody: Option<OpenSpecCustody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_status: Option<OpenSpecPrimaryStatus>,
    pub relative_path: String,
    pub absolute_path: String,
    #[serde(serialize_with = "backstage_core::optional_u128_decimal_string::serialize")]
    pub source_modified_unix_nanos: Option<u128>,
    pub markdown: String,
    pub progress: OpenSpecProgress,
    pub fingerprint: Option<String>,
    pub warnings: Vec<String>,
    pub open_spec_view: Option<OpenSpecView>,
}

pub struct LiveBundleState {
    pub progress: OpenSpecProgress,
    pub fingerprint: backstage_core::SourceFingerprint,
    pub warnings: Vec<String>,
}

struct ArtifactDetailCapture {
    progress: OpenSpecProgress,
    fingerprint: Option<backstage_core::SourceFingerprint>,
    warnings: Vec<String>,
    snapshots: std::collections::BTreeMap<String, backstage_core::SourceSnapshot>,
    read_failures: std::collections::BTreeMap<String, String>,
}

pub fn live_bundle_state(
    reader: &ContainedReader,
    project_root: &Path,
    bundle: &ArtifactBundle,
) -> Result<LiveBundleState, CatalogError> {
    let mut snapshots = Vec::with_capacity(bundle.members.len());
    let mut progress = OpenSpecProgress::Unavailable(backstage_core::ProgressFallback {
        parser: backstage_core::ParserProvenance {
            name: "openspec-task-markers".to_owned(),
            version: "1".to_owned(),
        },
        warnings: vec![],
    });
    let mut warnings = Vec::new();
    for member in &bundle.members {
        let snapshot = reader
            .read_snapshot(project_root.join(&member.relative_path))
            .map_err(|error| CatalogError::Read(error.to_string()))?
            .with_relative_path(&member.relative_path)
            .map_err(|error| CatalogError::Read(error.to_string()))?;
        if member.relative_path.ends_with("/tasks.md") || member.relative_path == "tasks.md" {
            if let Some(text) = snapshot.text() {
                progress = parse_openspec_tasks(text);
            } else {
                warnings.push(format!("{} is not UTF-8", member.relative_path));
            }
        }
        snapshots.push(snapshot);
    }
    let fingerprint = fingerprint_complete_snapshots(bundle.members.len(), &snapshots)
        .map_err(|error| CatalogError::Read(error.to_string()))?;
    Ok(LiveBundleState {
        progress,
        fingerprint,
        warnings,
    })
}

fn capture_artifact_detail(
    reader: &ContainedReader,
    project_root: &Path,
    bundle: &ArtifactBundle,
) -> ArtifactDetailCapture {
    let mut progress = unavailable_progress(Vec::new());
    let mut warnings = Vec::new();
    let mut snapshots = std::collections::BTreeMap::new();
    let mut read_failures = std::collections::BTreeMap::new();
    for member in &bundle.members {
        match reader.read_snapshot(project_root.join(&member.relative_path)) {
            Ok(snapshot) => {
                if is_tasks_member(&member.relative_path) {
                    if let Some(text) = snapshot.text() {
                        progress = parse_openspec_tasks(text);
                    } else {
                        let message = format!(
                            "{} is not UTF-8; deterministic progress is unavailable",
                            member.relative_path
                        );
                        warnings.push(message.clone());
                        progress = unavailable_progress(vec![message]);
                    }
                }
                snapshots.insert(member.relative_path.clone(), snapshot);
            }
            Err(error) => {
                let message = format!(
                    "{} could not be read; deterministic detail is partial: {error}",
                    member.relative_path
                );
                if is_tasks_member(&member.relative_path) {
                    progress = unavailable_progress(vec![message.clone()]);
                }
                warnings.push(message.clone());
                read_failures.insert(member.relative_path.clone(), message);
            }
        }
    }
    let ordered = bundle
        .members
        .iter()
        .filter_map(|member| snapshots.get(&member.relative_path).cloned())
        .collect::<Vec<_>>();
    let fingerprint = match fingerprint_complete_snapshots(bundle.members.len(), &ordered) {
        Ok(fingerprint) => Some(fingerprint),
        Err(SnapshotError::IncompleteManifest) => {
            warnings.push(
                "Source fingerprint unavailable because one or more members could not be read"
                    .to_owned(),
            );
            None
        }
        Err(error) => {
            warnings.push(format!("Source fingerprint unavailable: {error}"));
            None
        }
    };
    ArtifactDetailCapture {
        progress,
        fingerprint,
        warnings,
        snapshots,
        read_failures,
    }
}

fn unavailable_progress(messages: Vec<String>) -> OpenSpecProgress {
    OpenSpecProgress::Unavailable(ProgressFallback {
        parser: ParserProvenance {
            name: "openspec-task-markers".to_owned(),
            version: "1".to_owned(),
        },
        warnings: messages
            .into_iter()
            .map(|message| ParseWarning { line: 0, message })
            .collect(),
    })
}

fn is_tasks_member(relative_path: &str) -> bool {
    relative_path == "tasks.md" || relative_path.ends_with("/tasks.md")
}

pub fn markdown_detail(
    reader: &ContainedReader,
    index: &IndexSnapshot,
    document_id: &str,
) -> Result<MarkdownDetail, CatalogError> {
    for project in &index.projects {
        if let Some(document) = project
            .markdown_documents
            .iter()
            .find(|document| document.id == document_id)
        {
            let absolute = Path::new(&project.project.root_path).join(&document.relative_path);
            let snapshot = reader
                .read_snapshot(&absolute)
                .map_err(|error| CatalogError::Read(error.to_string()))?;
            let markdown = snapshot
                .text()
                .ok_or_else(|| CatalogError::Read("Markdown document is not UTF-8".to_owned()))?
                .to_owned();
            return Ok(MarkdownDetail {
                root_id: index.root_id.clone(),
                document_id: document.id.clone(),
                project_id: project.project.id.clone(),
                project_name: project.project.name.clone(),
                project_root: project.project.root_path.clone(),
                git: project.project.git.clone(),
                relative_path: document.relative_path.clone(),
                absolute_path: path_string(&absolute),
                source_modified_unix_nanos: snapshot.observation().modified_unix_nanos,
                markdown,
            });
        }
    }
    Err(CatalogError::NotFound)
}

pub fn artifact_detail(
    reader: &ContainedReader,
    index: &IndexSnapshot,
    artifact_id: &str,
) -> Result<ArtifactDetail, CatalogError> {
    for project in &index.projects {
        for indexed_bundle in &project.bundles {
            if let Some(member) = indexed_bundle
                .bundle
                .members
                .iter()
                .find(|member| member.id == artifact_id)
            {
                let absolute = Path::new(&project.project.root_path).join(&member.relative_path);
                let mut live = capture_artifact_detail(
                    reader,
                    Path::new(&project.project.root_path),
                    &indexed_bundle.bundle,
                );
                let snapshot = live
                    .snapshots
                    .remove(&member.relative_path)
                    .ok_or_else(|| {
                        let reason = live
                            .read_failures
                            .get(&member.relative_path)
                            .map_or("snapshot is unavailable", String::as_str);
                        CatalogError::Read(format!(
                            "Selected artifact {} could not be read: {reason}",
                            member.relative_path
                        ))
                    })?;
                let markdown = snapshot
                    .text()
                    .ok_or_else(|| {
                        CatalogError::Read(format!(
                            "Selected artifact {} is not UTF-8",
                            member.relative_path
                        ))
                    })?
                    .to_owned();
                let open_spec_view = (indexed_bundle.bundle.kind == BundleKind::OpenSpecChange)
                    .then(|| {
                        let mut sources = live
                            .snapshots
                            .iter()
                            .filter_map(|(relative_path, snapshot)| {
                                snapshot.text().map(|markdown| OpenSpecSource {
                                    relative_path: relative_path.clone(),
                                    markdown: markdown.to_owned(),
                                })
                            })
                            .collect::<Vec<_>>();
                        sources.push(OpenSpecSource {
                            relative_path: member.relative_path.clone(),
                            markdown: markdown.clone(),
                        });
                        build_openspec_view(&sources, &live.progress)
                    });
                live.warnings.extend(indexed_bundle.warnings.clone());
                let custody = indexed_bundle.bundle.custody.clone();
                let primary_status = custody
                    .as_ref()
                    .map(|custody| assess_openspec_status(custody, &live.progress));
                return Ok(ArtifactDetail {
                    root_id: index.root_id.clone(),
                    artifact_id: member.id.clone(),
                    bundle_id: indexed_bundle.bundle.id.clone(),
                    project_id: project.project.id.clone(),
                    project_name: project.project.name.clone(),
                    project_root: project.project.root_path.clone(),
                    git: project.project.git.clone(),
                    bundle_name: indexed_bundle.bundle.name.clone(),
                    members: indexed_bundle.bundle.members.clone(),
                    bundle_kind: indexed_bundle.bundle.kind,
                    recognition: indexed_bundle.bundle.recognition.clone(),
                    custody,
                    primary_status,
                    relative_path: member.relative_path.clone(),
                    absolute_path: path_string(&absolute),
                    source_modified_unix_nanos: snapshot.observation().modified_unix_nanos,
                    markdown,
                    progress: live.progress,
                    fingerprint: live
                        .fingerprint
                        .map(|fingerprint| fingerprint.as_str().to_owned()),
                    warnings: live.warnings,
                    open_spec_view,
                });
            }
        }
    }
    Err(CatalogError::NotFound)
}

#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("indexed Markdown ID was not found in the current index")]
    NotFound,
    #[error("artifact could not be read safely: {0}")]
    Read(String),
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub fn bundle_sources(project_root: &Path, bundle: &ArtifactBundle) -> Vec<PathBuf> {
    bundle
        .members
        .iter()
        .map(|member| project_root.join(&member.relative_path))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{Duration, Instant};

    use tempfile::TempDir;

    use super::*;

    fn detected_bundle(
        root: &TempDir,
    ) -> (
        ContainedReader,
        ProjectCandidate,
        ArtifactBundle,
        Vec<MarkdownDocument>,
    ) {
        let change = root.path().join("openspec/changes/bounded");
        fs::create_dir_all(&change).expect("change directory");
        fs::write(change.join("proposal.md"), "# Proposal\n").expect("proposal");
        fs::write(change.join("tasks.md"), "# Tasks\n\n- [ ] Bound work\n").expect("tasks");
        let root_path = fs::canonicalize(root.path()).expect("canonical root");
        let reader = ContainedReader::approve(&root_path, 1024 * 1024).expect("reader");
        let project = ProjectCandidate {
            id: "project_bounded".to_owned(),
            name: "bounded".to_owned(),
            root_path: path_string(&root_path),
            git: None,
        };
        let cancellation = CancellationToken::new();
        let policy = ScanPolicy::default();
        let budget = ProjectScanBudget::new(&policy, &cancellation);
        let detected = detect_project_content(
            &reader,
            &project,
            &root_path,
            &canonical_planning_patterns(),
            &policy,
            &budget,
        );
        let bundle = classify_project(&project.id, &project.name, detected.evidence)
            .into_iter()
            .next()
            .expect("bundle");
        (reader, project, bundle, detected.markdown_documents)
    }

    #[test]
    fn cancellation_between_bundle_members_prevents_the_next_read() {
        let root = TempDir::new().expect("root");
        let (reader, project, bundle, documents) = detected_bundle(&root);
        let cancellation = CancellationToken::new();
        let policy = ScanPolicy::default();
        let budget = ProjectScanBudget::new(&policy, &cancellation);
        let mut members_reached = 0;

        let (indexed, complete) = index_bundle(
            &reader,
            &project,
            bundle,
            &documents,
            &budget,
            &mut |checkpoint| {
                if matches!(checkpoint, CatalogCheckpoint::BeforeMember) {
                    members_reached += 1;
                    if members_reached == 2 {
                        cancellation.cancel();
                        fs::remove_file(root.path().join("openspec/changes/bounded/tasks.md"))
                            .expect("remove unread member");
                    }
                }
            },
        );

        assert!(!complete);
        assert_eq!(members_reached, 2);
        assert!(
            indexed
                .warnings
                .iter()
                .all(|warning| !warning.contains("could not be read"))
        );
    }

    #[test]
    fn stopped_captures_still_compose_every_safely_detected_source() {
        let root = TempDir::new().expect("root");
        let (reader, project, _, _) = detected_bundle(&root);
        let cancellation = CancellationToken::new();
        let policy = ScanPolicy::default();
        let mut cancelled = false;

        let index = build_index_controlled_with_patterns_and_checkpoint(
            &reader,
            vec![project],
            1,
            0,
            "2026-08-13T12:00:00Z".to_owned(),
            vec![],
            &canonical_planning_patterns(),
            &policy,
            &cancellation,
            |checkpoint| {
                if !cancelled && matches!(checkpoint, CatalogCheckpoint::BeforeMember) {
                    cancelled = true;
                    cancellation.cancel();
                }
            },
        );

        assert_eq!(index.projects[0].source_count, 2);
        assert_eq!(index.projects[0].records.len(), 1);
        assert_eq!(index.projects[0].records[0].sources.len(), 2);
        assert!(index.projects[0].records[0].fingerprint.is_none());
        assert!(
            index.projects[0].records[0]
                .warnings
                .iter()
                .any(|warning| warning.code == "openspec_progress_unavailable")
        );
    }

    #[test]
    fn expired_project_deadline_prevents_the_first_bundle_member_read() {
        let root = TempDir::new().expect("root");
        let (reader, project, bundle, documents) = detected_bundle(&root);
        let cancellation = CancellationToken::new();
        let budget = ProjectScanBudget {
            started: Instant::now() - Duration::from_secs(1),
            timeout: Duration::ZERO,
            cancellation: &cancellation,
        };

        let (indexed, complete) = index_bundle(
            &reader,
            &project,
            bundle,
            &documents,
            &budget,
            &mut |checkpoint| {
                if matches!(checkpoint, CatalogCheckpoint::BeforeMember) {
                    fs::remove_file(root.path().join("openspec/changes/bounded/proposal.md"))
                        .expect("remove unread member");
                }
            },
        );

        assert!(!complete);
        assert!(
            indexed
                .warnings
                .iter()
                .all(|warning| !warning.contains("could not be read"))
        );
    }
}
