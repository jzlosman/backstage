use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use backstage_app_lib::pi::{
    CommandOutput, CommandRequest, CommandRunner, PiCapability, PiConfig, SystemCommandRunner,
    probe_pi,
};

struct FixtureRunner {
    version: CommandOutput,
    probe: CommandOutput,
}

impl CommandRunner for FixtureRunner {
    fn run(&self, request: &CommandRequest) -> Result<CommandOutput, String> {
        if request.arguments == ["--version"] {
            return Ok(self.version.clone());
        }
        assert!(!request.arguments.iter().any(|argument| argument == "bash"));
        assert!(
            request
                .arguments
                .iter()
                .any(|argument| argument == "--no-tools")
        );
        assert!(
            request
                .arguments
                .iter()
                .any(|argument| argument == "--no-extensions")
        );
        assert!(
            request
                .arguments
                .iter()
                .any(|argument| argument == "--no-context-files")
        );
        assert!(request.current_dir.starts_with("/tmp/backstage-owned"));
        Ok(self.probe.clone())
    }
}

fn config() -> PiConfig {
    PiConfig {
        executable: "/usr/local/bin/pi".into(),
        required_version: "0.82.1".to_owned(),
        provider: "openai-codex".to_owned(),
        model: "gpt-5.6-sol".to_owned(),
        app_temp_dir: "/tmp/backstage-owned".into(),
        timeout_ms: 60_000,
        max_output_bytes: 1024 * 1024,
    }
}

#[test]
fn capability_probe_accepts_a_tool_free_settled_json_run() {
    let runner = FixtureRunner {
        version: CommandOutput::success("0.82.1\n"),
        probe: CommandOutput::success(
            r#"{"type":"message_end","message":{"role":"assistant","provider":"openai-codex","model":"gpt-5.6-sol","stopReason":"stop","content":[{"type":"text","text":"BACKSTAGE_PI_CAPABILITY_V1:nonce"}]}}
{"type":"agent_settled"}
"#,
        ),
    };

    let capability = probe_pi(&runner, &config(), "nonce");

    assert!(matches!(capability, PiCapability::Available { .. }));
}

#[test]
fn capability_probe_disables_generation_on_version_tool_retry_or_contract_failure() {
    let bad_version = FixtureRunner {
        version: CommandOutput::success("0.83.0\n"),
        probe: CommandOutput::success(""),
    };
    assert!(matches!(
        probe_pi(&bad_version, &config(), "nonce"),
        PiCapability::Unavailable { .. }
    ));

    let tool_event = FixtureRunner {
        version: CommandOutput::success("0.82.1\n"),
        probe: CommandOutput::success(
            r#"{"type":"tool_execution_start"}
{"type":"agent_settled"}
"#,
        ),
    };
    assert!(matches!(
        probe_pi(&tool_event, &config(), "nonce"),
        PiCapability::Unavailable { .. }
    ));
}

#[cfg(target_os = "macos")]
#[test]
fn system_runner_os_sandbox_allows_only_app_owned_writes() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let allowed = temp.path().join("allowed");
    let denied = temp.path().join("denied");
    std::fs::create_dir_all(&allowed).expect("allowed directory");
    std::fs::create_dir_all(&denied).expect("denied directory");
    let request = CommandRequest {
        program: "/bin/sh".into(),
        arguments: vec![
            "-c".to_owned(),
            "echo allowed > \"$1/output\"; echo denied > \"$2/output\"".to_owned(),
            "sh".to_owned(),
            allowed.to_string_lossy().into_owned(),
            denied.to_string_lossy().into_owned(),
        ],
        current_dir: allowed.clone(),
        environment: Default::default(),
        stdin: vec![],
        timeout: Duration::from_secs(5),
        max_output_bytes: 16 * 1024,
        sandbox_write_root: Some(allowed.clone()),
        cancelled: Arc::new(AtomicBool::new(false)),
    };

    let output = SystemCommandRunner
        .run(&request)
        .expect("sandbox process runs");

    assert_ne!(output.exit_code, Some(0));
    assert!(allowed.join("output").is_file());
    assert!(!denied.join("output").exists());
}

#[test]
fn installed_config_resolves_pi_or_keeps_the_command_fallback() {
    let config = PiConfig::installed("/tmp/backstage-owned".into());

    if config.executable.is_absolute() {
        assert!(config.executable.is_file());
    } else {
        assert_eq!(config.executable, std::path::Path::new("pi"));
    }
}
