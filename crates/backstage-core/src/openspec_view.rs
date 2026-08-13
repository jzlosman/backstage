use serde::{Deserialize, Serialize};

use crate::markdown_syntax::FenceTracker;
use crate::{OpenSpecProgress, TaskFact};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenSpecSource {
    pub relative_path: String,
    pub markdown: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenSpecView {
    pub overview: Vec<OpenSpecOverviewSection>,
    pub task_groups: Vec<OpenSpecTaskGroup>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenSpecOverviewSection {
    pub kind: OpenSpecOverviewKind,
    pub source_path: String,
    pub markdown: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenSpecOverviewKind {
    Why,
    WhatChanges,
    GoalsAndNonGoals,
    Decisions,
    RisksAndTradeOffs,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenSpecTaskGroup {
    pub title: String,
    pub source_path: String,
    pub tasks: Vec<TaskFact>,
}

pub fn build_openspec_view(
    sources: &[OpenSpecSource],
    progress: &OpenSpecProgress,
) -> OpenSpecView {
    let mut overview = Vec::new();
    for source in sources {
        let file_name = source.relative_path.rsplit('/').next().unwrap_or_default();
        let allowed = match file_name {
            "proposal.md" => &[OpenSpecOverviewKind::Why, OpenSpecOverviewKind::WhatChanges][..],
            "design.md" => &[
                OpenSpecOverviewKind::GoalsAndNonGoals,
                OpenSpecOverviewKind::Decisions,
                OpenSpecOverviewKind::RisksAndTradeOffs,
            ][..],
            _ => continue,
        };
        overview.extend(extract_sections(source, allowed));
    }

    overview.sort_by_key(|section| overview_rank(section.kind));

    let task_groups = match progress {
        OpenSpecProgress::Available(progress) => sources
            .iter()
            .find(|source| source.relative_path.rsplit('/').next() == Some("tasks.md"))
            .map(|source| group_tasks(source, &progress.tasks))
            .unwrap_or_default(),
        OpenSpecProgress::Unavailable(_) => Vec::new(),
    };

    OpenSpecView {
        overview,
        task_groups,
    }
}

fn extract_sections(
    source: &OpenSpecSource,
    allowed: &[OpenSpecOverviewKind],
) -> Vec<OpenSpecOverviewSection> {
    let lines = source.markdown.lines().collect::<Vec<_>>();
    let headings = markdown_headings(&source.markdown)
        .into_iter()
        .filter(|heading| heading.level == 2)
        .collect::<Vec<_>>();
    let mut sections = Vec::new();
    for (index, heading) in headings.iter().enumerate() {
        let Some(kind) = overview_kind(&heading.title) else {
            continue;
        };
        if !allowed.contains(&kind) {
            continue;
        }
        let end = headings
            .get(index + 1)
            .map_or(lines.len(), |next| next.line - 1);
        let markdown = lines[heading.line..end].join("\n").trim().to_owned();
        if !markdown.is_empty() {
            sections.push(OpenSpecOverviewSection {
                kind,
                source_path: source.relative_path.clone(),
                markdown,
            });
        }
    }
    sections
}

fn group_tasks(source: &OpenSpecSource, tasks: &[TaskFact]) -> Vec<OpenSpecTaskGroup> {
    let headings = markdown_headings(&source.markdown)
        .into_iter()
        .filter(|heading| heading.level >= 2)
        .collect::<Vec<_>>();
    let mut groups: Vec<OpenSpecTaskGroup> = Vec::new();
    for task in tasks {
        let title = headings
            .iter()
            .rev()
            .find(|heading| heading.line < task.location.line)
            .map_or_else(|| "Other tasks".to_owned(), |heading| heading.title.clone());
        if groups.last().is_none_or(|group| group.title != title) {
            groups.push(OpenSpecTaskGroup {
                title,
                source_path: source.relative_path.clone(),
                tasks: Vec::new(),
            });
        }
        groups
            .last_mut()
            .expect("task group exists")
            .tasks
            .push(task.clone());
    }
    groups
}

#[derive(Debug)]
struct MarkdownHeading {
    line: usize,
    level: usize,
    title: String,
}

fn markdown_headings(markdown: &str) -> Vec<MarkdownHeading> {
    let mut headings = Vec::new();
    let mut fence = FenceTracker::default();
    for (index, line) in markdown.lines().enumerate() {
        let trimmed = line.trim_start();
        if fence.consume(trimmed) {
            continue;
        }
        if fence.is_open() {
            continue;
        }
        let level = trimmed.bytes().take_while(|byte| *byte == b'#').count();
        if !(1..=6).contains(&level) || trimmed.as_bytes().get(level) != Some(&b' ') {
            continue;
        }
        let title = trimmed[level + 1..]
            .trim()
            .trim_end_matches('#')
            .trim()
            .to_owned();
        if !title.is_empty() {
            headings.push(MarkdownHeading {
                line: index + 1,
                level,
                title,
            });
        }
    }
    headings
}

fn overview_rank(kind: OpenSpecOverviewKind) -> usize {
    match kind {
        OpenSpecOverviewKind::Why => 0,
        OpenSpecOverviewKind::WhatChanges => 1,
        OpenSpecOverviewKind::GoalsAndNonGoals => 2,
        OpenSpecOverviewKind::Decisions => 3,
        OpenSpecOverviewKind::RisksAndTradeOffs => 4,
    }
}

fn overview_kind(title: &str) -> Option<OpenSpecOverviewKind> {
    match normalized_heading(title).as_str() {
        "why" => Some(OpenSpecOverviewKind::Why),
        "what changes" => Some(OpenSpecOverviewKind::WhatChanges),
        "goals and non goals" | "goals non goals" => Some(OpenSpecOverviewKind::GoalsAndNonGoals),
        "decisions" => Some(OpenSpecOverviewKind::Decisions),
        "risks and trade offs" | "risks trade offs" | "risks and tradeoffs" => {
            Some(OpenSpecOverviewKind::RisksAndTradeOffs)
        }
        _ => None,
    }
}

fn normalized_heading(title: &str) -> String {
    let expanded = title
        .to_lowercase()
        .replace(['/', '&'], " and ")
        .replace(['-', '_'], " ");
    expanded
        .split(|character: char| !character.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
