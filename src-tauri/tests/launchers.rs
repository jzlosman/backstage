use std::path::Path;
use std::sync::Mutex;

use backstage_app_lib::launcher::{LaunchError, Launcher, ProcessRequest, ProcessRunner};

#[derive(Default)]
struct RecordingRunner {
    requests: Mutex<Vec<ProcessRequest>>,
}

impl ProcessRunner for RecordingRunner {
    fn spawn(&self, request: ProcessRequest) -> Result<(), String> {
        self.requests.lock().expect("request lock").push(request);
        Ok(())
    }
}

#[test]
fn macos_terminal_launch_only_opens_the_project_directory() {
    let runner = RecordingRunner::default();
    let launcher = Launcher::new(&runner);

    launcher
        .open_terminal(Path::new("/Users/dev/workbench"))
        .expect("terminal request");

    let requests = runner.requests.lock().expect("request lock");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].program, "/usr/bin/open");
    assert_eq!(
        requests[0].arguments,
        ["-a", "Terminal", "/Users/dev/workbench"]
    );
    assert!(requests[0].working_directory.is_none());
    assert!(
        !requests[0]
            .arguments
            .iter()
            .any(|argument| argument.contains("git "))
    );
}

#[test]
fn unsupported_external_target_offers_copy_alternatives() {
    let runner = RecordingRunner::default();
    let launcher = Launcher::new(&runner);

    let error = launcher
        .open_external("superset", Path::new("/Users/dev/workbench"))
        .expect_err("unsupported target");

    assert_eq!(
        error,
        LaunchError::UnsupportedTarget {
            target: "superset".to_owned(),
            alternatives: vec!["copy_path".to_owned(), "copy_prompt".to_owned()],
        }
    );
    assert!(runner.requests.lock().expect("request lock").is_empty());
}
