use std::path::{Path, PathBuf};

use backstage_core::{
    ArtifactBundle, ArtifactRecognition, BundleKind, DetectorEvidence, EvidenceKind,
    MarkdownDocument, OpenSpecProgress, OpenSpecSource, OpenSpecView, SnapshotError,
    build_openspec_view, classify_project, fingerprint_complete_snapshots, parse_openspec_tasks,
};
use serde::Serialize;

use crate::discovery::{CancellationToken, ProjectCandidate, ScanPolicy, ScanWarning};
use crate::filesystem::ContainedReader;
use crate::index::{IndexSnapshot, IndexedBundle, IndexedProject};

const CANDIDATE_NAMES: &[&str] = &[
    "PLAN.md",
    "plan.md",
    "TDD.md",
    "tdd.md",
    "ROADMAP.md",
    "roadmap.md",
];

pub fn build_index(
    reader: &ContainedReader,
    projects: Vec<ProjectCandidate>,
    generation: u64,
    indexed_at: String,
    warnings: Vec<ScanWarning>,
) -> IndexSnapshot {
    build_index_controlled(
        reader,
        projects,
        generation,
        indexed_at,
        warnings,
        &ScanPolicy::default(),
        &CancellationToken::new(),
    )
}

pub fn build_index_controlled(
    reader: &ContainedReader,
    projects: Vec<ProjectCandidate>,
    generation: u64,
    indexed_at: String,
    mut warnings: Vec<ScanWarning>,
    policy: &ScanPolicy,
    cancellation: &CancellationToken,
) -> IndexSnapshot {
    let indexed_projects = projects
        .into_iter()
        .take_while(|_| !cancellation.is_cancelled())
        .map(|project| index_project(reader, project, policy, cancellation))
        .collect::<Vec<_>>();
    if cancellation.is_cancelled() {
        warnings.push(ScanWarning {
            code: "scan_cancelled".to_owned(),
            path: path_string(reader.root()),
            message: "Artifact classification was cancelled".to_owned(),
        });
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
        root_id: backstage_core::ApprovedRoot::new(reader.root(), true)
            .expect("contained reader root is a canonical directory")
            .id()
            .to_owned(),
        generation,
        indexed_at,
        projects: indexed_projects,
        warnings,
    }
}

fn index_project(
    reader: &ContainedReader,
    project: ProjectCandidate,
    policy: &ScanPolicy,
    cancellation: &CancellationToken,
) -> IndexedProject {
    let detected = detect_project_content(
        reader,
        &project,
        Path::new(&project.root_path),
        policy,
        cancellation,
    );
    let bundles = classify_project(&project.id, &project.name, detected.evidence)
        .into_iter()
        .map(|bundle| index_bundle(reader, &project, bundle))
        .collect();
    IndexedProject {
        project,
        bundles,
        markdown_documents: detected.markdown_documents,
    }
}

struct DetectedProjectContent {
    evidence: Vec<DetectorEvidence>,
    markdown_documents: Vec<MarkdownDocument>,
}

fn detect_project_content(
    reader: &ContainedReader,
    project: &ProjectCandidate,
    project_root: &Path,
    policy: &ScanPolicy,
    cancellation: &CancellationToken,
) -> DetectedProjectContent {
    let mut evidence = Vec::new();
    let mut markdown_documents = Vec::new();
    let started = std::time::Instant::now();
    let excluded = policy.exclusions.iter().cloned().collect();
    for entry in reader.walk_from(
        project_root,
        policy.max_depth,
        policy.max_entries,
        &excluded,
        || {
            cancellation.is_cancelled()
                || started.elapsed() >= std::time::Duration::from_millis(policy.timeout_ms)
        },
    ) {
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
        let Ok(source_modified_unix_nanos) = reader.regular_file_modified_unix_nanos(&path) else {
            continue;
        };
        markdown_documents.push(MarkdownDocument::new(
            &project.id,
            &project.name,
            &relative,
            source_modified_unix_nanos,
        ));
        if is_openspec_member(&relative) {
            evidence.push(DetectorEvidence::new(
                relative,
                EvidenceKind::OpenSpecMember,
                "Path is supported OpenSpec change material",
            ));
        } else if relative
            .rsplit('/')
            .next()
            .is_some_and(|name| CANDIDATE_NAMES.contains(&name))
        {
            evidence.push(DetectorEvidence::new(
                relative.clone(),
                EvidenceKind::CandidateName,
                format!(
                    "Filename matches configured planning candidate {}",
                    relative.rsplit('/').next().unwrap_or(&relative)
                ),
            ));
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
    }
}

fn is_markdown(relative: &str) -> bool {
    Path::new(relative)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
}

fn is_openspec_member(relative: &str) -> bool {
    let parts = relative.split('/').collect::<Vec<_>>();
    if parts.len() < 4 || parts[0] != "openspec" || parts[1] != "changes" {
        return false;
    }
    matches!(parts[3], "proposal.md" | "design.md" | "tasks.md")
        || (parts[3] == "specs" && parts.last() == Some(&"spec.md"))
}

fn index_bundle(
    reader: &ContainedReader,
    project: &ProjectCandidate,
    bundle: ArtifactBundle,
) -> IndexedBundle {
    let mut snapshots = Vec::new();
    let mut warnings = Vec::new();
    let mut progress = OpenSpecProgress::Unavailable(backstage_core::ProgressFallback {
        parser: backstage_core::ParserProvenance {
            name: "openspec-task-markers".to_owned(),
            version: "1".to_owned(),
        },
        warnings: vec![],
    });

    for member in &bundle.members {
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

    let source_modified_unix_nanos = snapshots
        .iter()
        .filter_map(|snapshot| snapshot.observation().modified_unix_nanos)
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

    IndexedBundle {
        bundle,
        progress,
        fingerprint,
        source_modified_unix_nanos,
        warnings,
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
    pub relative_path: String,
    pub absolute_path: String,
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
    snapshots: std::collections::BTreeMap<String, backstage_core::SourceSnapshot>,
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
    let snapshots = bundle
        .members
        .iter()
        .zip(snapshots)
        .map(|(member, snapshot)| (member.relative_path.clone(), snapshot))
        .collect();
    Ok(LiveBundleState {
        progress,
        fingerprint,
        warnings,
        snapshots,
    })
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
                let mut live = live_bundle_state(
                    reader,
                    Path::new(&project.project.root_path),
                    &indexed_bundle.bundle,
                )?;
                let open_spec_view = (indexed_bundle.bundle.kind == BundleKind::OpenSpecChange)
                    .then(|| {
                        let sources = live
                            .snapshots
                            .iter()
                            .filter_map(|(relative_path, snapshot)| {
                                snapshot.text().map(|markdown| OpenSpecSource {
                                    relative_path: relative_path.clone(),
                                    markdown: markdown.to_owned(),
                                })
                            })
                            .collect::<Vec<_>>();
                        build_openspec_view(&sources, &live.progress)
                    });
                let snapshot = live
                    .snapshots
                    .remove(&member.relative_path)
                    .ok_or_else(|| {
                        CatalogError::Read("Artifact snapshot is unavailable".to_owned())
                    })?;
                let markdown = snapshot
                    .text()
                    .ok_or_else(|| CatalogError::Read("Artifact is not UTF-8".to_owned()))?
                    .to_owned();
                live.warnings.extend(indexed_bundle.warnings.clone());
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
                    relative_path: member.relative_path.clone(),
                    absolute_path: path_string(&absolute),
                    source_modified_unix_nanos: snapshot.observation().modified_unix_nanos,
                    markdown,
                    progress: live.progress,
                    fingerprint: Some(live.fingerprint.as_str().to_owned()),
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
