use std::sync::{Arc, Mutex};

use backstage_app_lib::generation::GenerationSnapshot;
use backstage_app_lib::pi::{CommandOutput, CommandRequest, CommandRunner, PiConfig};
use backstage_app_lib::pi_jobs::{GenerationJobEvent, PiJobRunner};
use backstage_core::{GenerationMode, SourceFingerprint};

struct RecordingRunner {
    calls: Arc<Mutex<Vec<CommandRequest>>>,
    output: Result<CommandOutput, String>,
}

impl CommandRunner for RecordingRunner {
    fn run(&self, request: &CommandRequest) -> Result<CommandOutput, String> {
        self.calls.lock().expect("calls lock").push(request.clone());
        self.output.clone()
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

fn snapshot(fingerprint: &str) -> GenerationSnapshot {
    GenerationSnapshot {
        mode: GenerationMode::Summary,
        prompt_version: "summary-v1".to_owned(),
        included_paths: vec!["tasks.md".to_owned()],
        total_bytes: 12,
        source_fingerprint: SourceFingerprint::from_trusted(fingerprint),
        envelope: "bounded envelope".to_owned(),
    }
}

fn valid_output() -> CommandOutput {
    CommandOutput::success(
        r#"{"type":"message_end","message":{"role":"assistant","provider":"openai-codex","model":"gpt-5.6-sol","stopReason":"stop","content":[{"type":"text","text":"Summary text"}]}}
{"type":"agent_settled"}
"#,
    )
}

#[test]
fn explicit_generation_emits_started_and_completed_with_captured_fingerprint() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let runner = RecordingRunner {
        calls: calls.clone(),
        output: Ok(valid_output()),
    };
    let jobs = PiJobRunner::new(runner, config());

    let events = jobs.run("request-1", snapshot("sha256:a"), "2026-08-13T12:00:00Z");

    assert!(matches!(events[0], GenerationJobEvent::Started { .. }));
    assert!(matches!(events[1], GenerationJobEvent::Completed { .. }));
    assert_eq!(calls.lock().expect("calls lock").len(), 1);
}

#[test]
fn cancellation_prevents_process_invocation() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let jobs = PiJobRunner::new(
        RecordingRunner {
            calls: calls.clone(),
            output: Ok(valid_output()),
        },
        config(),
    );
    jobs.cancel("request-1");

    let events = jobs.run("request-1", snapshot("sha256:a"), "2026-08-13T12:00:00Z");

    assert!(matches!(events[0], GenerationJobEvent::Cancelled { .. }));
    assert!(calls.lock().expect("calls lock").is_empty());
}

#[test]
fn timeout_and_malformed_responses_emit_failure_without_retry() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let jobs = PiJobRunner::new(
        RecordingRunner {
            calls: calls.clone(),
            output: Err("Pi invocation timed out".to_owned()),
        },
        config(),
    );

    let events = jobs.run("request-1", snapshot("sha256:a"), "2026-08-13T12:00:00Z");

    assert!(matches!(events[1], GenerationJobEvent::Failed { .. }));
    assert_eq!(calls.lock().expect("calls lock").len(), 1);
}
