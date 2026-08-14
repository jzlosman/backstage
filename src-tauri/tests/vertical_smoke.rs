mod support;

use backstage_app_lib::catalog::{artifact_detail, build_index_with_patterns, markdown_detail};
use backstage_app_lib::discovery::{CancellationToken, ScanPolicy, discover_projects};
use backstage_app_lib::filesystem::ContainedReader;
use backstage_app_lib::generation::{GenerationLimits, build_generation_snapshot};
use backstage_app_lib::launcher::{Launcher, ProcessRequest, ProcessRunner};
use backstage_app_lib::{derive_artifact_path, derive_continuation_prompt, derive_markdown_path};
use backstage_core::{
    GeneratedResult, GeneratedView, GenerationMode, OpenSpecCustody, OpenSpecPrimaryStatus,
    OpenSpecProgress, PlanningPattern, canonical_planning_patterns, generation_completed,
    sources_changed, start_generation,
};
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use support::FixtureRepo;

#[derive(Default)]
struct RecordingRunner(Mutex<Vec<ProcessRequest>>);

impl ProcessRunner for RecordingRunner {
    fn spawn(&self, request: ProcessRequest) -> Result<(), String> {
        self.0.lock().expect("runner lock").push(request);
        Ok(())
    }
}

#[test]
fn disposable_real_repository_completes_the_vertical_read_only_flow() {
    let fixture = FixtureRepo::open_spec();
    let archive = fixture
        .path()
        .join("openspec/changes/archive/2026-08-01-finished-search");
    fs::create_dir_all(&archive).expect("create archived change");
    fs::write(
        archive.join("proposal.md"),
        "# Finished search\n\n## Why\n\nPreserve the completed decision.\n",
    )
    .expect("write archived proposal");
    fs::write(archive.join("tasks.md"), "# Tasks\n\n- [x] Ship search\n")
        .expect("write archived tasks");
    fs::create_dir_all(fixture.path().join("notes")).expect("create custom planning directory");
    fs::write(
        fixture.path().join("notes/session-plan.md"),
        "# Session plan\n",
    )
    .expect("write custom planning file");
    let mut patterns = canonical_planning_patterns();
    patterns.push(
        PlanningPattern::custom(r"^notes/session-plan\.md$", patterns.len() as u32)
            .expect("custom planning pattern"),
    );
    let initial_manifest = fixture.manifest();
    let reader = ContainedReader::approve(fixture.path(), 1024 * 1024).expect("approve root");
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());
    let first_index = build_index_with_patterns(
        &reader,
        discovered.projects,
        1,
        1,
        "2026-08-13T12:00:00Z".to_owned(),
        discovered.warnings,
        &patterns,
    );
    let readme = first_index.projects[0]
        .markdown_documents
        .iter()
        .find(|document| document.relative_path == "README.md")
        .expect("indexed ordinary Markdown");
    let readme_detail = markdown_detail(&reader, &first_index, &readme.id)
        .expect("contained ordinary Markdown detail");
    assert_eq!(readme_detail.markdown, "# Fixture project\n");
    assert_eq!(
        derive_markdown_path(&reader, &first_index, &readme.id)
            .expect("contained ordinary Markdown path"),
        readme_detail.absolute_path
    );

    let bundle = first_index.projects[0]
        .bundles
        .iter()
        .find(|bundle| {
            bundle.bundle.name == "ship-search"
                && bundle.bundle.custody == Some(OpenSpecCustody::Current)
        })
        .expect("discovered current OpenSpec bundle");
    let OpenSpecProgress::Available(progress) = &bundle.progress else {
        panic!("deterministic progress should be available")
    };
    assert_eq!((progress.completed, progress.total), (1, 2));
    let archived = first_index.projects[0]
        .bundles
        .iter()
        .find(|bundle| bundle.bundle.name == "finished-search")
        .expect("discovered archived OpenSpec bundle");
    assert!(matches!(
        archived.bundle.custody,
        Some(OpenSpecCustody::Archived {
            archived_on: Some(ref date)
        }) if date == "2026-08-01"
    ));
    assert_eq!(
        archived.primary_status,
        Some(OpenSpecPrimaryStatus::Archived)
    );
    let archived_task = archived
        .bundle
        .members
        .iter()
        .find(|member| member.relative_path.ends_with("tasks.md"))
        .expect("archived tasks member");
    let archived_detail =
        artifact_detail(&reader, &first_index, &archived_task.id).expect("archived detail");
    assert!(archived_detail.open_spec_view.is_some());
    assert_eq!(
        archived_detail.primary_status,
        Some(OpenSpecPrimaryStatus::Archived)
    );
    assert!(first_index.projects[0].bundles.iter().any(|bundle| {
        bundle.bundle.name == "session-plan.md"
            && bundle.bundle.kind == backstage_core::BundleKind::PossibleArtifact
    }));
    let task = bundle
        .bundle
        .members
        .iter()
        .find(|member| member.relative_path.ends_with("tasks.md"))
        .expect("tasks member");
    let detail = artifact_detail(&reader, &first_index, &task.id).expect("Markdown detail");
    assert!(detail.markdown.contains("Filter bundles"));

    let generation_paths = bundle
        .bundle
        .members
        .iter()
        .map(|member| Path::new(&detail.project_root).join(&member.relative_path))
        .collect::<Vec<_>>();
    let generation_snapshot = build_generation_snapshot(
        &reader,
        &generation_paths,
        GenerationMode::Summary,
        "summary-v1",
        &GenerationLimits::default(),
    )
    .expect("bounded Summary snapshot");
    let request = start_generation(
        GeneratedView::NeverGenerated,
        "smoke-generation",
        generation_snapshot.source_fingerprint.clone(),
    );
    let generated = GeneratedResult {
        text: "Fixture Summary".to_owned(),
        mode: GenerationMode::Summary,
        source_fingerprint: generation_snapshot.source_fingerprint.clone(),
        included_paths: generation_snapshot.included_paths.clone(),
        generated_at: "2026-08-13T12:00:01Z".to_owned(),
        model: Some("controlled-smoke-adapter".to_owned()),
        prompt_version: generation_snapshot.prompt_version.clone(),
    };
    let current = generation_completed(
        request,
        "smoke-generation",
        generated,
        &generation_snapshot.source_fingerprint,
    );
    assert!(matches!(current, GeneratedView::Current { .. }));

    let path = derive_artifact_path(&reader, &first_index, &task.id).expect("copy path");
    let prompt = derive_continuation_prompt(&reader, &first_index, &task.id).expect("copy prompt");
    assert!(prompt.contains(&path));
    let launcher_calls = RecordingRunner::default();
    Launcher::new(&launcher_calls)
        .open_terminal(Path::new(&detail.project_root))
        .expect("terminal handoff");
    assert_eq!(launcher_calls.0.lock().expect("runner lock").len(), 1);
    fixture.assert_unchanged(&initial_manifest);

    fs::write(
        fixture.path().join("openspec/changes/ship-search/tasks.md"),
        "# Tasks\n\n- [x] Parse query\n- [x] Filter bundles\n- [ ] Restore focus\n",
    )
    .expect("external source change");
    let changed_manifest = fixture.manifest();
    let discovered = discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());
    let second_index = build_index_with_patterns(
        &reader,
        discovered.projects,
        2,
        1,
        "2026-08-13T12:00:02Z".to_owned(),
        discovered.warnings,
        &patterns,
    );
    let changed_bundle = second_index.projects[0]
        .bundles
        .iter()
        .find(|bundle| {
            bundle.bundle.name == "ship-search"
                && bundle.bundle.custody == Some(OpenSpecCustody::Current)
        })
        .expect("refreshed current bundle");
    let stale = sources_changed(
        current,
        changed_bundle
            .fingerprint
            .as_ref()
            .expect("new fingerprint"),
        vec!["openspec/changes/ship-search/tasks.md".to_owned()],
    );

    assert!(matches!(stale, GeneratedView::Stale { .. }));
    fixture.assert_unchanged(&changed_manifest);
}
