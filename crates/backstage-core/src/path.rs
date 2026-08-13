use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApprovedRoot {
    id: String,
    path: String,
}

impl ApprovedRoot {
    pub fn new(path: impl AsRef<Path>, is_directory: bool) -> Result<Self, DomainError> {
        let path = path.as_ref();
        if !path.is_absolute() {
            return Err(DomainError::RootMustBeAbsolute);
        }
        if !is_directory {
            return Err(DomainError::RootMustBeDirectory);
        }
        let path = normalize_absolute(path)?;
        let path = path_to_string(&path);
        Ok(Self {
            id: stable_id("root", &path),
            path,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArtifactPath {
    id: String,
    root_id: String,
    absolute: String,
    relative: String,
}

impl ArtifactPath {
    pub fn new(
        root: &ApprovedRoot,
        path: impl AsRef<Path>,
        is_file: bool,
    ) -> Result<Self, DomainError> {
        let path = path.as_ref();
        if !path.is_absolute() {
            return Err(DomainError::ArtifactPathMustBeAbsolute);
        }
        if !is_file {
            return Err(DomainError::ArtifactMustBeFile);
        }

        let normalized = normalize_absolute(path)?;
        let root_path = Path::new(root.path());
        let relative = normalized
            .strip_prefix(root_path)
            .map_err(|_| DomainError::OutsideApprovedRoot)?;
        if relative.as_os_str().is_empty() {
            return Err(DomainError::ArtifactMustBeFile);
        }

        let absolute = path_to_string(&normalized);
        let relative = path_to_string(relative);
        Ok(Self {
            id: stable_id("artifact", &format!("{}:{relative}", root.id())),
            root_id: root.id().to_owned(),
            absolute,
            relative,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn root_id(&self) -> &str {
        &self.root_id
    }

    pub fn absolute(&self) -> &str {
        &self.absolute
    }

    pub fn relative(&self) -> &str {
        &self.relative
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, thiserror::Error)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum DomainError {
    #[error("approved root must be absolute")]
    RootMustBeAbsolute,
    #[error("approved root must be a directory")]
    RootMustBeDirectory,
    #[error("artifact path must be absolute")]
    ArtifactPathMustBeAbsolute,
    #[error("artifact must be a file")]
    ArtifactMustBeFile,
    #[error("path is outside every approved root")]
    OutsideApprovedRoot,
    #[error("path normalization attempted to escape its filesystem root")]
    PathNormalizationEscape,
    #[error("artifact is unavailable")]
    ArtifactUnavailable,
    #[error("explanation scope is too large")]
    ExplanationScopeTooLarge,
}

fn normalize_absolute(path: &Path) -> Result<PathBuf, DomainError> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new("/")),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(DomainError::PathNormalizationEscape);
                }
            }
            Component::Normal(segment) => normalized.push(segment),
        }
    }
    Ok(normalized)
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn stable_id(prefix: &str, value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    format!("{prefix}_{:x}", digest)[..prefix.len() + 1 + 24].to_owned()
}
