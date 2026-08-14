mod support;

use std::fs;

use backstage_app_lib::catalog::{
    CatalogError, artifact_detail, build_index, build_index_controlled, build_index_with_patterns,
    markdown_detail,
};
use backstage_app_lib::discovery::{
    CancellationToken, ProjectCandidate, ScanPolicy, discover_projects,
};
use backstage_app_lib::filesystem::ContainedReader;
use backstage_app_lib::{derive_artifact_path, derive_continuation_prompt, derive_markdown_path};
use backstage_core::{
    BundleKind, OpenSpecCustody, OpenSpecPrimaryStatus, OpenSpecProgress, PlanningPattern,
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
