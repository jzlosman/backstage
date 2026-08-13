use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use backstage_core::{GeneratedResult, GenerationMode, SourceFingerprint};
use parking_lot::Mutex;
use serde::Serialize;

use crate::generation::GenerationSnapshot;
use crate::pi::{CommandRunner, PiConfig, generation_request, parse_pi_output};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum GenerationJobEvent {
    Started {
        request_id: String,
        source_fingerprint: SourceFingerprint,
    },
    Completed {
        request_id: String,
        result: GeneratedResult,
    },
    Failed {
        request_id: String,
        source_fingerprint: SourceFingerprint,
        failure: String,
    },
    Cancelled {
        request_id: String,
    },
}

pub struct PiJobRunner<R> {
    runner: R,
    config: PiConfig,
    cancelled: Arc<Mutex<HashSet<String>>>,
    active: Arc<Mutex<std::collections::HashMap<String, Arc<AtomicBool>>>>,
}

impl<R: CommandRunner> PiJobRunner<R> {
    pub fn new(runner: R, config: PiConfig) -> Self {
        Self {
            runner,
            config,
            cancelled: Arc::new(Mutex::new(HashSet::new())),
            active: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub fn cancel(&self, request_id: &str) {
        self.cancelled.lock().insert(request_id.to_owned());
        self.cancellation_flag(request_id)
            .store(true, Ordering::Release);
    }

    pub fn cancellation_flag(&self, request_id: &str) -> Arc<AtomicBool> {
        self.active
            .lock()
            .entry(request_id.to_owned())
            .or_insert_with(|| Arc::new(AtomicBool::new(false)))
            .clone()
    }

    pub fn run(
        &self,
        request_id: &str,
        snapshot: GenerationSnapshot,
        generated_at: impl Into<String>,
    ) -> Vec<GenerationJobEvent> {
        if self.cancelled.lock().contains(request_id) {
            return vec![GenerationJobEvent::Cancelled {
                request_id: request_id.to_owned(),
            }];
        }
        let cancellation = self.cancellation_flag(request_id);
        let started = GenerationJobEvent::Started {
            request_id: request_id.to_owned(),
            source_fingerprint: snapshot.source_fingerprint.clone(),
        };
        let mut request = generation_request(&self.config, snapshot.envelope.clone());
        request.cancelled = cancellation;
        let event = match self.runner.run(&request) {
            Ok(output) => {
                match parse_pi_output(&output, &self.config.provider, &self.config.model, None) {
                    Ok(text) => GenerationJobEvent::Completed {
                        request_id: request_id.to_owned(),
                        result: GeneratedResult {
                            text,
                            mode: snapshot.mode,
                            source_fingerprint: snapshot.source_fingerprint,
                            included_paths: snapshot.included_paths,
                            generated_at: generated_at.into(),
                            model: Some(format!("{}/{}", self.config.provider, self.config.model)),
                            prompt_version: snapshot.prompt_version,
                        },
                    },
                    Err(failure) => GenerationJobEvent::Failed {
                        request_id: request_id.to_owned(),
                        source_fingerprint: snapshot.source_fingerprint,
                        failure,
                    },
                }
            }
            Err(failure) => GenerationJobEvent::Failed {
                request_id: request_id.to_owned(),
                source_fingerprint: snapshot.source_fingerprint,
                failure,
            },
        };
        self.active.lock().remove(request_id);
        vec![started, event]
    }
}

pub fn mode_key(mode: GenerationMode) -> &'static str {
    match mode {
        GenerationMode::Summary => "summary",
    }
}
