use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use backstage_core::{SnapshotError, SourceObservation, SourceSnapshot};
use cap_std::ambient_authority;
use cap_std::fs::Dir;

#[derive(Debug)]
pub struct ContainedReader {
    approved_root: PathBuf,
    canonical_root: PathBuf,
    root_dir: Dir,
    max_file_bytes: u64,
}

impl Clone for ContainedReader {
    fn clone(&self) -> Self {
        Self {
            approved_root: self.approved_root.clone(),
            canonical_root: self.canonical_root.clone(),
            root_dir: self
                .root_dir
                .try_clone()
                .expect("approved root handle should clone"),
            max_file_bytes: self.max_file_bytes,
        }
    }
}

impl ContainedReader {
    pub fn approve(root: impl AsRef<Path>, max_file_bytes: u64) -> Result<Self, ReadError> {
        let root = root.as_ref();
        if !root.is_absolute() {
            return Err(ReadError::PathMustBeAbsolute);
        }
        let approved_root = root.to_path_buf();
        let canonical_root = fs::canonicalize(root).map_err(|source| ReadError::Unavailable {
            path: root.to_path_buf(),
            source,
        })?;
        let metadata = fs::metadata(&canonical_root).map_err(|source| ReadError::Unavailable {
            path: canonical_root.clone(),
            source,
        })?;
        if !metadata.is_dir() {
            return Err(ReadError::RootMustBeDirectory {
                path: canonical_root,
            });
        }
        let root_dir =
            Dir::open_ambient_dir(&canonical_root, ambient_authority()).map_err(|source| {
                ReadError::Unavailable {
                    path: canonical_root.clone(),
                    source,
                }
            })?;
        Ok(Self {
            approved_root,
            canonical_root,
            root_dir,
            max_file_bytes,
        })
    }

    pub fn root(&self) -> &Path {
        &self.canonical_root
    }

    pub fn entry_type(
        &self,
        requested: impl AsRef<Path>,
    ) -> Result<cap_std::fs::FileType, ReadError> {
        let (relative, display) = self.relative_path_allow_root(requested.as_ref())?;
        self.root_dir
            .symlink_metadata(relative)
            .map(|metadata| metadata.file_type())
            .map_err(|source| map_capability_error(display, source))
    }

    pub fn walk(
        &self,
        max_depth: usize,
        max_entries: usize,
        excluded: &std::collections::HashSet<String>,
        cancelled: impl Fn() -> bool,
    ) -> Vec<Result<(PathBuf, cap_std::fs::FileType), ReadError>> {
        self.walk_from(
            &self.canonical_root,
            max_depth,
            max_entries,
            excluded,
            cancelled,
        )
    }

    pub fn walk_from(
        &self,
        requested: impl AsRef<Path>,
        max_depth: usize,
        max_entries: usize,
        excluded: &std::collections::HashSet<String>,
        cancelled: impl Fn() -> bool,
    ) -> Vec<Result<(PathBuf, cap_std::fs::FileType), ReadError>> {
        let requested = requested.as_ref();
        let (relative_start, display) = match self.relative_path_allow_root(requested) {
            Ok(paths) => paths,
            Err(error) => return vec![Err(error)],
        };
        let starting_dir = if relative_start.as_os_str().is_empty() {
            self.root_dir.try_clone().expect("root clones")
        } else {
            match self.root_dir.open_dir(&relative_start) {
                Ok(directory) => directory,
                Err(source) => {
                    return vec![Err(ReadError::Unavailable {
                        path: display,
                        source,
                    })];
                }
            }
        };
        let mut results = Vec::new();
        let mut pending = vec![(relative_start, starting_dir, 0)];
        while let Some((relative_dir, directory, depth)) = pending.pop() {
            if cancelled() || results.len() >= max_entries {
                break;
            }
            let entries = match directory.entries() {
                Ok(entries) => entries,
                Err(source) => {
                    results.push(Err(ReadError::Unavailable {
                        path: self.canonical_root.join(&relative_dir),
                        source,
                    }));
                    continue;
                }
            };
            for entry in entries {
                if cancelled() || results.len() >= max_entries {
                    break;
                }
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(source) => {
                        results.push(Err(ReadError::Unavailable {
                            path: self.canonical_root.join(&relative_dir),
                            source,
                        }));
                        continue;
                    }
                };
                let name = entry.file_name();
                let relative = relative_dir.join(&name);
                let file_type = match entry.file_type() {
                    Ok(file_type) => file_type,
                    Err(source) => {
                        results.push(Err(ReadError::Unavailable {
                            path: self.canonical_root.join(&relative),
                            source,
                        }));
                        continue;
                    }
                };
                results.push(Ok((self.canonical_root.join(&relative), file_type)));
                if file_type.is_dir()
                    && depth < max_depth
                    && !excluded.contains(&name.to_string_lossy().to_string())
                    && let Ok(child) = entry.open_dir()
                {
                    pending.push((relative, child, depth + 1));
                }
            }
        }
        results
    }

    pub fn regular_file_modified_unix_nanos(
        &self,
        requested: impl AsRef<Path>,
    ) -> Result<Option<u128>, ReadError> {
        let (relative, display) = self.relative_path(requested.as_ref())?;
        let metadata = self
            .root_dir
            .symlink_metadata(relative)
            .map_err(|source| map_capability_error(display.clone(), source))?;
        if !metadata.is_file() {
            return Err(ReadError::NotAFile { path: display });
        }
        Ok(metadata
            .modified()
            .ok()
            .and_then(|modified| {
                modified
                    .into_std()
                    .duration_since(std::time::UNIX_EPOCH)
                    .ok()
            })
            .map(|duration| duration.as_nanos()))
    }

    pub fn resolve_file(&self, requested: impl AsRef<Path>) -> Result<PathBuf, ReadError> {
        let (relative, display) = self.relative_path(requested.as_ref())?;
        let resolved = fs::canonicalize(&display).map_err(|source| ReadError::Unavailable {
            path: display.clone(),
            source,
        })?;
        if !resolved.starts_with(&self.canonical_root) {
            return Err(ReadError::OutsideApprovedRoot {
                path: display,
                resolved,
            });
        }
        let metadata =
            self.root_dir
                .metadata(&relative)
                .map_err(|source| ReadError::Unavailable {
                    path: display.clone(),
                    source,
                })?;
        validate_metadata(&display, &metadata, self.max_file_bytes)?;
        Ok(resolved)
    }

    pub fn read_bytes(&self, requested: impl AsRef<Path>) -> Result<Vec<u8>, ReadError> {
        let requested = requested.as_ref();
        let (relative, display) = self.relative_path(requested)?;
        let file = self
            .root_dir
            .open(&relative)
            .map_err(|source| map_capability_error(display.clone(), source))?;
        let metadata = file.metadata().map_err(|source| ReadError::Unavailable {
            path: display.clone(),
            source,
        })?;
        validate_metadata(&display, &metadata, self.max_file_bytes)?;
        read_bounded(&file, &display, self.max_file_bytes)
    }

    pub fn read_text(&self, requested: impl AsRef<Path>) -> Result<String, ReadError> {
        let requested = requested.as_ref();
        let bytes = self.read_bytes(requested)?;
        String::from_utf8(bytes).map_err(|source| ReadError::NotUtf8 {
            path: requested.to_path_buf(),
            source,
        })
    }

    pub fn read_snapshot(&self, requested: impl AsRef<Path>) -> Result<SourceSnapshot, ReadError> {
        let requested = requested.as_ref();
        let (relative, display) = self.relative_path(requested)?;
        let file = self
            .root_dir
            .open(&relative)
            .map_err(|source| map_capability_error(display.clone(), source))?;
        let before = observation(&file.metadata().map_err(|source| ReadError::Unavailable {
            path: display.clone(),
            source,
        })?);
        if before.byte_len > self.max_file_bytes {
            return Err(ReadError::FileTooLarge {
                path: display,
                actual: before.byte_len,
                limit: self.max_file_bytes,
            });
        }
        let bytes = read_bounded(&file, &display, self.max_file_bytes)?;
        let after = observation(&file.metadata().map_err(|source| ReadError::Unavailable {
            path: display.clone(),
            source,
        })?);
        SourceSnapshot::from_observations(relative, bytes, before, after).map_err(ReadError::from)
    }

    fn relative_path(&self, requested: &Path) -> Result<(PathBuf, PathBuf), ReadError> {
        let (relative, display) = self.relative_path_allow_root(requested)?;
        if relative.as_os_str().is_empty() {
            return Err(ReadError::OutsideApprovedRoot {
                path: requested.to_path_buf(),
                resolved: requested.to_path_buf(),
            });
        }
        Ok((relative, display))
    }

    fn relative_path_allow_root(&self, requested: &Path) -> Result<(PathBuf, PathBuf), ReadError> {
        if !requested.is_absolute() {
            return Err(ReadError::PathMustBeAbsolute);
        }
        let relative = requested
            .strip_prefix(&self.canonical_root)
            .or_else(|_| requested.strip_prefix(&self.approved_root))
            .map_err(|_| ReadError::OutsideApprovedRoot {
                path: requested.to_path_buf(),
                resolved: requested.to_path_buf(),
            })?;
        if relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            return Err(ReadError::OutsideApprovedRoot {
                path: requested.to_path_buf(),
                resolved: requested.to_path_buf(),
            });
        }
        Ok((relative.to_path_buf(), requested.to_path_buf()))
    }
}

fn validate_metadata(
    path: &Path,
    metadata: &cap_std::fs::Metadata,
    limit: u64,
) -> Result<(), ReadError> {
    if !metadata.is_file() {
        return Err(ReadError::NotAFile {
            path: path.to_path_buf(),
        });
    }
    if metadata.len() > limit {
        return Err(ReadError::FileTooLarge {
            path: path.to_path_buf(),
            actual: metadata.len(),
            limit,
        });
    }
    Ok(())
}

fn read_bounded(file: impl std::io::Read, path: &Path, limit: u64) -> Result<Vec<u8>, ReadError> {
    use std::io::Read;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|source| ReadError::Unavailable {
            path: path.to_path_buf(),
            source,
        })?;
    if bytes.len() as u64 > limit {
        return Err(ReadError::FileTooLarge {
            path: path.to_path_buf(),
            actual: bytes.len() as u64,
            limit,
        });
    }
    Ok(bytes)
}

fn map_capability_error(path: PathBuf, source: io::Error) -> ReadError {
    if source.kind() == io::ErrorKind::PermissionDenied {
        ReadError::OutsideApprovedRoot {
            path: path.clone(),
            resolved: path,
        }
    } else {
        ReadError::Unavailable { path, source }
    }
}

fn observation(metadata: &cap_std::fs::Metadata) -> SourceObservation {
    SourceObservation {
        byte_len: metadata.len(),
        modified_unix_nanos: metadata
            .modified()
            .ok()
            .and_then(|modified| {
                modified
                    .into_std()
                    .duration_since(std::time::UNIX_EPOCH)
                    .ok()
            })
            .map(|duration| duration.as_nanos()),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    #[error("path must be absolute")]
    PathMustBeAbsolute,
    #[error("approved root is not a directory: {path}")]
    RootMustBeDirectory { path: PathBuf },
    #[error("path {path} resolves outside the approved root to {resolved}")]
    OutsideApprovedRoot { path: PathBuf, resolved: PathBuf },
    #[error("path is not a regular file: {path}")]
    NotAFile { path: PathBuf },
    #[error("file {path} is {actual} bytes, above the {limit} byte limit")]
    FileTooLarge {
        path: PathBuf,
        actual: u64,
        limit: u64,
    },
    #[error("path is unavailable: {path}: {source}")]
    Unavailable { path: PathBuf, source: io::Error },
    #[error("file is not UTF-8: {path}: {source}")]
    NotUtf8 {
        path: PathBuf,
        source: std::string::FromUtf8Error,
    },
    #[error("could not create a stable source snapshot: {0}")]
    Snapshot(#[from] SnapshotError),
}
