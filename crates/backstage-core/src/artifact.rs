use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};

use crate::OpenSpecCustody;

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
    PlanningPatternMatch,
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
    #[serde(with = "crate::optional_u128_decimal_string")]
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactBundle {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub name: String,
    pub kind: BundleKind,
    pub recognition: ArtifactRecognition,
    pub members: Vec<ArtifactMember>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custody: Option<OpenSpecCustody>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ArtifactBundleData {
    id: String,
    project_id: String,
    project_name: String,
    name: String,
    kind: BundleKind,
    recognition: ArtifactRecognition,
    members: Vec<ArtifactMember>,
    #[serde(default)]
    custody: Option<OpenSpecCustody>,
}

impl<'de> Deserialize<'de> for ArtifactBundle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = ArtifactBundleData::deserialize(deserializer)?;
        let custody = data.custody.or_else(|| {
            (data.kind == BundleKind::OpenSpecChange).then_some(OpenSpecCustody::Current)
        });
        Ok(Self {
            id: data.id,
            project_id: data.project_id,
            project_name: data.project_name,
            name: data.name,
            kind: data.kind,
            recognition: data.recognition,
            members: data.members,
            custody,
        })
    }
}

pub fn classify_project(
    project_id: impl Into<String>,
    project_name: impl Into<String>,
    evidence: Vec<DetectorEvidence>,
) -> Vec<ArtifactBundle> {
    let project_id = project_id.into();
    let project_name = project_name.into();
    let mut openspec_groups: BTreeMap<String, (OpenSpecLocation, Vec<DetectorEvidence>)> =
        BTreeMap::new();
    let mut openspec_paths = BTreeSet::new();
    let mut candidates: BTreeMap<String, Vec<DetectorEvidence>> = BTreeMap::new();

    for item in evidence {
        match item.kind {
            EvidenceKind::OpenSpecMember => {
                if let Some(location) = openspec_location(&item.relative_path) {
                    openspec_paths.insert(item.relative_path.clone());
                    openspec_groups
                        .entry(location.directory.clone())
                        .or_insert_with(|| (location, Vec::new()))
                        .1
                        .push(item);
                }
            }
            EvidenceKind::PlanningPatternMatch => candidates
                .entry(item.relative_path.clone())
                .or_default()
                .push(item),
            EvidenceKind::OrdinaryMarkdown => {}
        }
    }

    let mut bundles = Vec::new();
    for (_, (location, mut members)) in openspec_groups {
        members.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        bundles.push(ArtifactBundle {
            id: stable_id(
                "bundle",
                &format!("{project_id}:openspec:{}", location.identity),
            ),
            project_id: project_id.clone(),
            project_name: project_name.clone(),
            name: location.display_name,
            kind: BundleKind::OpenSpecChange,
            recognition: ArtifactRecognition::Recognized {
                detector: "openspec-v1".to_owned(),
            },
            members: members
                .into_iter()
                .map(|evidence| member(&project_id, evidence))
                .collect(),
            custody: Some(location.custody),
        });
    }

    for (relative_path, mut matching_evidence) in candidates {
        if openspec_paths.contains(&relative_path) {
            continue;
        }
        matching_evidence.sort_by(|left, right| {
            left.reason
                .cmp(&right.reason)
                .then_with(|| left.relative_path.cmp(&right.relative_path))
        });
        let evidence = matching_evidence
            .into_iter()
            .next()
            .expect("candidate groups are non-empty");
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
            custody: None,
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

#[derive(Clone)]
struct OpenSpecLocation {
    directory: String,
    identity: String,
    display_name: String,
    custody: OpenSpecCustody,
}

pub fn is_supported_openspec_member(path: &str) -> bool {
    openspec_location(path).is_some()
}

fn openspec_location(path: &str) -> Option<OpenSpecLocation> {
    if path.starts_with('/') || path.contains('\\') {
        return None;
    }
    let parts = path.split('/').collect::<Vec<_>>();
    if parts
        .iter()
        .any(|part| part.is_empty() || matches!(*part, "." | ".."))
        || parts.first() != Some(&"openspec")
        || parts.get(1) != Some(&"changes")
    {
        return None;
    }

    if parts.get(2) == Some(&"archive") {
        let folder = *parts.get(3)?;
        if !is_supported_member(&parts, 4) {
            return None;
        }
        let (display_name, archived_on) = match valid_archive_prefix(folder) {
            Some((date, name)) => (name, Some(date)),
            None => (folder.to_owned(), None),
        };
        let directory = parts[..4].join("/");
        Some(OpenSpecLocation {
            identity: directory.clone(),
            directory,
            display_name,
            custody: OpenSpecCustody::Archived { archived_on },
        })
    } else {
        let change = *parts.get(2)?;
        if !is_supported_member(&parts, 3) {
            return None;
        }
        Some(OpenSpecLocation {
            directory: parts[..3].join("/"),
            identity: change.to_owned(),
            display_name: change.to_owned(),
            custody: OpenSpecCustody::Current,
        })
    }
}

fn is_supported_member(parts: &[&str], member_start: usize) -> bool {
    let member = &parts[member_start..];
    matches!(member, ["proposal.md"] | ["design.md"] | ["tasks.md"])
        || (member.len() >= 3
            && member.first() == Some(&"specs")
            && member.last() == Some(&"spec.md"))
}

fn valid_archive_prefix(folder: &str) -> Option<(String, String)> {
    let bytes = folder.as_bytes();
    if bytes.len() <= 11
        || bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || bytes.get(10) != Some(&b'-')
        || !bytes[..4].iter().all(u8::is_ascii_digit)
        || !bytes[5..7].iter().all(u8::is_ascii_digit)
        || !bytes[8..10].iter().all(u8::is_ascii_digit)
    {
        return None;
    }
    let year = folder[..4].parse::<u32>().ok()?;
    let month = folder[5..7].parse::<u32>().ok()?;
    let day = folder[8..10].parse::<u32>().ok()?;
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 400 == 0 || (year % 4 == 0 && year % 100 != 0) => 29,
        2 => 28,
        _ => return None,
    };
    if day == 0 || day > max_day {
        return None;
    }
    Some((folder[..10].to_owned(), folder[11..].to_owned()))
}

fn normalize_relative(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches("./").to_owned()
}

fn artifact_id(project_id: &str, relative_path: &str) -> String {
    stable_id("artifact", &format!("{project_id}:{relative_path}"))
}

fn stable_id(prefix: &str, value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    format!("{prefix}_{:x}", digest)[..prefix.len() + 1 + 24].to_owned()
}
