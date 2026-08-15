use backstage_core::{
    Capability, FactValue, FormatRegistry, MarkdownAdapter, PlanningFormatAdapter, PlanningPattern,
    PlanningPatternAdapter, ProjectSourceInventory, RecognitionLevel, RecordSourceCapture,
    SourceInventoryEntry, SourceObservation, SourceSnapshot, StructuredBlock,
    WayfinderLocalAdapter, WayfinderMapSectionKind, WayfinderTicketStatus,
    calculate_wayfinder_frontier, canonical_wayfinder_ticket_number, parse_wayfinder_map,
    parse_wayfinder_ticket,
};

fn observation(stamp: u128) -> SourceObservation {
    SourceObservation {
        byte_len: 100,
        modified_unix_nanos: Some(stamp),
    }
}

fn inventory(paths: &[&str]) -> ProjectSourceInventory {
    ProjectSourceInventory::new(
        "project_1",
        "Project",
        paths
            .iter()
            .enumerate()
            .map(|(index, path)| SourceInventoryEntry::new(*path, observation(index as u128 + 1)))
            .collect(),
    )
}

fn snapshot(path: &str, text: &str, stamp: u128) -> SourceSnapshot {
    let observed = SourceObservation {
        byte_len: text.len() as u64,
        modified_unix_nanos: Some(stamp),
    };
    SourceSnapshot::from_observations(path, text.as_bytes().to_vec(), observed, observed)
        .expect("snapshot")
}

#[test]
fn detects_only_exact_local_maps_and_groups_safe_descendant_markdown() {
    let adapter = WayfinderLocalAdapter::new();
    let inventory = inventory(&[
        ".scratch/search/map.md",
        ".scratch/search/issues/01-research.md",
        ".scratch/search/issues/1-bad.md",
        ".scratch/search/notes.md",
        ".scratch/other/MAP.md",
        "map.md",
        "remote.md",
    ]);
    let records = adapter.detect(&inventory).expect("detection");
    assert_eq!(
        records,
        adapter.detect(&inventory).expect("repeat detection")
    );

    assert_eq!(adapter.descriptor().adapter_id(), "wayfinder-local-v1");
    assert_eq!(adapter.descriptor().format_id(), "wayfinder-local");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].adapter_record_key, ".scratch/search");
    assert_eq!(records[0].display_name, "search");
    assert_eq!(records[0].recognition_level, RecognitionLevel::Recognized);
    assert_eq!(
        records[0]
            .claims
            .iter()
            .map(|claim| claim.relative_path.as_str())
            .collect::<Vec<_>>(),
        vec![
            ".scratch/search/issues/01-research.md",
            ".scratch/search/issues/1-bad.md",
            ".scratch/search/map.md",
            ".scratch/search/notes.md",
        ]
    );
    assert_eq!(
        records[0].capabilities,
        vec![
            Capability::new("overview", "Overview"),
            Capability::new("questions", "Questions"),
            Capability::new("source", "Source"),
        ]
    );
}

#[test]
fn registry_precedence_keeps_wayfinder_members_out_of_pattern_and_markdown_fallbacks() {
    let pattern = PlanningPattern::custom(r"(?:^|/)map\.md$", 0).expect("pattern");
    let registry = FormatRegistry::new(vec![
        Box::new(MarkdownAdapter::new()),
        Box::new(PlanningPatternAdapter::new(vec![pattern])),
        Box::new(WayfinderLocalAdapter::new()),
    ]);

    let detected = registry.detect(&inventory(&[
        ".scratch/search/map.md",
        ".scratch/search/notes.md",
        "map.md",
    ]));

    assert_eq!(detected.source_count, 3);
    let wayfinder = detected
        .records
        .iter()
        .find(|record| record.descriptor.format_id() == "wayfinder-local")
        .expect("wayfinder record");
    assert_eq!(wayfinder.claims.len(), 2);
    assert_eq!(
        detected
            .records
            .iter()
            .filter(|record| record.descriptor.format_id() == "planning-pattern")
            .flat_map(|record| &record.claims)
            .map(|claim| claim.relative_path.as_str())
            .collect::<Vec<_>>(),
        vec!["map.md"]
    );
    let represented = detected
        .records
        .iter()
        .map(|record| record.claims.len())
        .sum::<usize>();
    assert_eq!(represented, detected.source_count);
}

#[test]
fn canonical_ticket_filenames_are_strict_and_numbers_are_normalized() {
    assert_eq!(
        canonical_wayfinder_ticket_number("issues/01-first.md"),
        Some(1)
    );
    assert_eq!(
        canonical_wayfinder_ticket_number("issues/001-first.md"),
        Some(1)
    );
    for path in [
        "issues/1-first.md",
        "issues/00-zero.md",
        "issues/01-Upper.md",
        "issues/01-two--words.md",
        "issues/01-.md",
        "issues/nested/01-first.md",
        "issues/01-first.MD",
    ] {
        assert_eq!(canonical_wayfinder_ticket_number(path), None, "{path}");
    }
}

#[test]
fn map_parser_uses_exact_unfenced_headings_and_keeps_unambiguous_partial_sections() {
    let parsed = parse_wayfinder_map(
        ".scratch/search/map.md",
        "# Map\n\n## Destination\nShip it.\n\n````md\n```\n## Notes\nIgnored.\n```\n````\n\n## notes\nWrong case.\n\n## Out of scope\nRemote sync.\n",
    );

    assert_eq!(parsed.sections.len(), 2);
    assert_eq!(
        parsed.sections[0].kind,
        WayfinderMapSectionKind::Destination
    );
    assert!(parsed.sections[0].markdown.starts_with("Ship it."));
    assert!(parsed.sections[0].markdown.contains("## Notes\nIgnored."));
    assert_eq!(parsed.sections[1].kind, WayfinderMapSectionKind::OutOfScope);
    assert!(
        parsed
            .warnings
            .iter()
            .any(|warning| warning.code == "wayfinder_map_section_missing")
    );

    let ambiguous = parse_wayfinder_map(
        ".scratch/search/map.md",
        "## Destination\n\n## Destination\nSecond value\n## Notes\nUseful\n",
    );
    assert_eq!(ambiguous.sections.len(), 1);
    assert_eq!(ambiguous.sections[0].kind, WayfinderMapSectionKind::Notes);
    assert!(
        ambiguous
            .warnings
            .iter()
            .any(|warning| warning.code == "wayfinder_map_section_ambiguous")
    );
}

#[test]
fn ticket_parser_handles_defaults_exact_metadata_duplicates_and_partial_content() {
    let open = parse_wayfinder_ticket(
        ".scratch/search/issues/01-research.md",
        1,
        "Type: research\nBlocked by: 02, 003\n\n## Question\nWhat is safe?\n",
    );
    assert_eq!(open.kind.as_deref(), Some("research"));
    assert_eq!(open.status, Some(WayfinderTicketStatus::Open));
    assert_eq!(open.blockers, Some(vec![2, 3]));
    assert_eq!(open.question.as_deref(), Some("What is safe?"));
    assert!(open.answer.is_none());
    assert!(open.warnings.is_empty());

    let malformed = parse_wayfinder_ticket(
        ".scratch/search/issues/02-bad.md",
        2,
        "Type: task\nType: research\nStatus:\nBlocked by: 01, nope\n\n## Question\n\n## Question\nSecond.\n## Answer\nDone.\n",
    );
    assert!(malformed.kind.is_none());
    assert!(malformed.status.is_none());
    assert!(malformed.blockers.is_none());
    assert!(malformed.question.is_none());
    assert_eq!(malformed.answer.as_deref(), Some("Done."));
    assert!(malformed.warnings.len() >= 4);

    let overflowing_blocker = parse_wayfinder_ticket(
        ".scratch/search/issues/03-overflow.md",
        3,
        "Type: task\nBlocked by: 18446744073709551616\n\n## Question\nBounded?\n",
    );
    assert!(overflowing_blocker.blockers.is_none());
    assert!(
        overflowing_blocker
            .warnings
            .iter()
            .any(|warning| warning.code == "wayfinder_blocker_invalid"
                && warning.message.contains("integer range"))
    );

    let fenced_and_late = parse_wayfinder_ticket(
        ".scratch/search/issues/03-late.md",
        3,
        "```md\nType: task\n```\n## Question\nCanonical question.\nType: task\n## answer\nWrong-case answer.\n",
    );
    assert!(fenced_and_late.kind.is_none());
    assert_eq!(
        fenced_and_late.question.as_deref(),
        Some("Canonical question.\nType: task")
    );
    assert!(fenced_and_late.answer.is_none());
}

#[test]
fn frontier_requires_open_unclaimed_valid_tickets_and_unambiguous_resolved_blockers() {
    let tickets = vec![
        parse_wayfinder_ticket("issues/01-open.md", 1, "Type: task\n## Question\nFirst?\n"),
        parse_wayfinder_ticket(
            "issues/02-blocked.md",
            2,
            "Type: task\nBlocked by: 03\n## Question\nSecond?\n",
        ),
        parse_wayfinder_ticket(
            "issues/03-done.md",
            3,
            "Type: task\nStatus: resolved\n## Question\nThird?\n## Answer\nDone.\n",
        ),
        parse_wayfinder_ticket(
            "issues/04-claimed.md",
            4,
            "Type: task\nStatus: claimed\n## Question\nFourth?\n",
        ),
        parse_wayfinder_ticket(
            "issues/05-missing.md",
            5,
            "Type: task\nBlocked by: 99\n## Question\nFifth?\n",
        ),
        parse_wayfinder_ticket(
            "issues/06-open-blocker.md",
            6,
            "Type: task\nBlocked by: 01\n## Question\nSixth?\n",
        ),
    ];
    let frontier = calculate_wayfinder_frontier(&tickets);

    assert_eq!(frontier.ticket_numbers, vec![1, 2]);
    assert_eq!(frontier.next_ticket_number, Some(1));
    assert!(frontier.warnings.iter().any(|warning| {
        warning.code == "wayfinder_blocker_unresolved"
            && warning.source_path.as_deref() == Some("issues/05-missing.md")
    }));

    let duplicates = vec![
        parse_wayfinder_ticket("issues/01-a.md", 1, "Type: task\n## Question\nA?\n"),
        parse_wayfinder_ticket(
            "issues/001-b.md",
            1,
            "Type: task\nStatus: resolved\n## Question\nB?\n",
        ),
        parse_wayfinder_ticket(
            "issues/02-c.md",
            2,
            "Type: task\nBlocked by: 01\n## Question\nC?\n",
        ),
    ];
    let duplicate_frontier = calculate_wayfinder_frontier(&duplicates);
    assert!(duplicate_frontier.ticket_numbers.is_empty());
    assert!(
        duplicate_frontier
            .warnings
            .iter()
            .any(|warning| warning.code == "wayfinder_ticket_number_duplicate")
    );
}

#[test]
fn partial_canonical_ticket_capture_never_invents_a_frontier_from_remaining_tickets() {
    let adapter = WayfinderLocalAdapter::new();
    let inventory = inventory(&[
        ".scratch/search/map.md",
        ".scratch/search/issues/03-first-copy.md",
        ".scratch/search/issues/003-second-copy.md",
        ".scratch/search/issues/04-blocked.md",
    ]);
    let record = adapter.detect(&inventory).expect("detect").remove(0);
    let capture = RecordSourceCapture::partial(
        vec![
            snapshot(".scratch/search/map.md", "## Destination\nSafe.\n", 1),
            snapshot(
                ".scratch/search/issues/03-first-copy.md",
                "Type: task\nStatus: resolved\n## Question\nDone?\n## Answer\nYes.\n",
                2,
            ),
            snapshot(
                ".scratch/search/issues/04-blocked.md",
                "Type: task\nBlocked by: 03\n## Question\nReady?\n",
                3,
            ),
        ],
        vec![backstage_core::SourceCaptureFailure::new(
            ".scratch/search/issues/003-second-copy.md",
            "source_unavailable",
            "injected missing duplicate",
        )],
    );

    let summary = adapter.summarize(&record, &capture).expect("summary");
    assert!(
        summary
            .facts
            .iter()
            .all(|fact| fact.key != "wayfinder.frontier.count")
    );
    assert!(
        summary
            .warnings
            .iter()
            .any(|warning| warning.code == "wayfinder_frontier_unavailable")
    );
    let detail = adapter.build_detail(&record, &capture).expect("detail");
    assert!(detail[0].blocks.iter().any(|block| matches!(
        block,
        StructuredBlock::EmptyState { id, .. } if id == "frontier-unavailable"
    )));
    assert!(
        adapter
            .build_handoff(&record, &capture)
            .expect("handoff")
            .continuation_prompt
            .contains("Frontier: Unavailable")
    );
}

#[test]
fn adapter_builds_index_facts_structured_views_warnings_and_inert_handoff() {
    let adapter = WayfinderLocalAdapter::new();
    let inventory = inventory(&[
        ".scratch/search/map.md",
        ".scratch/search/issues/01-first.md",
        ".scratch/search/issues/02-second.md",
        ".scratch/search/issues/3-noncanonical.md",
        ".scratch/search/notes.md",
    ]);
    let record = adapter.detect(&inventory).expect("detect").remove(0);
    let capture = RecordSourceCapture::complete(vec![
        snapshot(
            ".scratch/search/map.md",
            "## Destination\nShip search.\n## Notes\nStay local.\n",
            1,
        ),
        snapshot(
            ".scratch/search/issues/01-first.md",
            "Type: research\n## Question\nWhat first?\n",
            2,
        ),
        snapshot(
            ".scratch/search/issues/02-second.md",
            "Type: task\nStatus: resolved\n## Question\nWhat second?\n## Answer\nDone.\n",
            3,
        ),
        snapshot(
            ".scratch/search/issues/3-noncanonical.md",
            "Type: task\n## Question\nNot a ticket.\n",
            4,
        ),
        snapshot(".scratch/search/notes.md", "# Notes\nRelated.\n", 5),
    ]);

    let summary = adapter.summarize(&record, &capture).expect("summary");
    assert!(summary.fingerprint.is_some());
    assert!(
        summary
            .warnings
            .iter()
            .any(|warning| warning.code == "wayfinder_ticket_filename_noncanonical")
    );
    assert_eq!(
        summary
            .facts
            .iter()
            .find(|fact| fact.key == "wayfinder.frontier.count")
            .map(|fact| &fact.value),
        Some(&FactValue::Count(1)),
    );
    let views = adapter.build_detail(&record, &capture).expect("detail");
    assert_eq!(
        views
            .iter()
            .map(|view| view.capability.id.as_str())
            .collect::<Vec<_>>(),
        vec!["overview", "questions", "source"]
    );
    assert!(views[0].blocks.iter().any(|block| matches!(block, StructuredBlock::MarkdownSection { title, .. } if title == "Destination")));
    assert!(views[1].blocks.iter().any(
        |block| matches!(block, StructuredBlock::ItemCollection { items, .. } if items.len() == 2)
    ));
    assert!(views[2].blocks.iter().any(|block| matches!(
        block,
        StructuredBlock::Warning { warning, .. }
            if warning.code == "wayfinder_ticket_filename_noncanonical"
    )));

    let handoff = adapter.build_handoff(&record, &capture).expect("handoff");
    assert_eq!(
        handoff.primary_source_path.as_deref(),
        Some(".scratch/search/map.md")
    );
    assert!(handoff.continuation_prompt.contains("Frontier: #1"));
    assert!(
        handoff
            .continuation_prompt
            .contains("Do not claim, resolve, or edit a ticket")
    );
}
