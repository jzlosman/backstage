use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectorEvidence {
    pub relative_path: String,
    pub kind: EvidenceKind,
    pub reason: String,
}

impl DetectorEvidence {
    pub fn new(
        relative_path: impl Into<String>,
        kind: EvidenceKind,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            relative_path: normalize_relative(&relative_path.into()),
            kind,
            reason: reason.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    OpenSpecMember,
    CandidateName,
    OrdinaryMarkdown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BundleKind {
    OpenSpecChange,
    PossibleArtifact,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ArtifactRecognition {
    Recognized { detector: String },
    Possible { reason: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownDocument {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub relative_path: String,
    pub source_modified_unix_nanos: Option<u128>,
}

impl MarkdownDocument {
    pub fn new(
        project_id: impl Into<String>,
        project_name: impl Into<String>,
        relative_path: impl Into<String>,
        source_modified_unix_nanos: Option<u128>,
    ) -> Self {
        let project_id = project_id.into();
        let relative_path = normalize_relative(&relative_path.into());
        Self {
            id: artifact_id(&project_id, &relative_path),
            project_id,
            project_name: project_name.into(),
            relative_path,
            source_modified_unix_nanos,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactMember {
    pub id: String,
    pub relative_path: String,
    pub evidence: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactBundle {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub name: String,
    pub kind: BundleKind,
    pub recognition: ArtifactRecognition,
    pub members: Vec<ArtifactMember>,
}

pub fn classify_project(
    project_id: impl Into<String>,
    project_name: impl Into<String>,
    evidence: Vec<DetectorEvidence>,
) -> Vec<ArtifactBundle> {
    let project_id = project_id.into();
    let project_name = project_name.into();
    let mut openspec_groups: BTreeMap<String, Vec<DetectorEvidence>> = BTreeMap::new();
    let mut candidates = Vec::new();

    for item in evidence {
        match item.kind {
            EvidenceKind::OpenSpecMember => {
                if let Some(change) = openspec_change_name(&item.relative_path) {
                    openspec_groups.entry(change).or_default().push(item);
                }
            }
            EvidenceKind::CandidateName => candidates.push(item),
            EvidenceKind::OrdinaryMarkdown => {}
        }
    }

    let mut bundles = Vec::new();
    for (change_name, mut members) in openspec_groups {
        members.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        bundles.push(ArtifactBundle {
            id: stable_id("bundle", &format!("{project_id}:openspec:{change_name}")),
            project_id: project_id.clone(),
            project_name: project_name.clone(),
            name: change_name,
            kind: BundleKind::OpenSpecChange,
            recognition: ArtifactRecognition::Recognized {
                detector: "openspec-v1".to_owned(),
            },
            members: members
                .into_iter()
                .map(|evidence| member(&project_id, evidence))
                .collect(),
        });
    }

    candidates.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    for evidence in candidates {
        let reason = evidence.reason.clone();
        bundles.push(ArtifactBundle {
            id: stable_id(
                "bundle",
                &format!("{project_id}:candidate:{}", evidence.relative_path),
            ),
            project_id: project_id.clone(),
            project_name: project_name.clone(),
            name: evidence
                .relative_path
                .rsplit('/')
                .next()
                .unwrap_or(&evidence.relative_path)
                .to_owned(),
            kind: BundleKind::PossibleArtifact,
            recognition: ArtifactRecognition::Possible { reason },
            members: vec![member(&project_id, evidence)],
        });
    }

    bundles.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
    bundles
}

fn member(project_id: &str, evidence: DetectorEvidence) -> ArtifactMember {
    ArtifactMember {
        id: artifact_id(project_id, &evidence.relative_path),
        relative_path: evidence.relative_path,
        evidence: evidence.reason,
    }
}

fn openspec_change_name(path: &str) -> Option<String> {
    let parts = path.split('/').collect::<Vec<_>>();
    (parts.len() >= 4 && parts[0] == "openspec" && parts[1] == "changes")
        .then(|| parts[2].to_owned())
}

fn normalize_relative(path: &str) -> String {
    path.replace('\\', "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_owned()
}

fn artifact_id(project_id: &str, relative_path: &str) -> String {
    stable_id("artifact", &format!("{project_id}:{relative_path}"))
}

fn stable_id(prefix: &str, value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    format!("{prefix}_{:x}", digest)[..prefix.len() + 1 + 24].to_owned()
}
