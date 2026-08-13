mod support;

use std::fs;

use backstage_app_lib::catalog::{
    CatalogError, artifact_detail, build_index, build_index_controlled, markdown_detail,
};
use backstage_app_lib::discovery::{
    CancellationToken, ProjectCandidate, ScanPolicy, discover_projects,
};
use backstage_app_lib::filesystem::ContainedReader;
use backstage_app_lib::{derive_artifact_path, derive_continuation_prompt, derive_markdown_path};
use backstage_core::{BundleKind, OpenSpecProgress};
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
