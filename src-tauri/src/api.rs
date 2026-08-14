use std::path::Path;

use backstage_core::{ApprovedRoot, HandoffContext, continuation_prompt};
use serde::Serialize;

use crate::catalog::{ArtifactDetail, artifact_detail, markdown_detail};
use crate::discovery::{CancellationToken, DiscoveryResult, ScanPolicy, discover_projects};
use crate::filesystem::ContainedReader;
use crate::index::IndexSnapshot;
use crate::storage::SqliteStore;

pub fn approve_root_path(
    store: &SqliteStore,
    path: impl AsRef<Path>,
) -> Result<ApprovedRoot, ApiError> {
    let reader = ContainedReader::approve(path, 2 * 1024 * 1024).map_err(ApiError::from_error)?;
    let root = ApprovedRoot::new(reader.root(), true).map_err(ApiError::from_error)?;
    store.upsert_root(&root).map_err(ApiError::from_error)?;
    Ok(root)
}

pub fn list_approved_roots(store: &SqliteStore) -> Result<Vec<ApprovedRoot>, ApiError> {
    store.list_roots().map_err(ApiError::from_error)
}

pub fn remove_approved_root(store: &SqliteStore, root_id: &str) -> Result<(), ApiError> {
    store.remove_root(root_id).map_err(ApiError::from_error)
}

pub fn derive_artifact_path(
    reader: &ContainedReader,
    index: &IndexSnapshot,
    artifact_id: &str,
) -> Result<String, ApiError> {
    Ok(detail(reader, index, artifact_id)?.absolute_path)
}

pub fn derive_markdown_path(
    reader: &ContainedReader,
    index: &IndexSnapshot,
    document_id: &str,
) -> Result<String, ApiError> {
    let detail = markdown_detail(reader, index, document_id).map_err(ApiError::from_error)?;
    Ok(reader
        .resolve_file(&detail.absolute_path)
        .map_err(ApiError::from_error)?
        .to_string_lossy()
        .into_owned())
}

pub fn derive_continuation_prompt(
    reader: &ContainedReader,
    index: &IndexSnapshot,
    artifact_id: &str,
) -> Result<String, ApiError> {
    let detail = detail(reader, index, artifact_id)?;
    Ok(continuation_prompt(&HandoffContext {
        project_path: detail.project_root,
        project_name: detail.project_name,
        bundle_name: detail.bundle_name,
        artifact_path: detail.absolute_path,
        bundle_kind: detail.bundle_kind,
        recognition: detail.recognition,
        custody: detail.custody,
        progress: detail.progress,
        warnings: detail.warnings,
    }))
}

fn detail(
    reader: &ContainedReader,
    index: &IndexSnapshot,
    artifact_id: &str,
) -> Result<ArtifactDetail, ApiError> {
    artifact_detail(reader, index, artifact_id).map_err(ApiError::from_error)
}

pub fn scan_approved_root(store: &SqliteStore, root_id: &str) -> Result<DiscoveryResult, ApiError> {
    let root = store
        .list_roots()
        .map_err(ApiError::from_error)?
        .into_iter()
        .find(|root| root.id() == root_id)
        .ok_or_else(|| ApiError::new("root_not_found", "Approved root is no longer available"))?;
    let policy = ScanPolicy::default();
    let reader = ContainedReader::approve(root.path(), policy.max_file_bytes)
        .map_err(ApiError::from_error)?;
    Ok(discover_projects(
        &reader,
        &policy,
        &CancellationToken::new(),
    ))
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    pub code: &'static str,
    pub message: String,
}

impl ApiError {
    pub fn from_error(error: impl std::error::Error) -> Self {
        Self {
            code: "operation_failed",
            message: error.to_string(),
        }
    }

    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
