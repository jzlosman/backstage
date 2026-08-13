use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::filesystem::ContainedReader;

#[derive(Clone, Debug, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ScanPolicy {
    pub exclusions: Vec<String>,
    pub max_depth: usize,
    pub max_entries: usize,
    pub max_file_bytes: u64,
    pub timeout_ms: u64,
}

impl Default for ScanPolicy {
    fn default() -> Self {
        Self {
            exclusions: vec![
                ".git".to_owned(),
                "node_modules".to_owned(),
                "target".to_owned(),
                ".cache".to_owned(),
                ".next".to_owned(),
                "dist".to_owned(),
                "build".to_owned(),
            ],
            max_depth: 8,
            max_entries: 50_000,
            max_file_bytes: 2 * 1024 * 1024,
            timeout_ms: 30_000,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CancellationToken(pub(crate) Arc<AtomicBool>);

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCandidate {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub git: Option<GitContext>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitContext {
    pub branch: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanWarning {
    pub code: String,
    pub path: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryResult {
    pub projects: Vec<ProjectCandidate>,
    pub warnings: Vec<ScanWarning>,
    pub cancelled: bool,
    pub entries_inspected: usize,
}

pub fn discover_projects(
    reader: &ContainedReader,
    policy: &ScanPolicy,
    cancellation: &CancellationToken,
) -> DiscoveryResult {
    let started = Instant::now();
    let timeout = Duration::from_millis(policy.timeout_ms);
    let excluded = policy.exclusions.iter().cloned().collect::<HashSet<_>>();
    let root = reader.root();
    let mut result = DiscoveryResult::default();
    let mut seen = HashSet::new();
    if cancellation.is_cancelled() {
        result.cancelled = true;
        return result;
    }

    let walker = reader.walk(policy.max_depth, policy.max_entries, &excluded, || {
        cancellation.is_cancelled() || started.elapsed() >= timeout
    });

    for entry in walker {
        if cancellation.is_cancelled() {
            result.cancelled = true;
            break;
        }
        if started.elapsed() >= timeout {
            result.warnings.push(warning(
                "scan_timeout",
                root,
                format!("Scan stopped after {} ms", policy.timeout_ms),
            ));
            break;
        }
        if result.entries_inspected >= policy.max_entries {
            result.warnings.push(warning(
                "entry_limit_reached",
                root,
                format!("Scan stopped after {} entries", policy.max_entries),
            ));
            break;
        }
        result.entries_inspected += 1;

        let (path, file_type) = match entry {
            Ok(entry) => entry,
            Err(error) => {
                result.warnings.push(ScanWarning {
                    code: "path_unreadable".to_owned(),
                    path: path_string(root),
                    message: error.to_string(),
                });
                continue;
            }
        };

        if file_type.is_symlink() {
            match reader.resolve_file(&path) {
                Err(crate::filesystem::ReadError::OutsideApprovedRoot { resolved, .. }) => {
                    result.warnings.push(warning(
                        "symlink_escape",
                        &path,
                        format!(
                            "Skipped symlink resolving outside approved root: {}",
                            resolved.display()
                        ),
                    ))
                }
                Err(error) => {
                    result
                        .warnings
                        .push(warning("symlink_unavailable", &path, error.to_string()))
                }
                _ => {}
            }
            continue;
        }

        if file_type.is_file() {
            continue;
        }

        if file_type.is_dir()
            && reader
                .entry_type(path.join(".git"))
                .is_ok_and(|kind| kind.is_dir())
        {
            add_project(reader, &path, &mut result, &mut seen);
        }
    }

    if result.projects.is_empty() && !result.cancelled {
        add_project(reader, root, &mut result, &mut seen);
    }

    result.projects.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.root_path.cmp(&right.root_path))
    });
    result
}

fn add_project(
    reader: &ContainedReader,
    path: &Path,
    result: &mut DiscoveryResult,
    seen: &mut HashSet<String>,
) {
    let root_path = path_string(path);
    if !seen.insert(root_path.clone()) {
        return;
    }
    let git = inspect_git(reader, path);
    if let Err(message) = &git {
        result
            .warnings
            .push(warning("git_unavailable", path, message));
    }
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(&root_path)
        .to_owned();
    result.projects.push(ProjectCandidate {
        id: stable_id("project", &root_path),
        name,
        root_path,
        git: git.ok(),
    });
}

fn inspect_git(reader: &ContainedReader, root: &Path) -> Result<GitContext, String> {
    let dot_git = root.join(".git");
    let metadata = reader
        .entry_type(&dot_git)
        .map_err(|error| error.to_string())?;
    if !metadata.is_dir() {
        return Err("Git worktree pointers are not inspected in v1".to_owned());
    }
    let head = reader
        .read_text(dot_git.join("HEAD"))
        .map_err(|error| error.to_string())?;
    let branch = head
        .trim()
        .strip_prefix("ref: refs/heads/")
        .unwrap_or_else(|| head.trim())
        .to_owned();
    if branch.is_empty() {
        return Err("Git HEAD is empty".to_owned());
    }
    Ok(GitContext { branch })
}

fn warning(code: &str, path: &Path, message: impl Into<String>) -> ScanWarning {
    ScanWarning {
        code: code.to_owned(),
        path: path_string(path),
        message: message.into(),
    }
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn stable_id(prefix: &str, value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    format!("{prefix}_{:x}", digest)[..prefix.len() + 1 + 24].to_owned()
}
