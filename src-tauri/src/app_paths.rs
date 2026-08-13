use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppPaths {
    config_dir: PathBuf,
    cache_dir: PathBuf,
    data_dir: PathBuf,
}

impl AppPaths {
    pub fn system() -> Result<Self, AppPathError> {
        let dirs = ProjectDirs::from("works", "Earendil", "Backstage")
            .ok_or(AppPathError::HomeDirectoryUnavailable)?;
        Ok(Self {
            config_dir: dirs.config_dir().to_path_buf(),
            cache_dir: dirs.cache_dir().to_path_buf(),
            data_dir: dirs.data_dir().to_path_buf(),
        })
    }

    pub fn under(base: impl AsRef<Path>) -> Self {
        let base = base.as_ref();
        Self {
            config_dir: base.join("config"),
            cache_dir: base.join("cache"),
            data_dir: base.join("data"),
        }
    }

    pub fn ensure_exists(&self) -> Result<(), AppPathError> {
        for directory in [&self.config_dir, &self.cache_dir, &self.data_dir] {
            fs::create_dir_all(directory).map_err(|source| AppPathError::Create {
                path: directory.clone(),
                source,
            })?;
        }
        Ok(())
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn database_path(&self) -> PathBuf {
        self.data_dir.join("backstage.sqlite3")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppPathError {
    #[error("the operating system did not provide an application data directory")]
    HomeDirectoryUnavailable,
    #[error("could not create app-owned directory {path}: {source}")]
    Create { path: PathBuf, source: io::Error },
}
