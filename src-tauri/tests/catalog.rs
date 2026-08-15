mod support;

use std::fs;

use backstage_app_lib::catalog::{
    CatalogError, artifact_detail, build_index, build_index_controlled, build_index_with_patterns,
    markdown_detail, work_record_detail, work_record_handoff,
};
use backstage_app_lib::discovery::{
    CancellationToken, ProjectCandidate, ScanPolicy, discover_projects,
};
use backstage_app_lib::filesystem::ContainedReader;
use backstage_app_lib::{derive_artifact_path, derive_continuation_prompt, derive_markdown_path};
use backstage_core::{
    BundleKind, OpenSpecCustody, OpenSpecPrimaryStatus, OpenSpecProgress, PlanningPattern,
    RecognitionLevel, StructuredBlock,
};
use tempfile::TempDir;

use support::FixtureRepo;

#[test]
fn catalog_recognizes_planning_work_and_indexes_all_markdown_separately() {
    let fixture = FixtureRepo::open_spec();
    fs::write(fixture.path().join("notes.MD"), "# Uppercase extension\n")
        .expect("write uppercase Markdown");
    let before = fixture.manifest();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());

    let index = build_index(
        &reader,
        discovered.projects,
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
    );

    let bundles = &index.projects[0].bundles;
    assert!(bundles.iter().any(|bundle| {
        bundle.bundle.name == "ship-search"
            && bundle.bundle.kind == BundleKind::OpenSpecChange
            && matches!(bundle.progress, OpenSpecProgress::Available(_))
    }));
    assert!(bundles.iter().any(|bundle| {
        bundle.bundle.name == "PLAN.md" && bundle.bundle.kind == BundleKind::PossibleArtifact
    }));
    assert!(
        !bundles
            .iter()
            .flat_map(|bundle| &bundle.bundle.members)
            .any(|member| member.relative_path == "README.md")
    );

    let documents = &index.projects[0].markdown_documents;
    let paths = documents
        .iter()
        .map(|document| document.relative_path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        vec![
            "PLAN.md",
            "README.md",
            "notes.MD",
            "openspec/changes/ship-search/design.md",
            "openspec/changes/ship-search/proposal.md",
            "openspec/changes/ship-search/specs/search/spec.md",
            "openspec/changes/ship-search/tasks.md",
        ]
    );
    let member_ids = bundles
        .iter()
        .flat_map(|bundle| &bundle.bundle.members)
        .map(|member| (&member.relative_path, &member.id))
        .collect::<std::collections::BTreeMap<_, _>>();
    for document in documents {
        if let Some(member_id) = member_ids.get(&document.relative_path) {
            assert_eq!(*member_id, &document.id);
        }
    }
    fixture.assert_unchanged(&before);
}

#[test]
fn catalog_composes_one_neutral_record_collection_with_complete_source_counts() {
    let fixture = FixtureRepo::open_spec();
    fs::write(fixture.path().join("notes.md"), "# Notes\n").expect("write notes");
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());

    let index = build_index(
        &reader,
        discovered.projects,
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
    );
    let project = &index.projects[0];

    assert_eq!(project.source_count, project.markdown_documents.len());
    assert_eq!(project.source_count, 7);
    assert_eq!(project.records.len(), 4);
    let openspec = project
        .records
        .iter()
        .find(|record| record.locator.format_id == "openspec")
        .expect("neutral OpenSpec record");
    assert_eq!(openspec.recognition.level, RecognitionLevel::Recognized);
    assert_eq!(openspec.sources.len(), 4);
    assert!(openspec.fingerprint.is_some());
    assert_eq!(
        openspec.source_modified_unix_nanos,
        openspec
            .sources
            .iter()
            .filter_map(|source| source.source_modified_unix_nanos)
            .max()
    );
    assert!(project.records.iter().any(|record| {
        record.recognition.level == RecognitionLevel::Possible
            && record.sources[0].relative_path == "PLAN.md"
    }));
    assert!(project.records.iter().any(|record| {
        record.recognition.level == RecognitionLevel::Plain
            && record.sources[0].relative_path == "README.md"
    }));
    let represented = project
        .records
        .iter()
        .flat_map(|record| {
            record
                .sources
                .iter()
                .map(|source| source.relative_path.as_str())
        })
        .collect::<std::collections::BTreeSet<_>>();
    let discovered = project
        .markdown_documents
        .iter()
        .map(|document| document.relative_path.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(represented, discovered);
}

#[test]
fn neutral_record_survives_a_partial_member_capture_with_source_warnings() {
    let root = TempDir::new().expect("root tempdir");
    let change = root.path().join("openspec/changes/partial");
    fs::create_dir_all(&change).expect("change directory");
    fs::write(change.join("proposal.md"), "## Why\n\nReadable.\n").expect("proposal");
    fs::write(change.join("tasks.md"), vec![b'x'; 256]).expect("oversized tasks");
    let reader = ContainedReader::approve(root.path(), 64).expect("approve root");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());

    let index = build_index(
        &reader,
        discovered.projects,
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
    );
    let record = index.projects[0]
        .records
        .iter()
        .find(|record| record.locator.format_id == "openspec")
        .expect("partial OpenSpec record");

    assert_eq!(record.sources.len(), 2);
    assert!(record.fingerprint.is_none());
    assert!(record.warnings.iter().any(|warning| {
        warning.source_path.as_deref() == Some("openspec/changes/partial/tasks.md")
    }));
    assert!(record.warnings.iter().any(|warning| {
        warning.code == "incomplete_source_snapshot"
            || warning.code == "openspec_progress_unavailable"
    }));
}

#[test]
fn neutral_detail_and_handoff_use_fresh_contained_sources() {
    let fixture = FixtureRepo::open_spec();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());
    let index = build_index(
        &reader,
        discovered.projects,
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
    );
    let record = index.projects[0]
        .records
        .iter()
        .find(|record| record.locator.format_id == "openspec")
        .expect("OpenSpec record");
    let indexed_fingerprint = record.fingerprint.clone();
    fs::write(
        fixture.path().join("openspec/changes/ship-search/tasks.md"),
        "# Tasks\n\n- [x] Fresh task\n- [ ] Remaining task\n",
    )
    .expect("change tasks after indexing");

    let detail = work_record_detail(&reader, &index, record.subject_id.as_str())
        .expect("fresh neutral detail");
    let handoff = work_record_handoff(&reader, &index, record.subject_id.as_str())
        .expect("fresh neutral handoff");

    assert_eq!(detail.subject_id, record.subject_id);
    assert_eq!(detail.index_generation, 1);
    assert_ne!(detail.fingerprint, indexed_fingerprint);
    assert!(detail.record.facts.iter().any(|fact| {
        fact.key == "openspec.task.done_count" && fact.value == backstage_core::FactValue::Count(1)
    }));
    assert!(detail.record.facts.iter().any(|fact| {
        fact.key == "openspec.task.open_count" && fact.value == backstage_core::FactValue::Count(1)
    }));
    assert_eq!(
        detail
            .capabilities
            .iter()
            .map(|view| view.capability.id.as_str())
            .collect::<Vec<_>>(),
        vec!["overview", "tasks", "source"]
    );
    assert!(detail.capabilities[1].blocks.iter().any(|block| matches!(
        block,
        StructuredBlock::Progress {
            completed: 1,
            total: 2,
            ..
        }
    )));
    assert!(detail.capabilities[2].blocks.iter().any(|block| matches!(
        block,
        StructuredBlock::MarkdownSection { markdown, .. }
            if markdown.contains("Fresh task")
    )));
    assert_eq!(
        handoff.handoff.primary_source_path.as_deref(),
        Some("openspec/changes/ship-search/tasks.md")
    );
    assert!(
        handoff
            .handoff
            .continuation_prompt
            .contains("Remaining task")
    );
}

#[test]
fn every_compiled_format_scan_detail_and_handoff_path_preserves_repository_bytes() {
    let root = TempDir::new().expect("root");
    let files = [
        ("README.md", "# Ordinary\n"),
        ("PLAN.md", "# Plan\n"),
        (
            "openspec/changes/change/proposal.md",
            "## Why\n\nPreserve source.\n",
        ),
        (
            "openspec/changes/change/tasks.md",
            "# Tasks\n\n- [ ] Verify\n",
        ),
        (
            ".scratch/effort/map.md",
            "## Destination\nPreserve local Wayfinder source.\n",
        ),
        (
            ".scratch/effort/issues/01-question.md",
            "Type: research\n\n## Question\nIs source immutable?\n",
        ),
    ];
    for (relative_path, contents) in files {
        let path = root.path().join(relative_path);
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(path, contents).expect("fixture source");
    }
    let before = files
        .iter()
        .map(|(relative_path, _)| {
            (
                *relative_path,
                fs::read(root.path().join(relative_path)).expect("before bytes"),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let reader = ContainedReader::approve(root.path(), 1024 * 1024).expect("reader");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());
    let index = build_index(
        &reader,
        discovered.projects,
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
    );
    let records = &index.projects[0].records;
    assert_eq!(
        records
            .iter()
            .map(|record| record.locator.format_id.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from([
            "markdown",
            "openspec",
            "planning-pattern",
            "wayfinder-local",
        ])
    );
    for record in records {
        work_record_detail(&reader, &index, record.subject_id.as_str()).expect("detail");
        work_record_handoff(&reader, &index, record.subject_id.as_str()).expect("handoff");
    }
    let after = files
        .iter()
        .map(|(relative_path, _)| {
            (
                *relative_path,
                fs::read(root.path().join(relative_path)).expect("after bytes"),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(after, before);
}

#[test]
fn local_wayfinder_detail_and_handoff_use_fresh_contained_sources_without_mutation() {
    let root = TempDir::new().expect("root");
    let effort = root.path().join(".scratch/search");
    fs::create_dir_all(effort.join("issues")).expect("effort");
    let map_path = effort.join("map.md");
    let ticket_path = effort.join("issues/01-first-question.md");
    fs::write(&map_path, "## Destination\n\n").expect("map");
    fs::write(
        &ticket_path,
        "Type: research\n\n## Question\nWhat should ship first?\n",
    )
    .expect("ticket");
    let reader = ContainedReader::approve(root.path(), 1024 * 1024).expect("reader");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());
    let index = build_index(
        &reader,
        discovered.projects,
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
    );
    let record = index.projects[0]
        .records
        .iter()
        .find(|record| record.locator.format_id == "wayfinder-local")
        .expect("Wayfinder record");
    assert_eq!(record.locator.adapter_record_key, ".scratch/search");
    assert_eq!(record.sources.len(), 2);
    assert!(record.facts.iter().any(|fact| {
        fact.key == "wayfinder.frontier.count" && fact.value == backstage_core::FactValue::Count(1)
    }));
    assert!(
        record
            .warnings
            .iter()
            .any(|warning| warning.code == "wayfinder_map_section_empty")
    );

    fs::write(&map_path, "## Destination\nShip local search.\n").expect("fresh map");
    let fresh_ticket = "Type: research\nStatus: claimed\n\n## Question\nWhat should ship first?\n";
    fs::write(&ticket_path, fresh_ticket).expect("fresh ticket");
    let detail =
        work_record_detail(&reader, &index, record.subject_id.as_str()).expect("Wayfinder detail");
    let handoff = work_record_handoff(&reader, &index, record.subject_id.as_str())
        .expect("Wayfinder handoff");

    assert!(detail.record.facts.iter().any(|fact| {
        fact.key == "wayfinder.frontier.count" && fact.value == backstage_core::FactValue::Count(0)
    }));
    assert!(
        detail
            .record
            .warnings
            .iter()
            .all(|warning| warning.code != "wayfinder_map_section_empty")
    );
    assert_eq!(
        detail
            .capabilities
            .iter()
            .map(|view| view.capability.id.as_str())
            .collect::<Vec<_>>(),
        vec!["overview", "questions", "source"]
    );
    assert!(detail.capabilities[1].blocks.iter().any(|block| matches!(
        block,
        StructuredBlock::ItemCollection { items, .. }
            if items.iter().any(|item| item.facts.iter().any(|fact| {
                fact.key == "wayfinder.ticket.status"
                    && fact.value == backstage_core::FactValue::Text("claimed".to_owned())
            }))
    )));
    assert_eq!(
        handoff.handoff.primary_source_path.as_deref(),
        Some(".scratch/search/map.md")
    );
    assert!(
        handoff
            .handoff
            .continuation_prompt
            .contains("Frontier: None")
    );
    assert_eq!(
        fs::read_to_string(&ticket_path).expect("unchanged ticket"),
        fresh_ticket
    );
    assert_eq!(
        fs::read_to_string(&map_path).expect("unchanged map"),
        "## Destination\nShip local search.\n"
    );
}

#[test]
fn local_wayfinder_keeps_partial_safe_sources_when_ticket_members_are_oversized_or_non_utf8() {
    let root = TempDir::new().expect("root");
    let effort = root.path().join(".scratch/partial/issues");
    fs::create_dir_all(&effort).expect("effort");
    fs::write(
        root.path().join(".scratch/partial/map.md"),
        "## Destination\nKeep partial source readable.\n",
    )
    .expect("map");
    fs::write(effort.join("01-oversized.md"), vec![b'x'; 512]).expect("oversized ticket");
    fs::write(
        effort.join("02-non-utf8.md"),
        [
            b'T', b'y', b'p', b'e', b':', b' ', b't', b'a', b's', b'k', b'\n', 0xff,
        ],
    )
    .expect("non-UTF-8 ticket");
    let reader = ContainedReader::approve(root.path(), 128).expect("reader");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());
    let index = build_index(
        &reader,
        discovered.projects,
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
    );
    let record = index.projects[0]
        .records
        .iter()
        .find(|record| record.locator.format_id == "wayfinder-local")
        .expect("partial Wayfinder record");

    assert_eq!(record.sources.len(), 3);
    assert!(record.fingerprint.is_none());
    assert!(record.warnings.iter().any(|warning| {
        warning.source_path.as_deref() == Some(".scratch/partial/issues/01-oversized.md")
    }));
    assert!(record.warnings.iter().any(|warning| {
        warning.code == "wayfinder_ticket_not_utf8"
            && warning.source_path.as_deref() == Some(".scratch/partial/issues/02-non-utf8.md")
    }));
    assert!(
        record
            .facts
            .iter()
            .all(|fact| fact.key != "wayfinder.frontier.count")
    );
    let detail = work_record_detail(&reader, &index, record.subject_id.as_str())
        .expect("partial Wayfinder detail");
    let source = detail
        .capabilities
        .iter()
        .find(|view| view.capability.id == "source")
        .expect("Source view");
    assert!(source.blocks.iter().any(|block| matches!(
        block,
        StructuredBlock::MarkdownSection { source, .. }
            if source.relative_path == ".scratch/partial/map.md"
    )));
    assert!(source.blocks.iter().any(|block| matches!(
        block,
        StructuredBlock::Warning { warning, .. }
            if warning.source_path.as_deref()
                == Some(".scratch/partial/issues/01-oversized.md")
    )));
    let handoff = work_record_handoff(&reader, &index, record.subject_id.as_str())
        .expect("partial Wayfinder handoff");
    assert!(
        handoff
            .handoff
            .continuation_prompt
            .contains("Frontier: Unavailable")
    );
}

#[test]
fn remote_wayfinder_links_and_similar_map_names_do_not_create_local_wayfinder_records() {
    let root = TempDir::new().expect("root");
    fs::write(
        root.path().join("map.md"),
        "[Remote map](https://github.com/example/repo/issues/1)\n",
    )
    .expect("remote link");
    let reader = ContainedReader::approve(root.path(), 1024 * 1024).expect("reader");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());
    let index = build_index(
        &reader,
        discovered.projects,
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
    );

    assert!(
        index.projects[0]
            .records
            .iter()
            .all(|record| record.locator.format_id != "wayfinder-local")
    );
    assert!(
        index.projects[0]
            .records
            .iter()
            .any(|record| record.locator.format_id == "markdown")
    );
}

#[test]
fn neutral_detail_keeps_readable_members_when_one_member_fails() {
    let fixture = FixtureRepo::open_spec();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());
    let index = build_index(
        &reader,
        discovered.projects,
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
    );
    let record = index.projects[0]
        .records
        .iter()
        .find(|record| record.locator.format_id == "openspec")
        .expect("OpenSpec record");
    fs::remove_file(
        fixture
            .path()
            .join("openspec/changes/ship-search/design.md"),
    )
    .expect("remove one member after indexing");

    let detail = work_record_detail(&reader, &index, record.subject_id.as_str())
        .expect("partial neutral detail");
    let source = detail
        .capabilities
        .iter()
        .find(|view| view.capability.id == "source")
        .expect("Source capability");

    assert!(source.blocks.iter().any(|block| matches!(
        block,
        StructuredBlock::MarkdownSection { source, .. }
            if source.relative_path.ends_with("proposal.md")
    )));
    assert!(source.blocks.iter().any(|block| matches!(
        block,
        StructuredBlock::Warning { warning, .. }
            if warning.source_path.as_deref()
                == Some("openspec/changes/ship-search/design.md")
    )));
    assert!(detail.fingerprint.is_none());
}

#[test]
fn frontend_catalog_timestamps_are_serialized_as_decimal_strings() {
    let fixture = FixtureRepo::open_spec();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());
    let index = build_index(
        &reader,
        discovered.projects,
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
    );
    let document = index.projects[0]
        .markdown_documents
        .iter()
        .find(|document| document.relative_path == "README.md")
        .expect("Markdown document");
    let markdown = markdown_detail(&reader, &index, &document.id).expect("Markdown detail");
    let artifact_id = index.projects[0].bundles[0].bundle.members[0].id.clone();
    let artifact = artifact_detail(&reader, &index, &artifact_id).expect("artifact detail");

    for payload in [
        serde_json::to_value(document).expect("serialize Markdown document"),
        serde_json::to_value(&index.projects[0].bundles[0]).expect("serialize indexed bundle"),
        serde_json::to_value(markdown).expect("serialize Markdown detail"),
        serde_json::to_value(artifact).expect("serialize artifact detail"),
    ] {
        assert!(
            payload["sourceModifiedUnixNanos"].is_string(),
            "timestamp was not a string: {payload}"
        );
    }
}

#[test]
fn configured_patterns_match_normalized_project_relative_markdown_and_deduplicate_matches() {
    let fixture = FixtureRepo::open_spec();
    fs::create_dir_all(fixture.path().join("docs/plans")).expect("planning directory");
    fs::write(
        fixture.path().join("docs/plans/launch.md"),
        "# Launch plan\n",
    )
    .expect("nested plan");
    fs::write(
        fixture.path().join("docs/plans/ignored.txt"),
        "not Markdown\n",
    )
    .expect("non Markdown");
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());
    let exact = PlanningPattern::custom("^docs/plans/launch\\.md$", 9).expect("exact pattern");
    let broad = PlanningPattern::custom("^docs/plans/.*", 0).expect("broad pattern");

    let first = build_index_with_patterns(
        &reader,
        discovered.projects.clone(),
        1,
        7,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings.clone(),
        &[broad.clone(), exact.clone()],
    );
    let second = build_index_with_patterns(
        &reader,
        discovered.projects,
        2,
        7,
        "2026-08-13T12:01:00Z".to_owned(),
        discovered.warnings,
        &[exact.clone(), broad.clone()],
    );

    assert_eq!(first.configuration_revision, 7);
    let candidates = first.projects[0]
        .bundles
        .iter()
        .filter(|bundle| bundle.bundle.kind == BundleKind::PossibleArtifact)
        .collect::<Vec<_>>();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].bundle.members.len(), 1);
    assert_eq!(
        candidates[0].bundle.members[0].relative_path,
        "docs/plans/launch.md"
    );
    let evidence = &candidates[0].bundle.members[0].evidence;
    assert!(evidence.contains(exact.id()));
    assert!(evidence.contains(exact.expression()));
    assert!(evidence.contains(broad.id()));
    assert!(evidence.contains(broad.expression()));
    let second_candidate = second.projects[0]
        .bundles
        .iter()
        .find(|bundle| bundle.bundle.kind == BundleKind::PossibleArtifact)
        .expect("second candidate");
    assert_eq!(candidates[0].bundle.id, second_candidate.bundle.id);
    assert_eq!(
        candidates[0].bundle.recognition,
        second_candidate.bundle.recognition
    );
}

#[test]
fn empty_planning_patterns_disable_candidates_without_affecting_openspec_or_markdown() {
    let fixture = FixtureRepo::open_spec();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());

    let index = build_index_with_patterns(
        &reader,
        discovered.projects,
        1,
        4,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
        &[],
    );

    assert!(
        index.projects[0]
            .bundles
            .iter()
            .all(|bundle| bundle.bundle.kind != BundleKind::PossibleArtifact)
    );
    assert!(index.projects[0].bundles.iter().any(|bundle| {
        bundle.bundle.name == "ship-search" && bundle.bundle.kind == BundleKind::OpenSpecChange
    }));
    assert!(
        index.projects[0]
            .markdown_documents
            .iter()
            .any(|document| { document.relative_path == "PLAN.md" })
    );
}

#[test]
fn broad_planning_pattern_does_not_duplicate_supported_openspec_members() {
    let fixture = FixtureRepo::open_spec();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());
    let broad = PlanningPattern::custom(".*", 0).expect("broad pattern");

    let index = build_index_with_patterns(
        &reader,
        discovered.projects,
        1,
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
        &[broad],
    );

    let openspec_paths = index.projects[0]
        .bundles
        .iter()
        .filter(|bundle| bundle.bundle.kind == BundleKind::OpenSpecChange)
        .flat_map(|bundle| &bundle.bundle.members)
        .map(|member| member.relative_path.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        index.projects[0]
            .bundles
            .iter()
            .filter(|bundle| bundle.bundle.kind == BundleKind::PossibleArtifact)
            .flat_map(|bundle| &bundle.bundle.members)
            .all(|member| !openspec_paths.contains(member.relative_path.as_str()))
    );
}

#[test]
fn bundle_recency_uses_recent_observed_member_even_when_content_is_oversized() {
    let root = TempDir::new().expect("root tempdir");
    let change = root.path().join("openspec/changes/observed-recency");
    fs::create_dir_all(&change).expect("change directory");
    let proposal = change.join("proposal.md");
    let tasks = change.join("tasks.md");
    fs::write(&proposal, "# Older proposal\n").expect("proposal");
    fs::write(&tasks, vec![b'x'; 256]).expect("oversized tasks");
    let old = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
    let recent = old + std::time::Duration::from_secs(60);
    fs::File::open(&proposal)
        .expect("open proposal")
        .set_times(std::fs::FileTimes::new().set_modified(old))
        .expect("set proposal time");
    fs::File::open(&tasks)
        .expect("open tasks")
        .set_times(std::fs::FileTimes::new().set_modified(recent))
        .expect("set tasks time");
    let reader = ContainedReader::approve(root.path(), 64).expect("approve root");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());

    let index = build_index(
        &reader,
        discovered.projects,
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
    );
    let project = &index.projects[0];
    let proposal_modified = project
        .markdown_documents
        .iter()
        .find(|document| document.relative_path.ends_with("proposal.md"))
        .and_then(|document| document.source_modified_unix_nanos)
        .expect("proposal observation");
    let tasks_modified = project
        .markdown_documents
        .iter()
        .find(|document| document.relative_path.ends_with("tasks.md"))
        .and_then(|document| document.source_modified_unix_nanos)
        .expect("tasks observation");
    let bundle = project.bundles.first().expect("OpenSpec bundle");

    assert!(tasks_modified > proposal_modified);
    assert_eq!(bundle.source_modified_unix_nanos, Some(tasks_modified));
    assert!(
        bundle
            .warnings
            .iter()
            .any(|warning| warning.contains("above the 64 byte limit"))
    );
}

#[test]
fn ordinary_markdown_detail_uses_indexed_identity_and_a_fresh_snapshot() {
    let fixture = FixtureRepo::open_spec();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());
    let index = build_index(
        &reader,
        discovered.projects,
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
    );
    let document = index.projects[0]
        .markdown_documents
        .iter()
        .find(|document| document.relative_path == "README.md")
        .expect("README document");

    let first = markdown_detail(&reader, &index, &document.id).expect("Markdown detail");

    assert_eq!(first.document_id, document.id);
    assert_eq!(first.relative_path, "README.md");
    assert_eq!(first.markdown, "# Fixture project\n");
    let serialized = serde_json::to_value(&first).expect("serialize detail");
    assert!(serialized.get("bundleId").is_none());
    assert!(serialized.get("progress").is_none());
    assert!(serialized.get("fingerprint").is_none());

    fs::write(fixture.path().join("README.md"), "# Externally changed\n")
        .expect("change source externally");
    let changed_manifest = fixture.manifest();
    let refreshed = markdown_detail(&reader, &index, &document.id).expect("fresh detail");
    assert_eq!(refreshed.markdown, "# Externally changed\n");
    fixture.assert_unchanged(&changed_manifest);

    fs::remove_file(fixture.path().join("README.md")).expect("remove source externally");
    assert!(matches!(
        markdown_detail(&reader, &index, &document.id),
        Err(CatalogError::Read(_))
    ));
    assert!(matches!(
        markdown_detail(&reader, &index, "document_missing"),
        Err(CatalogError::NotFound)
    ));
}

#[test]
fn markdown_path_handoff_uses_the_current_index_without_repository_mutation() {
    let fixture = FixtureRepo::open_spec();
    let before = fixture.manifest();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());
    let index = build_index(
        &reader,
        discovered.projects,
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
    );
    let document = index.projects[0]
        .markdown_documents
        .iter()
        .find(|document| document.relative_path == "README.md")
        .expect("README document");

    let path = derive_markdown_path(&reader, &index, &document.id).expect("derive Markdown path");

    assert_eq!(
        path,
        fs::canonicalize(fixture.path().join("README.md"))
            .expect("canonical README")
            .to_string_lossy()
    );
    assert_eq!(
        derive_markdown_path(&reader, &index, "document_missing")
            .expect_err("missing document")
            .message,
        "indexed Markdown ID was not found in the current index"
    );
    fixture.assert_unchanged(&before);

    fs::write(fixture.path().join("README.md"), "# Safely changed\n")
        .expect("change Markdown content");
    let changed_manifest = fixture.manifest();
    assert_eq!(
        derive_markdown_path(&reader, &index, &document.id).expect("derive changed Markdown path"),
        path
    );
    fixture.assert_unchanged(&changed_manifest);
}

#[cfg(unix)]
#[test]
fn markdown_path_handoff_canonicalizes_a_contained_symlink() {
    use std::os::unix::fs::symlink;

    let fixture = FixtureRepo::open_spec();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());
    let index = build_index(
        &reader,
        discovered.projects,
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
    );
    let document = index.projects[0]
        .markdown_documents
        .iter()
        .find(|document| document.relative_path == "README.md")
        .expect("README document");
    fs::remove_file(fixture.path().join("README.md")).expect("remove indexed source");
    symlink("PLAN.md", fixture.path().join("README.md")).expect("replace with contained symlink");
    let changed_manifest = fixture.manifest();

    let path =
        derive_markdown_path(&reader, &index, &document.id).expect("derive contained target path");

    assert_eq!(
        path,
        fs::canonicalize(fixture.path().join("PLAN.md"))
            .expect("canonical target")
            .to_string_lossy()
    );
    fixture.assert_unchanged(&changed_manifest);
}

#[cfg(unix)]
#[test]
fn ordinary_markdown_path_rejects_a_source_replaced_by_an_escaping_symlink() {
    use std::os::unix::fs::symlink;

    let fixture = FixtureRepo::open_spec();
    let outside = TempDir::new().expect("outside tempdir");
    let outside_file = outside.path().join("outside.md");
    fs::write(&outside_file, "# Outside\n").expect("write outside file");
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());
    let index = build_index(
        &reader,
        discovered.projects,
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
    );
    let document = index.projects[0]
        .markdown_documents
        .iter()
        .find(|document| document.relative_path == "README.md")
        .expect("README document");
    fs::remove_file(fixture.path().join("README.md")).expect("remove indexed source");
    symlink(&outside_file, fixture.path().join("README.md")).expect("replace with symlink");
    let changed_manifest = fixture.manifest();

    let error =
        derive_markdown_path(&reader, &index, &document.id).expect_err("reject symlink escape");

    assert_eq!(error.code, "operation_failed");
    fixture.assert_unchanged(&changed_manifest);
}

#[test]
fn artifact_scan_budget_starts_at_each_project_root() {
    let root = TempDir::new().expect("root tempdir");
    let project = root.path().join("project");
    fs::create_dir_all(project.join("openspec/changes/scoped-scan")).expect("create OpenSpec");
    fs::write(
        project.join("openspec/changes/scoped-scan/tasks.md"),
        "# Tasks\n\n- [ ] Keep the scan project-scoped\n",
    )
    .expect("write tasks");
    let project = fs::canonicalize(project).expect("canonical project path");
    let reader = ContainedReader::approve(root.path(), 1024 * 1024).expect("approve root");
    let policy = ScanPolicy {
        max_entries: 4,
        ..ScanPolicy::default()
    };

    let index = build_index_controlled(
        &reader,
        vec![ProjectCandidate {
            id: "project_1".to_owned(),
            name: "project".to_owned(),
            root_path: project.to_string_lossy().into_owned(),
            git: None,
        }],
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        vec![],
        &policy,
        &CancellationToken::new(),
    );

    assert_eq!(index.projects[0].bundles.len(), 1);
    assert_eq!(index.projects[0].bundles[0].bundle.name, "scoped-scan");
}

#[test]
fn catalog_timeout_publishes_one_bounded_partial_warning() {
    let root = TempDir::new().expect("root tempdir");
    fs::write(root.path().join("PLAN.md"), "# Plan\n").expect("planning file");
    let project_root = fs::canonicalize(root.path()).expect("canonical project path");
    let reader = ContainedReader::approve(root.path(), 1024 * 1024).expect("approve root");
    let policy = ScanPolicy {
        timeout_ms: 0,
        ..ScanPolicy::default()
    };

    let index = build_index_controlled(
        &reader,
        vec![ProjectCandidate {
            id: "project_timeout".to_owned(),
            name: "timeout".to_owned(),
            root_path: project_root.to_string_lossy().into_owned(),
            git: None,
        }],
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        vec![],
        &policy,
        &CancellationToken::new(),
    );

    let warnings = index
        .warnings
        .iter()
        .filter(|warning| warning.code == "artifact_index_timeout")
        .collect::<Vec<_>>();
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].message.contains("partial"));
}

#[test]
fn catalog_cancellation_publishes_one_bounded_partial_warning() {
    let root = TempDir::new().expect("root tempdir");
    fs::write(root.path().join("PLAN.md"), "# Plan\n").expect("planning file");
    let project_root = fs::canonicalize(root.path()).expect("canonical project path");
    let reader = ContainedReader::approve(root.path(), 1024 * 1024).expect("approve root");
    let cancellation = CancellationToken::new();
    cancellation.cancel();

    let index = build_index_controlled(
        &reader,
        vec![ProjectCandidate {
            id: "project_cancelled".to_owned(),
            name: "cancelled".to_owned(),
            root_path: project_root.to_string_lossy().into_owned(),
            git: None,
        }],
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        vec![],
        &ScanPolicy::default(),
        &cancellation,
    );

    let warnings = index
        .warnings
        .iter()
        .filter(|warning| warning.code == "artifact_index_cancelled")
        .collect::<Vec<_>>();
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].message.contains("partial"));
}

#[test]
fn nested_project_neutral_capture_uses_project_relative_paths() {
    let root = TempDir::new().expect("root tempdir");
    let project = root.path().join("nested-project");
    let change = project.join("openspec/changes/nested-change");
    fs::create_dir_all(&change).expect("create OpenSpec change");
    fs::write(
        change.join("proposal.md"),
        "# Proposal\n\n## Why\nNested.\n",
    )
    .expect("write proposal");
    fs::write(
        change.join("tasks.md"),
        "# Tasks\n\n- [ ] Keep nested projects readable\n",
    )
    .expect("write tasks");
    let project = fs::canonicalize(project).expect("canonical project path");
    let reader = ContainedReader::approve(root.path(), 1024 * 1024).expect("approve parent root");
    let index = build_index(
        &reader,
        vec![ProjectCandidate {
            id: "project_nested_neutral".to_owned(),
            name: "nested-project".to_owned(),
            root_path: project.to_string_lossy().into_owned(),
            git: None,
        }],
        1,
        "2026-08-15T12:00:00Z".to_owned(),
        vec![],
    );
    let record = index.projects[0]
        .records
        .iter()
        .find(|record| record.locator.format_id == "openspec")
        .expect("neutral OpenSpec record");

    assert!(record.fingerprint.is_some());
    assert!(record.facts.iter().any(|fact| {
        fact.key == "openspec.task.total_count" && fact.value == backstage_core::FactValue::Count(1)
    }));
    assert!(record.warnings.iter().all(|warning| {
        warning.code != "incomplete_source_snapshot"
            && warning.code != "openspec_progress_unavailable"
    }));

    let detail = work_record_detail(&reader, &index, record.subject_id.as_str())
        .expect("nested neutral detail");
    assert!(detail.fingerprint.is_some());
    let source = detail
        .capabilities
        .iter()
        .find(|view| view.capability.id == "source")
        .expect("source capability");
    assert!(source.blocks.iter().any(|block| matches!(
        block,
        StructuredBlock::MarkdownSection { source, .. }
            if source.relative_path
                == "openspec/changes/nested-change/tasks.md"
    )));
}

#[test]
fn nested_project_artifact_detail_uses_project_relative_member_paths() {
    let root = TempDir::new().expect("root tempdir");
    let project = root.path().join("nested-project");
    let tasks = project.join("openspec/changes/nested-change/tasks.md");
    fs::create_dir_all(tasks.parent().expect("tasks parent")).expect("create OpenSpec");
    fs::write(&tasks, "# Tasks\n\n- [ ] Keep nested projects readable\n").expect("write tasks");
    let project = fs::canonicalize(project).expect("canonical project path");
    let reader = ContainedReader::approve(root.path(), 1024 * 1024).expect("approve parent root");
    let index = build_index(
        &reader,
        vec![ProjectCandidate {
            id: "project_nested".to_owned(),
            name: "nested-project".to_owned(),
            root_path: project.to_string_lossy().into_owned(),
            git: None,
        }],
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        vec![],
    );
    let task = index.projects[0].bundles[0]
        .bundle
        .members
        .iter()
        .find(|member| member.relative_path.ends_with("tasks.md"))
        .expect("tasks member");

    let detail = artifact_detail(&reader, &index, &task.id).expect("nested artifact detail");

    assert_eq!(
        detail.relative_path,
        "openspec/changes/nested-change/tasks.md"
    );
    assert!(detail.markdown.contains("Keep nested projects readable"));
    let view = detail.open_spec_view.expect("structured OpenSpec view");
    assert_eq!(view.task_groups.len(), 1);
    assert_eq!(
        view.task_groups[0].tasks[0].text,
        "Keep nested projects readable"
    );
}

#[test]
fn planning_candidate_does_not_receive_a_structured_openspec_view() {
    let fixture = FixtureRepo::open_spec();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());
    let index = build_index(
        &reader,
        discovered.projects,
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
    );
    let candidate = index.projects[0]
        .bundles
        .iter()
        .find(|bundle| bundle.bundle.kind == BundleKind::PossibleArtifact)
        .expect("planning candidate");

    let detail = artifact_detail(&reader, &index, &candidate.bundle.members[0].id)
        .expect("candidate detail");

    assert!(detail.open_spec_view.is_none());
}

#[test]
fn handoffs_use_backend_normalized_paths_and_preserve_the_repository() {
    let fixture = FixtureRepo::open_spec();
    let before = fixture.manifest();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());
    let index = build_index(
        &reader,
        discovered.projects,
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
    );
    let task = index.projects[0]
        .bundles
        .iter()
        .find(|bundle| bundle.bundle.name == "ship-search")
        .expect("OpenSpec bundle")
        .bundle
        .members
        .iter()
        .find(|member| member.relative_path.ends_with("tasks.md"))
        .expect("tasks member");

    let path = derive_artifact_path(&reader, &index, &task.id).expect("derived path");
    let prompt = derive_continuation_prompt(&reader, &index, &task.id).expect("derived prompt");

    assert!(path.starts_with(reader.root().to_string_lossy().as_ref()));
    assert!(path.ends_with("openspec/changes/ship-search/tasks.md"));
    assert!(prompt.contains(&path));
    assert!(prompt.contains("1 of 2 tasks complete"));
    fixture.assert_unchanged(&before);
}

#[test]
fn archived_openspec_uses_distinct_identity_and_the_current_structured_reader() {
    let fixture = FixtureRepo::open_spec();
    let archive = fixture
        .path()
        .join("openspec/changes/archive/2026-08-13-ship-search");
    fs::create_dir_all(archive.join("specs/search")).expect("create archive");
    fs::write(
        archive.join("proposal.md"),
        "# Archived proposal\n\n## Why\nHistory.\n",
    )
    .expect("write archived proposal");
    fs::write(
        archive.join("tasks.md"),
        "# Tasks\n\n- [x] Archived task\n- [ ] Historical open task\n",
    )
    .expect("write archived tasks");
    fs::write(archive.join("specs/search/spec.md"), "# Archived spec\n")
        .expect("write archived spec");
    let malformed = fixture
        .path()
        .join("openspec/changes/archive/not-dated-change");
    fs::create_dir_all(&malformed).expect("create malformed archive");
    fs::write(malformed.join("tasks.md"), "- [x] Retained task\n")
        .expect("write malformed archived tasks");
    fs::create_dir_all(archive.join("nested")).expect("create unsupported nested directory");
    fs::write(
        archive.join("nested/design.md"),
        "# Not a supported member\n",
    )
    .expect("write unsupported nested member");

    let before = fixture.manifest();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());
    let index = build_index(
        &reader,
        discovered.projects,
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
    );
    let openspec = index.projects[0]
        .bundles
        .iter()
        .filter(|bundle| bundle.bundle.kind == BundleKind::OpenSpecChange)
        .collect::<Vec<_>>();

    assert_eq!(openspec.len(), 3);
    let current = openspec
        .iter()
        .find(|bundle| bundle.bundle.custody == Some(OpenSpecCustody::Current))
        .expect("current copy");
    let archived = openspec
        .iter()
        .find(|bundle| {
            bundle.bundle.custody
                == Some(OpenSpecCustody::Archived {
                    archived_on: Some("2026-08-13".to_owned()),
                })
        })
        .expect("dated archive");
    let malformed = openspec
        .iter()
        .find(|bundle| bundle.bundle.name == "not-dated-change")
        .expect("malformed archive");
    assert_ne!(current.bundle.id, archived.bundle.id);
    assert_eq!(archived.bundle.name, "ship-search");
    assert_eq!(
        archived.primary_status,
        Some(OpenSpecPrimaryStatus::Archived)
    );
    assert_eq!(
        malformed.bundle.custody,
        Some(OpenSpecCustody::Archived { archived_on: None })
    );
    assert_eq!(
        malformed.primary_status,
        Some(OpenSpecPrimaryStatus::Archived)
    );
    assert!(
        !archived
            .bundle
            .members
            .iter()
            .any(|member| { member.relative_path.ends_with("nested/design.md") })
    );
    assert!(index.projects[0].markdown_documents.iter().any(|document| {
        document.relative_path == "openspec/changes/archive/2026-08-13-ship-search/nested/design.md"
    }));

    let task = archived
        .bundle
        .members
        .iter()
        .find(|member| member.relative_path.ends_with("tasks.md"))
        .expect("archived tasks member");
    let detail = artifact_detail(&reader, &index, &task.id).expect("archived detail");
    assert_eq!(
        detail.custody,
        Some(OpenSpecCustody::Archived {
            archived_on: Some("2026-08-13".to_owned()),
        })
    );
    assert_eq!(detail.primary_status, Some(OpenSpecPrimaryStatus::Archived));
    assert_eq!(
        detail
            .open_spec_view
            .as_ref()
            .expect("structured archived view")
            .task_groups[0]
            .tasks
            .len(),
        2
    );
    let prompt = derive_continuation_prompt(&reader, &index, &task.id)
        .expect("archived continuation prompt");
    assert!(prompt.contains("Custody: Archived on 2026-08-13"));
    assert!(prompt.contains("openspec/changes/archive/2026-08-13-ship-search/tasks.md"));
    fixture.assert_unchanged(&before);
}

#[test]
fn unreadable_tasks_do_not_hide_readable_openspec_overview_or_source() {
    let root = TempDir::new().expect("root tempdir");
    let change = root.path().join("openspec/changes/tolerant-detail");
    fs::create_dir_all(&change).expect("change directory");
    fs::write(
        change.join("proposal.md"),
        "# Proposal\n\n## Why\nKeep readable sources available.\n",
    )
    .expect("proposal");
    fs::write(change.join("tasks.md"), vec![b'x'; 256]).expect("oversized tasks");
    let reader = ContainedReader::approve(root.path(), 96).expect("approve root");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());
    let index = build_index(
        &reader,
        discovered.projects,
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
    );
    let bundle = &index.projects[0].bundles[0];
    let proposal = bundle
        .bundle
        .members
        .iter()
        .find(|member| member.relative_path.ends_with("proposal.md"))
        .expect("proposal member");
    let tasks = bundle
        .bundle
        .members
        .iter()
        .find(|member| member.relative_path.ends_with("tasks.md"))
        .expect("tasks member");

    let detail = artifact_detail(&reader, &index, &proposal.id).expect("readable proposal detail");

    assert!(detail.markdown.contains("Keep readable sources available"));
    assert!(detail.fingerprint.is_none());
    let OpenSpecProgress::Unavailable(fallback) = &detail.progress else {
        panic!("progress should be unavailable")
    };
    assert!(
        fallback
            .warnings
            .iter()
            .any(|warning| warning.message.contains("tasks.md"))
    );
    let view = detail.open_spec_view.expect("readable overview");
    assert_eq!(view.overview.len(), 1);
    assert!(view.overview[0].markdown.contains("readable sources"));
    assert!(
        detail
            .warnings
            .iter()
            .any(|warning| warning.contains("tasks.md") && warning.contains("96 byte limit"))
    );

    let error = artifact_detail(&reader, &index, &tasks.id)
        .expect_err("selected unreadable member must return a focused error");
    assert!(error.to_string().contains("Selected artifact"));
    assert!(error.to_string().contains("tasks.md"));
}

#[test]
fn non_utf8_tasks_are_a_warning_for_other_members_and_a_focused_error_when_selected() {
    let root = TempDir::new().expect("root tempdir");
    let change = root.path().join("openspec/changes/non-utf8-detail");
    fs::create_dir_all(&change).expect("change directory");
    fs::write(
        change.join("proposal.md"),
        "# Proposal\n\n## Why\nPreserve this overview.\n",
    )
    .expect("proposal");
    fs::write(change.join("tasks.md"), [0xff, 0xfe, 0xfd]).expect("non-UTF-8 tasks");
    let reader = ContainedReader::approve(root.path(), 1024).expect("approve root");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());
    let index = build_index(
        &reader,
        discovered.projects,
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
    );
    let bundle = &index.projects[0].bundles[0];
    let proposal = bundle
        .bundle
        .members
        .iter()
        .find(|member| member.relative_path.ends_with("proposal.md"))
        .expect("proposal member");
    let tasks = bundle
        .bundle
        .members
        .iter()
        .find(|member| member.relative_path.ends_with("tasks.md"))
        .expect("tasks member");

    let detail = artifact_detail(&reader, &index, &proposal.id).expect("readable proposal detail");

    assert!(detail.fingerprint.is_some());
    let OpenSpecProgress::Unavailable(fallback) = &detail.progress else {
        panic!("progress should be unavailable")
    };
    assert!(fallback.warnings.iter().any(|warning| {
        warning.message.contains("tasks.md") && warning.message.contains("UTF-8")
    }));
    assert_eq!(
        detail
            .open_spec_view
            .expect("readable overview")
            .overview
            .len(),
        1
    );

    let error = artifact_detail(&reader, &index, &tasks.id)
        .expect_err("selected non-UTF-8 member must return a focused error");
    assert!(error.to_string().contains("Selected artifact"));
    assert!(error.to_string().contains("tasks.md"));
    assert!(error.to_string().contains("UTF-8"));
}

#[test]
fn malformed_tasks_remain_browsable_with_progress_warning() {
    let fixture = FixtureRepo::open_spec();
    fs::write(
        fixture.path().join("openspec/changes/ship-search/tasks.md"),
        "# Tasks\n\n- [~] Unsupported but readable\n",
    )
    .expect("malform tasks");
    let before = fixture.manifest();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve fixture");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());
    let index = build_index(
        &reader,
        discovered.projects,
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
    );
    let bundle = index.projects[0]
        .bundles
        .iter()
        .find(|bundle| bundle.bundle.name == "ship-search")
        .expect("OpenSpec bundle");
    let task = bundle
        .bundle
        .members
        .iter()
        .find(|member| member.relative_path.ends_with("tasks.md"))
        .expect("tasks member");

    let detail = artifact_detail(&reader, &index, &task.id).expect("browsable detail");

    assert!(detail.markdown.contains("Unsupported but readable"));
    let OpenSpecProgress::Unavailable(fallback) = detail.progress else {
        panic!("progress should be unavailable")
    };
    assert_eq!(fallback.warnings.len(), 1);
    let view = detail.open_spec_view.expect("structured OpenSpec fallback");
    assert!(view.task_groups.is_empty());
    fixture.assert_unchanged(&before);
}
