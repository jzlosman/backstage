use serde::{Deserialize, Serialize};

use crate::markdown_syntax::FenceTracker;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", content = "progress", rename_all = "snake_case")]
pub enum OpenSpecProgress {
    Available(TaskProgress),
    Unavailable(ProgressFallback),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskProgress {
    pub total: usize,
    pub completed: usize,
    pub remaining_count: usize,
    pub tasks: Vec<TaskFact>,
    pub remaining: Vec<TaskFact>,
    pub parser: ParserProvenance,
    pub warnings: Vec<ParseWarning>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressFallback {
    pub parser: ParserProvenance,
    pub warnings: Vec<ParseWarning>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskFact {
    pub text: String,
    pub completed: bool,
    pub location: SourceLocation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ParserProvenance {
    pub name: String,
    pub version: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ParseWarning {
    pub line: usize,
    pub message: String,
}

pub fn parse_openspec_tasks(source: &str) -> OpenSpecProgress {
    let parser = ParserProvenance {
        name: "openspec-task-markers".to_owned(),
        version: "1".to_owned(),
    };
    let mut tasks = Vec::new();
    let mut warnings = Vec::new();
    let mut fence = FenceTracker::default();

    for (index, line) in source.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim_start();
        if fence.consume(trimmed) {
            continue;
        }
        if fence.is_open() {
            continue;
        }

        let indent = line.len() - trimmed.len();
        let marker = if trimmed.starts_with("- [") || trimmed.starts_with("* [") {
            Some(trimmed)
        } else {
            None
        };
        let Some(marker) = marker else { continue };

        let state = marker.as_bytes().get(3).copied();
        let close = marker.as_bytes().get(4).copied();
        let separator = marker.as_bytes().get(5).copied();
        if close != Some(b']') || separator != Some(b' ') {
            warnings.push(ParseWarning {
                line: line_number,
                message: "unsupported task marker syntax".to_owned(),
            });
            continue;
        }

        let completed = match state {
            Some(b' ') => false,
            Some(b'x' | b'X') => true,
            _ => {
                warnings.push(ParseWarning {
                    line: line_number,
                    message: "unsupported task marker state".to_owned(),
                });
                continue;
            }
        };
        let text = marker[6..].trim().to_owned();
        if text.is_empty() {
            warnings.push(ParseWarning {
                line: line_number,
                message: "task marker has no task text".to_owned(),
            });
            continue;
        }
        tasks.push(TaskFact {
            text,
            completed,
            location: SourceLocation {
                line: line_number,
                column: indent + 3,
            },
        });
    }

    if tasks.is_empty() {
        return OpenSpecProgress::Unavailable(ProgressFallback { parser, warnings });
    }

    let completed = tasks.iter().filter(|task| task.completed).count();
    let remaining = tasks
        .iter()
        .filter(|task| !task.completed)
        .cloned()
        .collect::<Vec<_>>();
    OpenSpecProgress::Available(TaskProgress {
        total: tasks.len(),
        completed,
        remaining_count: remaining.len(),
        tasks,
        remaining,
        parser,
        warnings,
    })
}
