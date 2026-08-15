use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceObservation {
    pub byte_len: u64,
    pub modified_unix_nanos: Option<u128>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSnapshot {
    relative_path: String,
    content: Vec<u8>,
    content_digest: String,
    observation: SourceObservation,
}

impl SourceSnapshot {
    pub fn from_observations(
        relative_path: impl AsRef<Path>,
        content: Vec<u8>,
        before: SourceObservation,
        after: SourceObservation,
    ) -> Result<Self, SnapshotError> {
        if before != after || before.byte_len != content.len() as u64 {
            return Err(SnapshotError::SourceChangedDuringRead);
        }
        let relative_path = normalize_relative(relative_path.as_ref())?;
        let content_digest = format!("sha256:{:x}", Sha256::digest(&content));
        Ok(Self {
            relative_path,
            content,
            content_digest,
            observation: after,
        })
    }

    pub fn relative_path(&self) -> &str {
        &self.relative_path
    }

    pub fn with_relative_path(
        mut self,
        relative_path: impl AsRef<Path>,
    ) -> Result<Self, SnapshotError> {
        self.relative_path = normalize_relative(relative_path.as_ref())?;
        Ok(self)
    }

    pub fn content(&self) -> &[u8] {
        &self.content
    }

    pub fn text(&self) -> Option<&str> {
        std::str::from_utf8(&self.content).ok()
    }

    pub fn content_digest(&self) -> &str {
        &self.content_digest
    }

    pub fn observation(&self) -> SourceObservation {
        self.observation
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SourceFingerprint(String);

impl SourceFingerprint {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn from_trusted(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SnapshotError {
    #[error("snapshot path must be a normalized relative path")]
    InvalidRelativePath,
    #[error("source changed while it was being read")]
    SourceChangedDuringRead,
    #[error("source manifest is incomplete")]
    IncompleteManifest,
}

pub fn fingerprint_complete_snapshots(
    expected_members: usize,
    snapshots: &[SourceSnapshot],
) -> Result<SourceFingerprint, SnapshotError> {
    if expected_members == 0 || snapshots.len() != expected_members {
        return Err(SnapshotError::IncompleteManifest);
    }
    Ok(fingerprint_snapshots(snapshots))
}

pub fn fingerprint_snapshots(snapshots: &[SourceSnapshot]) -> SourceFingerprint {
    let mut ordered = snapshots.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    let mut hasher = Sha256::new();
    hasher.update(b"backstage-bundle-fingerprint-v1\0");
    hasher.update((ordered.len() as u64).to_be_bytes());
    for snapshot in ordered {
        hasher.update((snapshot.relative_path.len() as u64).to_be_bytes());
        hasher.update(snapshot.relative_path.as_bytes());
        hasher.update((snapshot.content.len() as u64).to_be_bytes());
        hasher.update(snapshot.content_digest.as_bytes());
    }
    SourceFingerprint(format!("sha256:{:x}", hasher.finalize()))
}

fn normalize_relative(path: &Path) -> Result<String, SnapshotError> {
    if path.is_absolute() {
        return Err(SnapshotError::InvalidRelativePath);
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(segment) => normalized.push(segment),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(SnapshotError::InvalidRelativePath);
            }
        }
    }
    if normalized.as_os_str().is_empty() {
        return Err(SnapshotError::InvalidRelativePath);
    }
    Ok(normalized.to_string_lossy().replace('\\', "/"))
}
