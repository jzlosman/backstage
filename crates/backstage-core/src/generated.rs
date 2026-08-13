use serde::{Deserialize, Serialize};

use crate::SourceFingerprint;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationMode {
    Summary,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedResult {
    pub text: String,
    pub mode: GenerationMode,
    pub source_fingerprint: SourceFingerprint,
    pub included_paths: Vec<String>,
    pub generated_at: String,
    pub model: Option<String>,
    pub prompt_version: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum GeneratedView {
    NeverGenerated,
    Generating {
        request_id: String,
        requested_fingerprint: SourceFingerprint,
        previous: Option<GeneratedResult>,
    },
    Current {
        result: GeneratedResult,
    },
    Stale {
        result: GeneratedResult,
        changed_inputs: Vec<String>,
    },
    Failed {
        previous: Option<GeneratedResult>,
        failure: String,
    },
}

pub fn start_generation(
    state: GeneratedView,
    request_id: impl Into<String>,
    requested_fingerprint: SourceFingerprint,
) -> GeneratedView {
    GeneratedView::Generating {
        request_id: request_id.into(),
        requested_fingerprint,
        previous: previous_result(&state),
    }
}

pub fn generation_completed(
    state: GeneratedView,
    request_id: &str,
    result: GeneratedResult,
    current_fingerprint: &SourceFingerprint,
) -> GeneratedView {
    let GeneratedView::Generating {
        request_id: active_request,
        requested_fingerprint,
        ..
    } = &state
    else {
        return state;
    };
    if active_request != request_id || requested_fingerprint != &result.source_fingerprint {
        return state;
    }
    if &result.source_fingerprint == current_fingerprint {
        GeneratedView::Current { result }
    } else {
        GeneratedView::Stale {
            result,
            changed_inputs: Vec::new(),
        }
    }
}

pub fn generation_failed(
    state: GeneratedView,
    request_id: &str,
    failure: impl Into<String>,
) -> GeneratedView {
    let GeneratedView::Generating {
        request_id: active_request,
        requested_fingerprint,
        previous,
    } = state
    else {
        return state;
    };
    if active_request != request_id {
        return GeneratedView::Generating {
            request_id: active_request,
            requested_fingerprint,
            previous,
        };
    }
    GeneratedView::Failed {
        previous,
        failure: failure.into(),
    }
}

pub fn sources_changed(
    state: GeneratedView,
    current_fingerprint: &SourceFingerprint,
    changed_inputs: Vec<String>,
) -> GeneratedView {
    match state {
        GeneratedView::Current { result } if &result.source_fingerprint != current_fingerprint => {
            GeneratedView::Stale {
                result,
                changed_inputs,
            }
        }
        GeneratedView::Generating {
            request_id,
            requested_fingerprint,
            previous,
        } => GeneratedView::Generating {
            request_id,
            requested_fingerprint,
            previous,
        },
        other => other,
    }
}

pub fn previous_result(state: &GeneratedView) -> Option<GeneratedResult> {
    match state {
        GeneratedView::Current { result } | GeneratedView::Stale { result, .. } => {
            Some(result.clone())
        }
        GeneratedView::Generating { previous, .. } | GeneratedView::Failed { previous, .. } => {
            previous.clone()
        }
        GeneratedView::NeverGenerated => None,
    }
}
