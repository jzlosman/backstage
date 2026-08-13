use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessRequest {
    pub program: String,
    pub arguments: Vec<String>,
    pub working_directory: Option<PathBuf>,
}

pub trait ProcessRunner: Send + Sync {
    fn spawn(&self, request: ProcessRequest) -> Result<(), String>;
}

pub struct SystemProcessRunner;

impl ProcessRunner for SystemProcessRunner {
    fn spawn(&self, request: ProcessRequest) -> Result<(), String> {
        let mut command = Command::new(request.program);
        command.args(request.arguments);
        if let Some(directory) = request.working_directory {
            command.current_dir(directory);
        }
        command
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

pub struct Launcher<'a> {
    runner: &'a dyn ProcessRunner,
}

impl<'a> Launcher<'a> {
    pub fn new(runner: &'a dyn ProcessRunner) -> Self {
        Self { runner }
    }

    pub fn open_terminal(&self, project_root: &Path) -> Result<(), LaunchError> {
        if !project_root.is_absolute() {
            return Err(LaunchError::InvalidProjectPath);
        }
        #[cfg(target_os = "macos")]
        let request = ProcessRequest {
            program: "/usr/bin/open".to_owned(),
            arguments: vec![
                "-a".to_owned(),
                "Terminal".to_owned(),
                path_string(project_root),
            ],
            working_directory: None,
        };
        #[cfg(not(target_os = "macos"))]
        let request = ProcessRequest {
            program: "xdg-open".to_owned(),
            arguments: vec![path_string(project_root)],
            working_directory: None,
        };
        self.runner.spawn(request).map_err(LaunchError::Failed)
    }

    pub fn open_external(&self, target: &str, _project_root: &Path) -> Result<(), LaunchError> {
        Err(LaunchError::UnsupportedTarget {
            target: target.to_owned(),
            alternatives: vec!["copy_path".to_owned(), "copy_prompt".to_owned()],
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, thiserror::Error)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum LaunchError {
    #[error("project path must be absolute")]
    InvalidProjectPath,
    #[error("external target {target} is unsupported")]
    UnsupportedTarget {
        target: String,
        alternatives: Vec<String>,
    },
    #[error("launcher failed: {0}")]
    Failed(String),
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
