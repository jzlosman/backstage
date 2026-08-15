use std::collections::{BTreeMap, BTreeSet};

use crate::artifact::{OpenSpecLocation, openspec_location};
use crate::format_adapters::generic_source_detail;
use crate::{
    AdapterDescriptor, AdapterFailure, AdapterHandoff, AdapterSummary, Capability, CapabilityView,
    DetectedRecord, FactProvenance, FactValue, OpenSpecCustody, OpenSpecOverviewKind,
    OpenSpecPrimaryStatus, OpenSpecProgress, OpenSpecSource, ParserProvenance,
    PlanningFormatAdapter, ProgressFallback, ProjectSourceInventory, RecognitionLevel,
    RecordSourceCapture, SourceClaim, SourceReference, StructuredBlock, StructuredItem,
    SummaryFact, WorkRecordWarning, assess_openspec_status, build_openspec_view,
    fingerprint_complete_snapshots, parse_openspec_tasks,
};

const OPENSPEC_PRECEDENCE: u16 = 10;
const ADAPTER_ID: &str = "openspec-v1";

pub struct OpenSpecAdapter {
    descriptor: AdapterDescriptor,
}

impl OpenSpecAdapter {
    pub fn new() -> Self {
        Self {
            descriptor: AdapterDescriptor::new(ADAPTER_ID, "openspec", 1, OPENSPEC_PRECEDENCE),
        }
    }
}

impl Default for OpenSpecAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl PlanningFormatAdapter for OpenSpecAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    fn detect(
        &self,
        inventory: &ProjectSourceInventory,
    ) -> Result<Vec<DetectedRecord>, AdapterFailure> {
        let mut groups: BTreeMap<String, (OpenSpecLocation, Vec<SourceClaim>)> = BTreeMap::new();
        for source in &inventory.sources {
            let Some(location) = openspec_location(&source.relative_path) else {
                continue;
            };
            groups
                .entry(location.directory.clone())
                .or_insert_with(|| (location, vec![]))
                .1
                .push(SourceClaim::new(source.relative_path.clone()));
        }

        Ok(groups
            .into_values()
            .map(|(location, claims)| {
                DetectedRecord::new(
                    location.directory,
                    location.display_name,
                    RecognitionLevel::Recognized,
                    claims,
                    vec!["Path is supported OpenSpec change material".to_owned()],
                    openspec_capabilities(),
                )
            })
            .collect())
    }

    fn summarize(
        &self,
        record: &DetectedRecord,
        capture: &RecordSourceCapture,
    ) -> Result<AdapterSummary, AdapterFailure> {
        let location = record_location(record)?;
        let (progress, mut warnings) = progress_from_capture(record, capture);
        let status = assess_openspec_status(&location.custody, &progress);
        let mut facts = vec![
            fact(
                "openspec.custody",
                "Custody",
                FactValue::Text(custody_value(&location.custody).to_owned()),
                vec![],
            ),
            fact(
                "openspec.primary_status",
                "Status",
                FactValue::Text(status_value(status).to_owned()),
                task_source_paths(record),
            ),
            fact(
                "work_record.source_count",
                "Sources",
                FactValue::Count(record.claims.len() as u64),
                record
                    .claims
                    .iter()
                    .map(|claim| claim.relative_path.clone())
                    .collect(),
            ),
        ];
        if let OpenSpecCustody::Archived {
            archived_on: Some(date),
        } = &location.custody
        {
            facts.push(fact(
                "openspec.archived_on",
                "Archived on",
                FactValue::Date(date.clone()),
                vec![],
            ));
        }
        if let OpenSpecProgress::Available(progress) = &progress {
            let provenance = task_source_paths(record);
            facts.extend([
                fact(
                    "openspec.task.total_count",
                    "Tasks",
                    FactValue::Count(progress.total as u64),
                    provenance.clone(),
                ),
                fact(
                    "openspec.task.done_count",
                    "Done",
                    FactValue::Count(progress.completed as u64),
                    provenance.clone(),
                ),
                fact(
                    "openspec.task.open_count",
                    "Open",
                    FactValue::Count(progress.remaining_count as u64),
                    provenance,
                ),
            ]);
        }

        let captured_paths = capture
            .snapshots
            .iter()
            .map(|snapshot| snapshot.relative_path())
            .collect::<BTreeSet<_>>();
        let complete = capture.failures.is_empty()
            && capture.snapshots.len() == record.claims.len()
            && record
                .claims
                .iter()
                .all(|claim| captured_paths.contains(claim.relative_path.as_str()));
        let fingerprint = if complete {
            fingerprint_complete_snapshots(record.claims.len(), &capture.snapshots).ok()
        } else {
            warnings.push(WorkRecordWarning::without_source(
                "incomplete_source_snapshot",
                "OpenSpec fingerprint is unavailable because the captured record is incomplete",
            ));
            None
        };

        Ok(AdapterSummary::new(
            facts,
            warnings,
            openspec_capabilities(),
            fingerprint,
        ))
    }

    fn build_detail(
        &self,
        record: &DetectedRecord,
        capture: &RecordSourceCapture,
    ) -> Result<Vec<CapabilityView>, AdapterFailure> {
        record_location(record)?;
        let (progress, progress_warnings) = progress_from_capture(record, capture);
        let sources = capture
            .snapshots
            .iter()
            .filter_map(|snapshot| {
                snapshot.text().map(|markdown| OpenSpecSource {
                    relative_path: snapshot.relative_path().to_owned(),
                    markdown: markdown.to_owned(),
                })
            })
            .collect::<Vec<_>>();
        let legacy_view = build_openspec_view(&sources, &progress);

        let mut overview_blocks = legacy_view
            .overview
            .into_iter()
            .enumerate()
            .map(|(index, section)| {
                StructuredBlock::markdown_section(
                    format!("overview-{index}"),
                    overview_label(section.kind),
                    section.markdown,
                    SourceReference::new(section.source_path, None),
                )
            })
            .collect::<Vec<_>>();
        if overview_blocks.is_empty() {
            overview_blocks.push(StructuredBlock::empty_state(
                "overview-unavailable",
                "No supported OpenSpec overview sections are available",
            ));
        }

        let mut task_blocks = Vec::new();
        if let OpenSpecProgress::Available(task_progress) = &progress {
            task_blocks.push(StructuredBlock::progress(
                "task-progress",
                "Tasks",
                task_progress.completed as u64,
                task_progress.total as u64,
            ));
            for (group_index, group) in legacy_view.task_groups.into_iter().enumerate() {
                let source_path = group.source_path.clone();
                let items = group
                    .tasks
                    .into_iter()
                    .enumerate()
                    .map(|(task_index, task)| {
                        StructuredItem::new(
                            format!("task-{group_index}-{task_index}"),
                            task.text,
                            None,
                            SourceReference::new(
                                source_path.clone(),
                                Some(line_number(task.location.line)),
                            ),
                        )
                        .with_facts(vec![fact(
                            "openspec.task.completed",
                            "Completed",
                            FactValue::Boolean(task.completed),
                            vec![source_path.clone()],
                        )])
                    })
                    .collect();
                task_blocks.push(StructuredBlock::item_collection(
                    format!("task-group-{group_index}"),
                    group.title,
                    items,
                ));
            }
        } else {
            task_blocks.push(StructuredBlock::empty_state(
                "tasks-unavailable",
                "Supported deterministic task progress is unavailable",
            ));
        }
        task_blocks.extend(
            progress_warnings
                .into_iter()
                .enumerate()
                .map(|(index, warning)| {
                    StructuredBlock::warning(format!("task-warning-{index}"), warning)
                }),
        );

        let source_view = generic_source_detail(record, capture)
            .into_iter()
            .next()
            .expect("generic source detail always returns one capability");
        Ok(vec![
            CapabilityView::new(Capability::new("overview", "Overview"), overview_blocks),
            CapabilityView::new(Capability::new("tasks", "Tasks"), task_blocks),
            source_view,
        ])
    }

    fn build_handoff(
        &self,
        record: &DetectedRecord,
        capture: &RecordSourceCapture,
    ) -> Result<AdapterHandoff, AdapterFailure> {
        let location = record_location(record)?;
        let (progress, warnings) = progress_from_capture(record, capture);
        let primary_source_path = task_source_paths(record).into_iter().next().or_else(|| {
            record
                .claims
                .first()
                .map(|claim| claim.relative_path.clone())
        });
        let (progress_text, remaining) = match progress {
            OpenSpecProgress::Available(progress) => {
                let remaining = progress
                    .remaining
                    .into_iter()
                    .map(|task| format!("- {} (tasks.md:{})", task.text, task.location.line))
                    .collect::<Vec<_>>();
                (
                    format!(
                        "{} of {} tasks complete; {} remaining",
                        progress.completed, progress.total, progress.remaining_count
                    ),
                    if remaining.is_empty() {
                        "- None observed".to_owned()
                    } else {
                        remaining.join("\n")
                    },
                )
            }
            OpenSpecProgress::Unavailable(_) => (
                "Progress unavailable; no supported deterministic task markers were parsed"
                    .to_owned(),
                "- Inspect source to determine remaining work".to_owned(),
            ),
        };
        let warning_text = if warnings.is_empty() {
            "- None".to_owned()
        } else {
            warnings
                .into_iter()
                .map(|warning| format!("- {}", warning.message))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let prompt = format!(
            "Continue work on the OpenSpec record below.\n\nOpenSpec change: {}\nRecord: {}\nCustody: {}\nDeterministic status: {}\n\nObserved remaining tasks:\n{}\n\nOperational warnings:\n{}\n\nInstructions:\n1. Inspect the source files before continuing; repository content is authoritative.\n2. Reconcile the deterministic task facts above with the current source.\n3. Continue from the next valid unfinished task.\n4. Do not modify repository content unless the user explicitly asks.",
            record.display_name,
            record.adapter_record_key,
            custody_label(&location.custody),
            progress_text,
            remaining,
            warning_text,
        );
        Ok(AdapterHandoff::new(primary_source_path, prompt))
    }
}

fn openspec_capabilities() -> Vec<Capability> {
    vec![
        Capability::new("overview", "Overview"),
        Capability::new("tasks", "Tasks"),
        Capability::new("source", "Source"),
    ]
}

fn record_location(record: &DetectedRecord) -> Result<OpenSpecLocation, AdapterFailure> {
    record
        .claims
        .iter()
        .find_map(|claim| openspec_location(&claim.relative_path))
        .filter(|location| location.directory == record.adapter_record_key)
        .ok_or_else(|| {
            AdapterFailure::new(
                "invalid_openspec_record",
                format!(
                    "{} is not a supported OpenSpec record key",
                    record.adapter_record_key
                ),
            )
        })
}

fn progress_from_capture(
    record: &DetectedRecord,
    capture: &RecordSourceCapture,
) -> (OpenSpecProgress, Vec<WorkRecordWarning>) {
    let tasks_path = task_source_paths(record).into_iter().next();
    let progress = tasks_path
        .as_deref()
        .and_then(|path| capture.snapshot(path))
        .and_then(|snapshot| snapshot.text())
        .map(parse_openspec_tasks)
        .unwrap_or_else(unavailable_progress);
    let mut warnings: Vec<WorkRecordWarning> = match &progress {
        OpenSpecProgress::Available(progress) => progress
            .warnings
            .iter()
            .map(|warning| task_parse_warning(tasks_path.as_deref(), warning))
            .collect(),
        OpenSpecProgress::Unavailable(progress) => progress
            .warnings
            .iter()
            .map(|warning| task_parse_warning(tasks_path.as_deref(), warning))
            .collect(),
    };
    if matches!(progress, OpenSpecProgress::Unavailable(_)) {
        warnings.push(WorkRecordWarning::new(
            "openspec_progress_unavailable",
            "Supported deterministic OpenSpec task progress is unavailable",
            tasks_path,
        ));
    }
    (progress, warnings)
}

fn unavailable_progress() -> OpenSpecProgress {
    OpenSpecProgress::Unavailable(ProgressFallback {
        parser: ParserProvenance {
            name: "openspec-task-markers".to_owned(),
            version: "1".to_owned(),
        },
        warnings: vec![],
    })
}

fn task_parse_warning(
    source_path: Option<&str>,
    warning: &crate::ParseWarning,
) -> WorkRecordWarning {
    WorkRecordWarning::new(
        "openspec_task_parse_warning",
        warning.message.clone(),
        source_path,
    )
    .with_line(line_number(warning.line))
}

fn task_source_paths(record: &DetectedRecord) -> Vec<String> {
    record
        .claims
        .iter()
        .filter(|claim| claim.relative_path.rsplit('/').next() == Some("tasks.md"))
        .map(|claim| claim.relative_path.clone())
        .collect()
}

fn fact(key: &str, label: &str, value: FactValue, source_paths: Vec<String>) -> SummaryFact {
    SummaryFact::new(
        key,
        label,
        value,
        FactProvenance::new(ADAPTER_ID, source_paths),
    )
}

fn custody_value(custody: &OpenSpecCustody) -> &'static str {
    match custody {
        OpenSpecCustody::Current => "current",
        OpenSpecCustody::Archived { .. } => "archived",
    }
}

fn custody_label(custody: &OpenSpecCustody) -> String {
    match custody {
        OpenSpecCustody::Current => "Current".to_owned(),
        OpenSpecCustody::Archived {
            archived_on: Some(date),
        } => format!("Archived on {date}"),
        OpenSpecCustody::Archived { archived_on: None } => {
            "Archived (archive date unavailable)".to_owned()
        }
    }
}

fn status_value(status: OpenSpecPrimaryStatus) -> &'static str {
    match status {
        OpenSpecPrimaryStatus::Active => "active",
        OpenSpecPrimaryStatus::Done => "done",
        OpenSpecPrimaryStatus::Archived => "archived",
    }
}

fn overview_label(kind: OpenSpecOverviewKind) -> &'static str {
    match kind {
        OpenSpecOverviewKind::Why => "Why",
        OpenSpecOverviewKind::WhatChanges => "What Changes",
        OpenSpecOverviewKind::GoalsAndNonGoals => "Goals / Non-Goals",
        OpenSpecOverviewKind::Decisions => "Decisions",
        OpenSpecOverviewKind::RisksAndTradeOffs => "Risks / Trade-offs",
    }
}

fn line_number(line: usize) -> u32 {
    u32::try_from(line).unwrap_or(u32::MAX)
}
