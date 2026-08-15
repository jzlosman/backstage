use std::collections::{BTreeMap, BTreeSet};

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::format_adapters::generic_source_detail;
use crate::{
    AdapterDescriptor, AdapterFailure, AdapterHandoff, AdapterSummary, Capability, CapabilityView,
    DetectedRecord, FactProvenance, FactValue, PlanningFormatAdapter, ProjectSourceInventory,
    RecognitionLevel, RecordLocator, RecordSourceCapture, SourceClaim, SourceReference,
    StructuredBlock, StructuredItem, StructuredRelationship, SubjectId, SummaryFact,
    WorkRecordWarning, fingerprint_complete_snapshots,
};

const ADAPTER_ID: &str = "wayfinder-local-v1";
const FORMAT_ID: &str = "wayfinder-local";
const PRECEDENCE: u16 = 20;

pub struct WayfinderLocalAdapter {
    descriptor: AdapterDescriptor,
}

impl WayfinderLocalAdapter {
    pub fn new() -> Self {
        Self {
            descriptor: AdapterDescriptor::new(ADAPTER_ID, FORMAT_ID, 1, PRECEDENCE),
        }
    }
}

impl Default for WayfinderLocalAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl PlanningFormatAdapter for WayfinderLocalAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    fn detect(
        &self,
        inventory: &ProjectSourceInventory,
    ) -> Result<Vec<DetectedRecord>, AdapterFailure> {
        let roots = inventory
            .sources
            .iter()
            .filter_map(|source| wayfinder_root(&source.relative_path))
            .collect::<BTreeSet<_>>();
        Ok(roots
            .into_iter()
            .map(|root| {
                let prefix = format!("{root}/");
                let claims = inventory
                    .sources
                    .iter()
                    .filter(|source| source.relative_path.starts_with(&prefix))
                    .map(|source| SourceClaim::new(source.relative_path.clone()))
                    .collect::<Vec<_>>();
                let has_questions = claims.iter().any(|claim| {
                    claim
                        .relative_path
                        .strip_prefix(&prefix)
                        .and_then(canonical_wayfinder_ticket_number)
                        .is_some()
                });
                let display_name = root.rsplit('/').next().unwrap_or(&root).to_owned();
                DetectedRecord::new(
                    root,
                    display_name,
                    RecognitionLevel::Recognized,
                    claims,
                    vec!["Exact .scratch/<effort>/map.md local Wayfinder map".to_owned()],
                    wayfinder_capabilities(has_questions),
                )
            })
            .collect())
    }

    fn summarize(
        &self,
        record: &DetectedRecord,
        capture: &RecordSourceCapture,
    ) -> Result<AdapterSummary, AdapterFailure> {
        validate_record(record)?;
        let parsed = parse_record(record, capture);
        let mut warnings = parsed.warnings;
        let open_count = parsed
            .tickets
            .iter()
            .filter(|ticket| ticket.status == Some(WayfinderTicketStatus::Open))
            .count() as u64;
        let claimed_count = parsed
            .tickets
            .iter()
            .filter(|ticket| ticket.status == Some(WayfinderTicketStatus::Claimed))
            .count() as u64;
        let resolved_count = parsed
            .tickets
            .iter()
            .filter(|ticket| ticket.status == Some(WayfinderTicketStatus::Resolved))
            .count() as u64;
        let ticket_paths = parsed
            .tickets
            .iter()
            .map(|ticket| ticket.source_path.clone())
            .collect::<Vec<_>>();
        let mut facts = vec![
            fact(
                "wayfinder.ticket.count",
                "Tickets",
                FactValue::Count(parsed.canonical_ticket_count as u64),
                ticket_paths.clone(),
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
        if parsed.ticket_inputs_complete {
            facts.extend([
                fact(
                    "wayfinder.ticket.open_count",
                    "Open",
                    FactValue::Count(open_count),
                    ticket_paths.clone(),
                ),
                fact(
                    "wayfinder.ticket.claimed_count",
                    "Claimed",
                    FactValue::Count(claimed_count),
                    ticket_paths.clone(),
                ),
                fact(
                    "wayfinder.ticket.resolved_count",
                    "Resolved",
                    FactValue::Count(resolved_count),
                    ticket_paths.clone(),
                ),
                fact(
                    "wayfinder.frontier.count",
                    "Frontier",
                    FactValue::Count(parsed.frontier.ticket_numbers.len() as u64),
                    ticket_paths.clone(),
                ),
            ]);
            if let Some(next) = parsed.frontier.next_ticket_number {
                facts.push(fact(
                    "wayfinder.frontier.next_ticket",
                    "Next candidate",
                    FactValue::Count(next),
                    ticket_paths,
                ));
            }
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
                "Wayfinder fingerprint is unavailable because the captured record is incomplete",
            ));
            None
        };
        Ok(AdapterSummary::new(
            facts,
            warnings,
            wayfinder_capabilities(!parsed.tickets.is_empty()),
            fingerprint,
        ))
    }

    fn build_detail(
        &self,
        record: &DetectedRecord,
        capture: &RecordSourceCapture,
    ) -> Result<Vec<CapabilityView>, AdapterFailure> {
        validate_record(record)?;
        let parsed = parse_record(record, capture);
        let mut overview = parsed
            .map
            .as_ref()
            .into_iter()
            .flat_map(|map| &map.sections)
            .map(|section| {
                StructuredBlock::markdown_section(
                    format!("map-{}", section.kind.key()),
                    section.kind.label(),
                    section.markdown.clone(),
                    SourceReference::new(parsed.map_path.clone(), Some(to_line(section.line))),
                )
            })
            .collect::<Vec<_>>();
        if overview.is_empty() {
            overview.push(StructuredBlock::empty_state(
                "overview-unavailable",
                "No supported local Wayfinder map sections are available",
            ));
        }
        if parsed.ticket_inputs_complete {
            let frontier_facts = vec![
                fact(
                    "wayfinder.frontier.count",
                    "Eligible tickets",
                    FactValue::Count(parsed.frontier.ticket_numbers.len() as u64),
                    parsed
                        .tickets
                        .iter()
                        .map(|ticket| ticket.source_path.clone())
                        .collect(),
                ),
                fact(
                    "wayfinder.frontier.next",
                    "Next candidate",
                    FactValue::Text(
                        parsed
                            .frontier
                            .next_ticket_number
                            .map(|number| format!("#{number}"))
                            .unwrap_or_else(|| "None".to_owned()),
                    ),
                    vec![],
                ),
            ];
            overview.push(StructuredBlock::fact_register(
                "frontier-summary",
                "Frontier",
                frontier_facts,
            ));
            if !parsed.frontier.ticket_numbers.is_empty() {
                overview.push(StructuredBlock::item_collection(
                    "frontier-tickets",
                    "Eligible tickets",
                    parsed
                        .frontier
                        .ticket_numbers
                        .iter()
                        .filter_map(|number| {
                            parsed
                                .tickets
                                .iter()
                                .find(|ticket| ticket.number == *number)
                        })
                        .map(|ticket| {
                            StructuredItem::new(
                                format!("frontier-{}", ticket.number),
                                format!(
                                    "#{} {}",
                                    ticket.number,
                                    ticket.question.as_deref().unwrap_or("Question unavailable")
                                ),
                                None,
                                SourceReference::new(
                                    ticket.source_path.clone(),
                                    ticket.question_line.map(to_line),
                                ),
                            )
                        })
                        .collect(),
                ));
            }
        } else {
            overview.push(StructuredBlock::empty_state(
                "frontier-unavailable",
                "Frontier is unavailable because one or more canonical ticket sources could not be parsed safely",
            ));
        }
        overview.extend(
            parsed
                .map
                .as_ref()
                .into_iter()
                .flat_map(|map| &map.warnings)
                .enumerate()
                .map(|(index, warning)| {
                    StructuredBlock::warning(format!("map-warning-{index}"), warning.clone())
                }),
        );

        let eligible = parsed
            .frontier
            .ticket_numbers
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let question_items = parsed
            .tickets
            .iter()
            .map(|ticket| ticket_item(record, ticket, eligible.contains(&ticket.number)))
            .collect::<Vec<_>>();
        let mut questions = if question_items.is_empty() {
            vec![StructuredBlock::empty_state(
                "questions-unavailable",
                "No canonical local Wayfinder decision tickets are available",
            )]
        } else {
            vec![StructuredBlock::item_collection(
                "questions",
                "Questions",
                question_items,
            )]
        };
        questions.extend(
            parsed
                .tickets
                .iter()
                .flat_map(|ticket| &ticket.warnings)
                .chain(parsed.frontier.warnings.iter())
                .enumerate()
                .map(|(index, warning)| {
                    StructuredBlock::warning(format!("question-warning-{index}"), warning.clone())
                }),
        );

        let mut source = generic_source_detail(record, capture)
            .into_iter()
            .next()
            .expect("generic source detail always returns one view");
        source
            .blocks
            .extend(parsed.warnings.iter().enumerate().map(|(index, warning)| {
                StructuredBlock::warning(format!("wayfinder-warning-{index}"), warning.clone())
            }));
        let mut views = vec![CapabilityView::new(
            Capability::new("overview", "Overview"),
            overview,
        )];
        if !parsed.tickets.is_empty() {
            views.push(CapabilityView::new(
                Capability::new("questions", "Questions"),
                questions,
            ));
        }
        views.push(source);
        Ok(views)
    }

    fn build_handoff(
        &self,
        record: &DetectedRecord,
        capture: &RecordSourceCapture,
    ) -> Result<AdapterHandoff, AdapterFailure> {
        validate_record(record)?;
        let parsed = parse_record(record, capture);
        let frontier = if !parsed.ticket_inputs_complete {
            "Unavailable because one or more canonical ticket sources could not be parsed safely"
                .to_owned()
        } else if parsed.frontier.ticket_numbers.is_empty() {
            "None deterministically eligible".to_owned()
        } else {
            parsed
                .frontier
                .ticket_numbers
                .iter()
                .map(|number| format!("#{number}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        Ok(AdapterHandoff::new(
            Some(parsed.map_path.clone()),
            format!(
                "Continue local Wayfinder work from the safely captured sources.\n\nEffort: {}\nMap: {}\nFrontier: {}\n\nInspect the exact map and ticket sources before continuing. Do not claim, resolve, or edit a ticket unless the user explicitly asks.",
                record.display_name, parsed.map_path, frontier
            ),
        ))
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WayfinderMapSectionKind {
    Destination,
    Notes,
    DecisionsSoFar,
    NotYetSpecified,
    OutOfScope,
}

impl WayfinderMapSectionKind {
    fn from_heading(heading: &str) -> Option<Self> {
        match heading {
            "Destination" => Some(Self::Destination),
            "Notes" => Some(Self::Notes),
            "Decisions so far" => Some(Self::DecisionsSoFar),
            "Not yet specified" => Some(Self::NotYetSpecified),
            "Out of scope" => Some(Self::OutOfScope),
            _ => None,
        }
    }

    fn all() -> [Self; 5] {
        [
            Self::Destination,
            Self::Notes,
            Self::DecisionsSoFar,
            Self::NotYetSpecified,
            Self::OutOfScope,
        ]
    }

    fn label(self) -> &'static str {
        match self {
            Self::Destination => "Destination",
            Self::Notes => "Notes",
            Self::DecisionsSoFar => "Decisions so far",
            Self::NotYetSpecified => "Not yet specified",
            Self::OutOfScope => "Out of scope",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Destination => "destination",
            Self::Notes => "notes",
            Self::DecisionsSoFar => "decisions",
            Self::NotYetSpecified => "unspecified",
            Self::OutOfScope => "out-of-scope",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WayfinderMapSection {
    pub kind: WayfinderMapSectionKind,
    pub markdown: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedWayfinderMap {
    pub sections: Vec<WayfinderMapSection>,
    pub warnings: Vec<WorkRecordWarning>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WayfinderTicketStatus {
    Open,
    Claimed,
    Resolved,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WayfinderTicket {
    pub number: u64,
    pub source_path: String,
    pub kind: Option<String>,
    pub status: Option<WayfinderTicketStatus>,
    pub blockers: Option<Vec<u64>>,
    pub question: Option<String>,
    pub answer: Option<String>,
    pub question_line: Option<usize>,
    pub answer_line: Option<usize>,
    pub warnings: Vec<WorkRecordWarning>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WayfinderFrontier {
    pub ticket_numbers: Vec<u64>,
    pub next_ticket_number: Option<u64>,
    pub warnings: Vec<WorkRecordWarning>,
}

pub fn canonical_wayfinder_ticket_number(relative_to_effort: &str) -> Option<u64> {
    let name = relative_to_effort.strip_prefix("issues/")?;
    if name.contains('/') {
        return None;
    }
    let captures = Regex::new(r"^([0-9]{2,})-([a-z0-9]+(?:-[a-z0-9]+)*)\.md$")
        .expect("static regular expression")
        .captures(name)?;
    let digits = captures.get(1)?.as_str();
    let number = digits.parse::<u64>().ok()?;
    (number > 0).then_some(number)
}

pub fn parse_wayfinder_map(source_path: &str, markdown: &str) -> ParsedWayfinderMap {
    let mut occurrences: BTreeMap<WayfinderMapSectionKind, Vec<RawSection>> = BTreeMap::new();
    for section in supported_sections(markdown, WayfinderMapSectionKind::from_heading) {
        occurrences
            .entry(section.kind)
            .or_default()
            .push(section.raw);
    }
    let mut sections = Vec::new();
    let mut warnings = Vec::new();
    for kind in WayfinderMapSectionKind::all() {
        match occurrences.remove(&kind).unwrap_or_default().as_slice() {
            [] => warnings.push(WorkRecordWarning::new(
                "wayfinder_map_section_missing",
                format!("Map section '{}' is unavailable", kind.label()),
                Some(source_path),
            )),
            [section] if !section.markdown.trim().is_empty() => {
                sections.push(WayfinderMapSection {
                    kind,
                    markdown: section.markdown.trim().to_owned(),
                    line: section.line,
                });
            }
            [section] => warnings.push(
                WorkRecordWarning::new(
                    "wayfinder_map_section_empty",
                    format!("Map section '{}' is empty", kind.label()),
                    Some(source_path),
                )
                .with_line(to_line(section.line)),
            ),
            duplicates => warnings.push(
                WorkRecordWarning::new(
                    "wayfinder_map_section_ambiguous",
                    format!(
                        "Map section '{}' occurs {} times and is unavailable",
                        kind.label(),
                        duplicates.len()
                    ),
                    Some(source_path),
                )
                .with_line(to_line(duplicates[0].line)),
            ),
        }
    }
    sections.sort_by_key(|section| section.line);
    warnings.sort();
    ParsedWayfinderMap { sections, warnings }
}

pub fn parse_wayfinder_ticket(source_path: &str, number: u64, markdown: &str) -> WayfinderTicket {
    let mut warnings = Vec::new();
    let metadata = metadata_before_first_level_two_heading(markdown);
    let kind = parse_unique_metadata(
        source_path,
        "Type",
        metadata.get("Type").cloned().unwrap_or_default(),
        &mut warnings,
        |value| matches!(value, "research" | "prototype" | "grilling" | "task"),
        false,
    );
    let status = match metadata.get("Status") {
        None => Some(WayfinderTicketStatus::Open),
        Some(values) => parse_unique_metadata(
            source_path,
            "Status",
            values.clone(),
            &mut warnings,
            |value| matches!(value, "claimed" | "resolved"),
            false,
        )
        .and_then(|value| match value.as_str() {
            "claimed" => Some(WayfinderTicketStatus::Claimed),
            "resolved" => Some(WayfinderTicketStatus::Resolved),
            _ => None,
        }),
    };
    let blockers = match metadata.get("Blocked by") {
        None => Some(vec![]),
        Some(values) => parse_unique_metadata(
            source_path,
            "Blocked by",
            values.clone(),
            &mut warnings,
            |_| true,
            false,
        )
        .and_then(|value| parse_blockers(source_path, &value, &mut warnings)),
    };

    let sections = supported_sections(markdown, |heading| match heading {
        "Question" => Some("question"),
        "Answer" => Some("answer"),
        _ => None,
    });
    let mut questions = Vec::new();
    let mut answers = Vec::new();
    for section in sections {
        if section.kind == "question" {
            questions.push(section.raw);
        } else {
            answers.push(section.raw);
        }
    }
    let (question, question_line) =
        parse_ticket_section(source_path, "Question", questions, true, &mut warnings);
    let (answer, answer_line) =
        parse_ticket_section(source_path, "Answer", answers, false, &mut warnings);
    warnings.sort();
    warnings.dedup();
    WayfinderTicket {
        number,
        source_path: source_path.to_owned(),
        kind,
        status,
        blockers,
        question,
        answer,
        question_line,
        answer_line,
        warnings,
    }
}

pub fn calculate_wayfinder_frontier(tickets: &[WayfinderTicket]) -> WayfinderFrontier {
    let mut by_number: BTreeMap<u64, Vec<&WayfinderTicket>> = BTreeMap::new();
    for ticket in tickets {
        by_number.entry(ticket.number).or_default().push(ticket);
    }
    let mut warnings = Vec::new();
    let duplicates = by_number
        .iter()
        .filter(|(_, entries)| entries.len() > 1)
        .map(|(number, _)| *number)
        .collect::<BTreeSet<_>>();
    for number in &duplicates {
        for ticket in &by_number[number] {
            warnings.push(WorkRecordWarning::new(
                "wayfinder_ticket_number_duplicate",
                format!("Ticket number #{number} is ambiguous after normalization"),
                Some(ticket.source_path.clone()),
            ));
        }
    }
    let mut eligible = Vec::new();
    for ticket in tickets {
        if duplicates.contains(&ticket.number)
            || ticket.status != Some(WayfinderTicketStatus::Open)
            || ticket.kind.is_none()
            || ticket.question.is_none()
        {
            continue;
        }
        let Some(blockers) = &ticket.blockers else {
            continue;
        };
        let mut all_resolved = true;
        for blocker in blockers {
            let matches = by_number.get(blocker).cloned().unwrap_or_default();
            if matches.len() != 1
                || duplicates.contains(blocker)
                || matches[0].status != Some(WayfinderTicketStatus::Resolved)
            {
                all_resolved = false;
                if matches.is_empty() || matches.len() > 1 || duplicates.contains(blocker) {
                    warnings.push(WorkRecordWarning::new(
                        "wayfinder_blocker_unresolved",
                        format!(
                            "Ticket #{} cannot resolve declared blocker #{} to one ticket",
                            ticket.number, blocker
                        ),
                        Some(ticket.source_path.clone()),
                    ));
                }
            }
        }
        if all_resolved {
            eligible.push(ticket.number);
        }
    }
    eligible.sort_unstable();
    eligible.dedup();
    warnings.sort();
    warnings.dedup();
    WayfinderFrontier {
        next_ticket_number: eligible.first().copied(),
        ticket_numbers: eligible,
        warnings,
    }
}

struct ParsedRecord {
    map_path: String,
    map: Option<ParsedWayfinderMap>,
    tickets: Vec<WayfinderTicket>,
    canonical_ticket_count: usize,
    ticket_inputs_complete: bool,
    frontier: WayfinderFrontier,
    warnings: Vec<WorkRecordWarning>,
}

fn parse_record(record: &DetectedRecord, capture: &RecordSourceCapture) -> ParsedRecord {
    let map_path = format!("{}/map.md", record.adapter_record_key);
    let mut warnings = Vec::new();
    let map = capture
        .snapshot(&map_path)
        .and_then(|snapshot| snapshot.text())
        .map(|markdown| parse_wayfinder_map(&map_path, markdown));
    if map.is_none() {
        warnings.push(WorkRecordWarning::new(
            "wayfinder_map_unavailable",
            "The canonical local Wayfinder map is not safely readable",
            Some(map_path.clone()),
        ));
    }
    let prefix = format!("{}/", record.adapter_record_key);
    let mut tickets = Vec::new();
    let mut canonical_ticket_count = 0;
    let mut ticket_inputs_complete = true;
    for claim in &record.claims {
        let Some(relative) = claim.relative_path.strip_prefix(&prefix) else {
            continue;
        };
        if !relative.starts_with("issues/") {
            continue;
        }
        let Some(number) = canonical_wayfinder_ticket_number(relative) else {
            if relative.strip_prefix("issues/").is_some_and(|name| {
                !name.contains('/') && name.to_ascii_lowercase().ends_with(".md")
            }) {
                warnings.push(WorkRecordWarning::new(
                    "wayfinder_ticket_filename_noncanonical",
                    "Issue source resembles a ticket but does not use canonical issues/<NN>-<slug>.md naming",
                    Some(claim.relative_path.clone()),
                ));
            }
            continue;
        };
        canonical_ticket_count += 1;
        let Some(snapshot) = capture.snapshot(&claim.relative_path) else {
            ticket_inputs_complete = false;
            warnings.push(WorkRecordWarning::new(
                "wayfinder_ticket_unavailable",
                "Canonical ticket source could not be captured safely",
                Some(claim.relative_path.clone()),
            ));
            continue;
        };
        match snapshot.text() {
            Some(markdown) => tickets.push(parse_wayfinder_ticket(
                &claim.relative_path,
                number,
                markdown,
            )),
            None => {
                ticket_inputs_complete = false;
                warnings.push(WorkRecordWarning::new(
                    "wayfinder_ticket_not_utf8",
                    "Canonical ticket source is not valid UTF-8",
                    Some(claim.relative_path.clone()),
                ));
            }
        }
    }
    tickets.sort_by(|left, right| {
        left.number
            .cmp(&right.number)
            .then_with(|| left.source_path.cmp(&right.source_path))
    });
    let mut frontier = calculate_wayfinder_frontier(&tickets);
    if !ticket_inputs_complete {
        frontier.ticket_numbers.clear();
        frontier.next_ticket_number = None;
        frontier.warnings.push(WorkRecordWarning::without_source(
            "wayfinder_frontier_unavailable",
            "Frontier is unavailable because one or more canonical ticket sources could not be parsed safely",
        ));
    }
    warnings.extend(
        map.as_ref()
            .into_iter()
            .flat_map(|parsed| parsed.warnings.clone()),
    );
    warnings.extend(tickets.iter().flat_map(|ticket| ticket.warnings.clone()));
    warnings.extend(frontier.warnings.clone());
    warnings.sort();
    warnings.dedup();
    ParsedRecord {
        map_path,
        map,
        tickets,
        canonical_ticket_count,
        ticket_inputs_complete,
        frontier,
        warnings,
    }
}

struct SupportedSection<K> {
    kind: K,
    raw: RawSection,
}

#[derive(Clone)]
struct RawSection {
    markdown: String,
    line: usize,
}

fn supported_sections<K: Copy>(
    markdown: &str,
    recognize: impl Fn(&str) -> Option<K>,
) -> Vec<SupportedSection<K>> {
    let mut sections = Vec::new();
    let mut current: Option<(K, usize, Vec<&str>)> = None;
    let mut fence: Option<(char, usize)> = None;
    for (index, line) in markdown.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim_start();
        if let Some((marker, run_length)) = fence_marker(trimmed) {
            match fence {
                Some((active, opening_length))
                    if marker == active
                        && run_length >= opening_length
                        && trimmed[run_length..].trim().is_empty() =>
                {
                    fence = None;
                }
                None => fence = Some((marker, run_length)),
                _ => {}
            }
            if let Some((_, _, content)) = &mut current {
                content.push(line);
            }
            continue;
        }
        if fence.is_none() {
            if let Some(heading) = line.strip_prefix("## ") {
                if let Some((kind, start, content)) = current.take() {
                    sections.push(SupportedSection {
                        kind,
                        raw: RawSection {
                            markdown: content.join("\n"),
                            line: start,
                        },
                    });
                }
                current = recognize(heading.trim_end()).map(|kind| (kind, line_number, vec![]));
                continue;
            }
        }
        if let Some((_, _, content)) = &mut current {
            content.push(line);
        }
    }
    if let Some((kind, start, content)) = current {
        sections.push(SupportedSection {
            kind,
            raw: RawSection {
                markdown: content.join("\n"),
                line: start,
            },
        });
    }
    sections
}

fn metadata_before_first_level_two_heading(
    markdown: &str,
) -> BTreeMap<String, Vec<(String, usize)>> {
    let mut metadata: BTreeMap<String, Vec<(String, usize)>> = BTreeMap::new();
    let mut fence: Option<(char, usize)> = None;
    for (index, line) in markdown.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some((marker, run_length)) = fence_marker(trimmed) {
            match fence {
                Some((active, opening_length))
                    if marker == active
                        && run_length >= opening_length
                        && trimmed[run_length..].trim().is_empty() =>
                {
                    fence = None;
                }
                None => fence = Some((marker, run_length)),
                _ => {}
            }
            continue;
        }
        if fence.is_some() {
            continue;
        }
        if line.starts_with("## ") {
            break;
        }
        for key in ["Type", "Status", "Blocked by"] {
            if let Some(value) = line.strip_prefix(&format!("{key}:")) {
                metadata
                    .entry(key.to_owned())
                    .or_default()
                    .push((value.trim().to_owned(), index + 1));
            }
        }
    }
    metadata
}

fn parse_unique_metadata(
    source_path: &str,
    label: &str,
    values: Vec<(String, usize)>,
    warnings: &mut Vec<WorkRecordWarning>,
    supported: impl Fn(&str) -> bool,
    optional: bool,
) -> Option<String> {
    match values.as_slice() {
        [] if optional => None,
        [] => {
            warnings.push(WorkRecordWarning::new(
                "wayfinder_metadata_missing",
                format!("Required {label}: metadata is absent"),
                Some(source_path),
            ));
            None
        }
        [(value, line)] if value.is_empty() => {
            warnings.push(
                WorkRecordWarning::new(
                    "wayfinder_metadata_empty",
                    format!("{label}: metadata is empty"),
                    Some(source_path),
                )
                .with_line(to_line(*line)),
            );
            None
        }
        [(value, line)] if supported(value) => Some(value.clone()),
        [(_, line)] => {
            warnings.push(
                WorkRecordWarning::new(
                    "wayfinder_metadata_unsupported",
                    format!("{label}: metadata uses an unsupported value"),
                    Some(source_path),
                )
                .with_line(to_line(*line)),
            );
            None
        }
        duplicates => {
            warnings.push(
                WorkRecordWarning::new(
                    "wayfinder_metadata_ambiguous",
                    format!("{label}: metadata occurs {} times", duplicates.len()),
                    Some(source_path),
                )
                .with_line(to_line(duplicates[0].1)),
            );
            None
        }
    }
}

fn parse_blockers(
    source_path: &str,
    value: &str,
    warnings: &mut Vec<WorkRecordWarning>,
) -> Option<Vec<u64>> {
    let mut blockers = Vec::new();
    for part in value.split(',') {
        let part = part.trim();
        if part.len() < 2 || !part.bytes().all(|byte| byte.is_ascii_digit()) {
            warnings.push(WorkRecordWarning::new(
                "wayfinder_blocker_invalid",
                "Blocked by: must contain comma-separated two-or-more-digit positive ticket numbers",
                Some(source_path),
            ));
            return None;
        }
        let Ok(number) = part.parse::<u64>() else {
            warnings.push(WorkRecordWarning::new(
                "wayfinder_blocker_invalid",
                "Blocked by: ticket number exceeds the supported integer range",
                Some(source_path),
            ));
            return None;
        };
        if number == 0 {
            warnings.push(WorkRecordWarning::new(
                "wayfinder_blocker_invalid",
                "Blocked by: ticket numbers must be positive",
                Some(source_path),
            ));
            return None;
        }
        blockers.push(number);
    }
    if blockers.is_empty() {
        warnings.push(WorkRecordWarning::new(
            "wayfinder_blocker_invalid",
            "Blocked by: metadata is empty",
            Some(source_path),
        ));
        return None;
    }
    blockers.sort_unstable();
    blockers.dedup();
    Some(blockers)
}

fn parse_ticket_section(
    source_path: &str,
    label: &str,
    sections: Vec<RawSection>,
    required: bool,
    warnings: &mut Vec<WorkRecordWarning>,
) -> (Option<String>, Option<usize>) {
    match sections.as_slice() {
        [] if !required => (None, None),
        [] => {
            warnings.push(WorkRecordWarning::new(
                "wayfinder_ticket_section_missing",
                format!("Ticket section '{label}' is absent"),
                Some(source_path),
            ));
            (None, None)
        }
        [section] if !section.markdown.trim().is_empty() => {
            (Some(section.markdown.trim().to_owned()), Some(section.line))
        }
        [section] => {
            warnings.push(
                WorkRecordWarning::new(
                    "wayfinder_ticket_section_empty",
                    format!("Ticket section '{label}' is empty"),
                    Some(source_path),
                )
                .with_line(to_line(section.line)),
            );
            (None, None)
        }
        duplicates => {
            warnings.push(
                WorkRecordWarning::new(
                    "wayfinder_ticket_section_ambiguous",
                    format!(
                        "Ticket section '{label}' occurs {} times and is unavailable",
                        duplicates.len()
                    ),
                    Some(source_path),
                )
                .with_line(to_line(duplicates[0].line)),
            );
            (None, None)
        }
    }
}

fn ticket_item(
    record: &DetectedRecord,
    ticket: &WayfinderTicket,
    eligible: bool,
) -> StructuredItem {
    let status = ticket
        .status
        .map(|status| match status {
            WayfinderTicketStatus::Open => "open",
            WayfinderTicketStatus::Claimed => "claimed",
            WayfinderTicketStatus::Resolved => "resolved",
        })
        .unwrap_or("unavailable");
    let blockers = ticket
        .blockers
        .as_ref()
        .map(|numbers| {
            if numbers.is_empty() {
                "None".to_owned()
            } else {
                numbers
                    .iter()
                    .map(|number| format!("#{number}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        })
        .unwrap_or_else(|| "Unavailable".to_owned());
    let markdown = match (&ticket.question, &ticket.answer) {
        (Some(question), Some(answer)) => Some(format!("{question}\n\n### Answer\n{answer}")),
        (Some(question), None) => Some(question.clone()),
        (None, Some(answer)) => Some(format!("### Answer\n{answer}")),
        (None, None) => None,
    };
    let relationships = ticket
        .blockers
        .as_ref()
        .into_iter()
        .flatten()
        .map(|blocker| {
            StructuredRelationship::new(
                "blocked_by",
                ticket_subject(record, *blocker),
                format!("Blocked by #{blocker}"),
            )
        })
        .collect();
    StructuredItem::new(
        format!("ticket-{}", ticket.number),
        format!(
            "#{} {}",
            ticket.number,
            ticket.question.as_deref().unwrap_or("Question unavailable")
        ),
        markdown,
        SourceReference::new(
            ticket.source_path.clone(),
            ticket.question_line.map(to_line),
        ),
    )
    .with_facts(vec![
        fact(
            "wayfinder.ticket.type",
            "Type",
            FactValue::Text(
                ticket
                    .kind
                    .clone()
                    .unwrap_or_else(|| "unavailable".to_owned()),
            ),
            vec![ticket.source_path.clone()],
        ),
        fact(
            "wayfinder.ticket.status",
            "Status",
            FactValue::Text(status.to_owned()),
            vec![ticket.source_path.clone()],
        ),
        fact(
            "wayfinder.ticket.blockers",
            "Blocked by",
            FactValue::Text(blockers),
            vec![ticket.source_path.clone()],
        ),
        fact(
            "wayfinder.ticket.frontier",
            "In frontier",
            FactValue::Boolean(eligible),
            vec![ticket.source_path.clone()],
        ),
    ])
    .with_relationships(relationships)
}

fn ticket_subject(record: &DetectedRecord, number: u64) -> SubjectId {
    RecordLocator::new(
        record.adapter_record_key.clone(),
        "wayfinder-ticket",
        format!("{}/issues/{number}", record.adapter_record_key),
    )
    .subject_id()
}

fn wayfinder_root(path: &str) -> Option<String> {
    let parts = path.split('/').collect::<Vec<_>>();
    if parts.len() == 3 && parts[0] == ".scratch" && !parts[1].is_empty() && parts[2] == "map.md" {
        Some(format!(".scratch/{}", parts[1]))
    } else {
        None
    }
}

fn validate_record(record: &DetectedRecord) -> Result<(), AdapterFailure> {
    let expected_map = format!("{}/map.md", record.adapter_record_key);
    if wayfinder_root(&expected_map).as_deref() == Some(record.adapter_record_key.as_str())
        && record
            .claims
            .iter()
            .any(|claim| claim.relative_path == expected_map)
    {
        Ok(())
    } else {
        Err(AdapterFailure::new(
            "invalid_wayfinder_record",
            "Wayfinder record does not contain its exact canonical local map",
        ))
    }
}

fn wayfinder_capabilities(has_questions: bool) -> Vec<Capability> {
    let mut capabilities = vec![Capability::new("overview", "Overview")];
    if has_questions {
        capabilities.push(Capability::new("questions", "Questions"));
    }
    capabilities.push(Capability::new("source", "Source"));
    capabilities
}

fn fact(key: &str, label: &str, value: FactValue, source_paths: Vec<String>) -> SummaryFact {
    SummaryFact::new(
        key,
        label,
        value,
        FactProvenance::new(ADAPTER_ID, source_paths),
    )
}

fn fence_marker(line: &str) -> Option<(char, usize)> {
    let marker = line.chars().next()?;
    if !matches!(marker, '`' | '~') {
        return None;
    }
    let run_length = line
        .chars()
        .take_while(|candidate| *candidate == marker)
        .count();
    (run_length >= 3).then_some((marker, run_length))
}

fn to_line(line: usize) -> u32 {
    u32::try_from(line).unwrap_or(u32::MAX)
}
