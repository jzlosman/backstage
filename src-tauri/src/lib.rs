#![forbid(unsafe_code)]

pub mod api;
pub mod app_paths;
pub mod catalog;
pub mod discovery;
pub mod filesystem;
pub mod generation;
pub mod index;
pub mod launcher;
pub mod pi;
pub mod pi_jobs;
pub mod storage;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use backstage_core::{
    ApprovedRoot, GeneratedResult, GeneratedView, GenerationMode, PlanningPattern,
    PlanningPatternConfiguration, PlanningPatternError, generation_completed, generation_failed,
    previous_result, sources_changed, start_generation,
};
use parking_lot::Mutex;
use tauri::State;

pub use api::{
    ApiError, approve_root_path, derive_artifact_path, derive_continuation_prompt,
    derive_markdown_path, list_approved_roots, remove_approved_root,
};
use catalog::{ArtifactDetail, MarkdownDetail};
use discovery::{CancellationToken, DiscoveryResult, ScanPolicy, discover_projects};
use filesystem::ContainedReader;
use generation::{GenerationLimits, build_generation_snapshot, bundle_generation_paths};
use index::{CompletionDisposition, IndexSnapshot, IndexedBundle, ScanCoordinator};
use launcher::{Launcher, SystemProcessRunner};
use pi::{PiCapability, PiConfig, SystemCommandRunner, probe_pi};
use pi_jobs::{GenerationJobEvent, PiJobRunner};
use storage::{RootRemovalInventory, SqliteStore, StoreError};

pub struct RuntimeState {
    store: SqliteStore,
    scans: ScanCoordinator,
    generated: Mutex<BTreeMap<String, GeneratedView>>,
    settings_mutation: Mutex<()>,
    generated_publication: Mutex<()>,
    scan_admission: Mutex<()>,
    scan_cancellations: Mutex<BTreeMap<String, CancellationToken>>,
    pi_cancellations: Mutex<BTreeMap<String, ActivePiRequest>>,
    pi_temp: PathBuf,
}

struct ActivePiRequest {
    root_id: String,
    bundle_id: String,
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternMutation {
    pub patterns: Vec<PlanningPattern>,
    pub configuration_revision: u64,
    pub indexes: Vec<IndexSnapshot>,
    pub failed_root_ids: Vec<String>,
}

#[tauri::command]
fn list_roots(state: State<'_, RuntimeState>) -> Result<Vec<ApprovedRoot>, ApiError> {
    list_approved_roots(&state.store)
}

#[tauri::command]
fn approve_root(path: String, state: State<'_, RuntimeState>) -> Result<ApprovedRoot, ApiError> {
    let _settings = state.settings_mutation.lock();
    approve_root_path(&state.store, path)
}

#[tauri::command]
fn remove_root(
    root_id: String,
    state: State<'_, RuntimeState>,
) -> Result<RootRemovalInventory, ApiError> {
    remove_root_runtime(&root_id, state.inner())
}

#[tauri::command]
fn list_patterns(state: State<'_, RuntimeState>) -> Result<PlanningPatternConfiguration, ApiError> {
    state
        .store
        .planning_configuration()
        .map_err(pattern_store_error)
}

#[tauri::command]
fn add_pattern(
    expression: String,
    state: State<'_, RuntimeState>,
) -> Result<PatternMutation, ApiError> {
    add_pattern_runtime(&expression, state.inner())
}

#[tauri::command]
fn remove_pattern(id: String, state: State<'_, RuntimeState>) -> Result<PatternMutation, ApiError> {
    remove_pattern_runtime(&id, state.inner())
}

#[tauri::command]
fn restore_default_patterns(state: State<'_, RuntimeState>) -> Result<PatternMutation, ApiError> {
    restore_default_patterns_runtime(state.inner())
}

#[tauri::command]
fn scan_root(root_id: String, state: State<'_, RuntimeState>) -> Result<DiscoveryResult, ApiError> {
    let root = find_root(&state.store, &root_id)?;
    let configuration = state
        .store
        .planning_configuration()
        .map_err(pattern_store_error)?;
    scan_root_with_configuration(&root, &configuration, state.inner())
}

fn add_pattern_runtime(
    expression: &str,
    state: &RuntimeState,
) -> Result<PatternMutation, ApiError> {
    let _settings = state.settings_mutation.lock();
    let configuration = state
        .store
        .add_planning_pattern(expression)
        .map_err(pattern_store_error)?;
    rescan_after_pattern_mutation(configuration, state)
}

#[cfg(test)]
fn add_pattern_runtime_with_response_seam(
    expression: &str,
    state: &RuntimeState,
    at_response_seam: impl FnOnce(),
) -> Result<PatternMutation, ApiError> {
    let _settings = state.settings_mutation.lock();
    let configuration = state
        .store
        .add_planning_pattern(expression)
        .map_err(pattern_store_error)?;
    let (roots, failed_root_ids) = rescan_pattern_roots(&configuration, state)?;
    at_response_seam();
    Ok(pattern_mutation_response(
        configuration,
        &roots,
        failed_root_ids,
        state,
    ))
}

fn remove_pattern_runtime(id: &str, state: &RuntimeState) -> Result<PatternMutation, ApiError> {
    let _settings = state.settings_mutation.lock();
    let configuration = state
        .store
        .remove_planning_pattern(id)
        .map_err(pattern_store_error)?;
    rescan_after_pattern_mutation(configuration, state)
}

fn restore_default_patterns_runtime(state: &RuntimeState) -> Result<PatternMutation, ApiError> {
    let _settings = state.settings_mutation.lock();
    let configuration = state
        .store
        .restore_default_planning_patterns()
        .map_err(pattern_store_error)?;
    rescan_after_pattern_mutation(configuration, state)
}

fn rescan_after_pattern_mutation(
    configuration: PlanningPatternConfiguration,
    state: &RuntimeState,
) -> Result<PatternMutation, ApiError> {
    let (roots, failed_root_ids) = rescan_pattern_roots(&configuration, state)?;
    Ok(pattern_mutation_response(
        configuration,
        &roots,
        failed_root_ids,
        state,
    ))
}

fn rescan_pattern_roots(
    configuration: &PlanningPatternConfiguration,
    state: &RuntimeState,
) -> Result<(Vec<ApprovedRoot>, Vec<String>), ApiError> {
    let roots = state.store.list_roots().map_err(ApiError::from_error)?;
    let failed_root_ids = run_bounded_tasks(&roots, 4, |root| {
        match scan_root_with_configuration(root, configuration, state) {
            Err(_) => true,
            Ok(result) => result
                .warnings
                .iter()
                .any(|warning| warning.code == "cache_write_failed"),
        }
    })
    .into_iter()
    .map(|index| roots[index].id().to_owned())
    .collect();
    Ok((roots, failed_root_ids))
}

fn pattern_mutation_response(
    configuration: PlanningPatternConfiguration,
    roots: &[ApprovedRoot],
    failed_root_ids: Vec<String>,
    state: &RuntimeState,
) -> PatternMutation {
    let indexes = roots
        .iter()
        .filter_map(|root| {
            state
                .scans
                .current(root.id())
                .or_else(|| state.store.load_index(root.id()).ok().flatten())
        })
        .collect();
    PatternMutation {
        patterns: configuration.patterns,
        configuration_revision: configuration.revision,
        indexes,
        failed_root_ids,
    }
}

fn run_bounded_tasks<T: Sync>(
    items: &[T],
    max_concurrency: usize,
    task_failed: impl Fn(&T) -> bool + Sync,
) -> Vec<usize> {
    let next = std::sync::atomic::AtomicUsize::new(0);
    let failed = (0..items.len())
        .map(|_| std::sync::atomic::AtomicBool::new(false))
        .collect::<Vec<_>>();
    let workers = items.len().min(max_concurrency.max(1));
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(item) = items.get(index) else {
                        break;
                    };
                    failed[index].store(task_failed(item), std::sync::atomic::Ordering::Relaxed);
                }
            });
        }
    });
    failed
        .into_iter()
        .enumerate()
        .filter_map(|(index, failed)| failed.into_inner().then_some(index))
        .collect()
}

fn scan_root_with_configuration(
    root: &ApprovedRoot,
    configuration: &PlanningPatternConfiguration,
    state: &RuntimeState,
) -> Result<DiscoveryResult, ApiError> {
    scan_root_with_configuration_after_initial_check(root, configuration, state, || {})
}

fn scan_root_with_configuration_after_initial_check(
    root: &ApprovedRoot,
    configuration: &PlanningPatternConfiguration,
    state: &RuntimeState,
    after_initial_check: impl FnOnce(),
) -> Result<DiscoveryResult, ApiError> {
    find_root(&state.store, root.id())?;
    after_initial_check();
    let (permit, cancellation) = {
        let _admission = state.scan_admission.lock();
        find_root(&state.store, root.id())?;
        let permit = state
            .scans
            .begin_for_revision(root.id(), configuration.revision);
        if !permit.admitted {
            return Err(ApiError::new(
                "scan_superseded",
                "A newer scan configuration is already active",
            ));
        }
        let cancellation = CancellationToken::new();
        if let Some(previous) = state
            .scan_cancellations
            .lock()
            .insert(root.id().to_owned(), cancellation.clone())
        {
            previous.cancel();
        }
        (permit, cancellation)
    };
    let policy = ScanPolicy::default();
    let reader = match ContainedReader::approve(root.path(), policy.max_file_bytes) {
        Ok(reader) => reader,
        Err(error) => {
            state.scans.fail(&permit, error.to_string());
            clear_scan_cancellation(state, root.id(), &cancellation);
            return Err(ApiError::from_error(error));
        }
    };
    let mut discovered = discover_projects(&reader, &policy, &cancellation);
    let index = catalog::build_index_controlled_with_patterns(
        &reader,
        discovered.projects.clone(),
        permit.generation,
        configuration.revision,
        chrono::Utc::now().to_rfc3339(),
        discovered.warnings.clone(),
        &configuration.patterns,
        &policy,
        &cancellation,
    );
    if let Err(error) = find_root(&state.store, root.id()) {
        if error.code == "root_not_found" {
            state.scans.forget(root.id());
        } else {
            state.scans.fail(&permit, error.message.clone());
        }
        clear_scan_cancellation(state, root.id(), &cancellation);
        return Err(error);
    }
    if cancellation.is_cancelled() {
        discovered.cancelled = true;
        state.scans.cancel(&permit);
    } else {
        let _publication = state.generated_publication.lock();
        if state.scans.complete(&permit, index.clone()) == CompletionDisposition::Accepted
            && let Err(error) = state.store.save_index(&index)
        {
            discovered.warnings.push(discovery::ScanWarning {
                code: "cache_write_failed".to_owned(),
                path: root.path().to_owned(),
                message: format!(
                    "The new index is usable in memory but could not be cached: {error}"
                ),
            });
        }
    }
    clear_scan_cancellation(state, root.id(), &cancellation);
    Ok(discovered)
}

fn clear_scan_cancellation(state: &RuntimeState, root_id: &str, cancellation: &CancellationToken) {
    let mut cancellations = state.scan_cancellations.lock();
    if cancellations
        .get(root_id)
        .is_some_and(|active| std::sync::Arc::ptr_eq(&active.0, &cancellation.0))
    {
        cancellations.remove(root_id);
    }
}

fn remove_root_runtime(
    root_id: &str,
    state: &RuntimeState,
) -> Result<RootRemovalInventory, ApiError> {
    let _settings = state.settings_mutation.lock();
    let _admission = state.scan_admission.lock();
    let _publication = state.generated_publication.lock();
    find_root(&state.store, root_id)?;
    let retained_current = state
        .store
        .list_roots()
        .map_err(ApiError::from_error)?
        .into_iter()
        .filter(|root| root.id() != root_id)
        .filter_map(|root| state.scans.current(root.id()))
        .collect::<Vec<_>>();
    let inventory = state
        .store
        .remove_root_state_with_retained_indexes(root_id, &retained_current)
        .map_err(pattern_store_error)?;
    if let Some(cancellation) = state.scan_cancellations.lock().remove(root_id) {
        cancellation.cancel();
    }
    state.scans.forget(root_id);
    let mut cancelled_bundle_ids = std::collections::BTreeSet::new();
    state.pi_cancellations.lock().retain(|_, request| {
        if request.root_id == root_id {
            request
                .cancelled
                .store(true, std::sync::atomic::Ordering::Release);
            cancelled_bundle_ids.insert(request.bundle_id.clone());
            false
        } else {
            true
        }
    });
    let reachable = inventory
        .indexes
        .iter()
        .flat_map(|index| &index.projects)
        .flat_map(|project| &project.bundles)
        .map(|bundle| bundle.bundle.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let mut generated = state.generated.lock();
    for bundle_id in cancelled_bundle_ids {
        if !reachable.contains(bundle_id.as_str()) {
            generated.remove(&bundle_id);
        }
    }
    generated.retain(|bundle_id, _| reachable.contains(bundle_id.as_str()));
    Ok(inventory)
}

fn pattern_store_error(error: StoreError) -> ApiError {
    let code = match &error {
        StoreError::PlanningPattern(PlanningPatternError::TooManyPatterns { .. }) => {
            "planning_pattern_limit"
        }
        StoreError::PlanningPattern(_) => "planning_pattern_invalid",
        StoreError::PlanningPatternAlreadyExists(_) => "planning_pattern_exists",
        StoreError::PlanningPatternNotFound(_) => "planning_pattern_not_found",
        StoreError::RootNotFound(_) => "root_not_found",
        _ => "operation_failed",
    };
    ApiError::new(code, error.to_string())
}

#[tauri::command]
fn cancel_scan(root_id: String, state: State<'_, RuntimeState>) -> bool {
    state
        .scan_cancellations
        .lock()
        .get(&root_id)
        .is_some_and(|token| {
            token.cancel();
            true
        })
}

#[tauri::command]
fn get_index(
    root_id: String,
    state: State<'_, RuntimeState>,
) -> Result<Option<IndexSnapshot>, ApiError> {
    if let Some(current) = state.scans.current(&root_id) {
        return Ok(Some(current));
    }
    let cached = state
        .store
        .load_index(&root_id)
        .map_err(ApiError::from_error)?;
    if let Some(snapshot) = cached.clone() {
        let _publication = state.generated_publication.lock();
        state.scans.hydrate(snapshot);
    }
    Ok(cached)
}

#[tauri::command]
fn get_artifact_detail(
    root_id: String,
    artifact_id: String,
    state: State<'_, RuntimeState>,
) -> Result<ArtifactDetail, ApiError> {
    let root = find_root(&state.store, &root_id)?;
    let index = state
        .scans
        .current(&root_id)
        .or_else(|| state.store.load_index(&root_id).ok().flatten())
        .ok_or_else(|| ApiError::new("index_unavailable", "No usable index is available"))?;
    let reader = ContainedReader::approve(root.path(), ScanPolicy::default().max_file_bytes)
        .map_err(ApiError::from_error)?;
    catalog::artifact_detail(&reader, &index, &artifact_id).map_err(ApiError::from_error)
}

#[tauri::command]
fn get_markdown_detail(
    root_id: String,
    document_id: String,
    state: State<'_, RuntimeState>,
) -> Result<MarkdownDetail, ApiError> {
    let root = find_root(&state.store, &root_id)?;
    let index = state
        .scans
        .current(&root_id)
        .or_else(|| state.store.load_index(&root_id).ok().flatten())
        .ok_or_else(|| ApiError::new("index_unavailable", "No usable index is available"))?;
    let reader = ContainedReader::approve(root.path(), ScanPolicy::default().max_file_bytes)
        .map_err(ApiError::from_error)?;
    catalog::markdown_detail(&reader, &index, &document_id).map_err(ApiError::from_error)
}

#[tauri::command]
fn copy_artifact_path(
    root_id: String,
    artifact_id: String,
    state: State<'_, RuntimeState>,
) -> Result<String, ApiError> {
    with_artifact_context(&root_id, &state, |reader, index| {
        derive_artifact_path(reader, index, &artifact_id)
    })
}

#[tauri::command]
fn copy_markdown_path(
    root_id: String,
    document_id: String,
    state: State<'_, RuntimeState>,
) -> Result<String, ApiError> {
    with_artifact_context(&root_id, &state, |reader, index| {
        derive_markdown_path(reader, index, &document_id)
    })
}

#[tauri::command]
fn copy_continuation_prompt(
    root_id: String,
    artifact_id: String,
    state: State<'_, RuntimeState>,
) -> Result<String, ApiError> {
    with_artifact_context(&root_id, &state, |reader, index| {
        derive_continuation_prompt(reader, index, &artifact_id)
    })
}

#[tauri::command]
fn open_terminal(
    root_id: String,
    project_id: String,
    state: State<'_, RuntimeState>,
) -> Result<(), ApiError> {
    let index = state
        .scans
        .current(&root_id)
        .or_else(|| state.store.load_index(&root_id).ok().flatten())
        .ok_or_else(|| ApiError::new("index_unavailable", "No usable index is available"))?;
    let project = index
        .projects
        .iter()
        .find(|project| project.project.id == project_id)
        .ok_or_else(|| ApiError::new("project_not_found", "Project is no longer indexed"))?;
    let root = find_root(&state.store, &root_id)?;
    let reader = ContainedReader::approve(root.path(), ScanPolicy::default().max_file_bytes)
        .map_err(ApiError::from_error)?;
    let canonical_project =
        std::fs::canonicalize(&project.project.root_path).map_err(ApiError::from_error)?;
    if !canonical_project.starts_with(reader.root()) {
        return Err(ApiError::new(
            "outside_approved_root",
            "Project escaped its approved root",
        ));
    }
    Launcher::new(&SystemProcessRunner)
        .open_terminal(&canonical_project)
        .map_err(ApiError::from_error)
}

#[tauri::command]
fn open_external(
    root_id: String,
    project_id: String,
    target: String,
    state: State<'_, RuntimeState>,
) -> Result<(), ApiError> {
    let index = state
        .scans
        .current(&root_id)
        .or_else(|| state.store.load_index(&root_id).ok().flatten())
        .ok_or_else(|| ApiError::new("index_unavailable", "No usable index is available"))?;
    let project = index
        .projects
        .iter()
        .find(|project| project.project.id == project_id)
        .ok_or_else(|| ApiError::new("project_not_found", "Project is no longer indexed"))?;
    Launcher::new(&SystemProcessRunner)
        .open_external(&target, std::path::Path::new(&project.project.root_path))
        .map_err(ApiError::from_error)
}

fn with_artifact_context<T>(
    root_id: &str,
    state: &State<'_, RuntimeState>,
    operation: impl FnOnce(&ContainedReader, &IndexSnapshot) -> Result<T, ApiError>,
) -> Result<T, ApiError> {
    let root = find_root(&state.store, root_id)?;
    let index = state
        .scans
        .current(root_id)
        .or_else(|| state.store.load_index(root_id).ok().flatten())
        .ok_or_else(|| ApiError::new("index_unavailable", "No usable index is available"))?;
    let reader = ContainedReader::approve(root.path(), ScanPolicy::default().max_file_bytes)
        .map_err(ApiError::from_error)?;
    operation(&reader, &index)
}

#[tauri::command]
fn get_generated_view(
    root_id: String,
    bundle_id: String,
    state: State<'_, RuntimeState>,
) -> Result<GeneratedView, ApiError> {
    let index = current_index(&root_id, &state)?;
    let (project_root, bundle) = find_bundle(&index, &bundle_id)?;
    let root = find_root(&state.store, &root_id)?;
    let reader = ContainedReader::approve(root.path(), ScanPolicy::default().max_file_bytes)
        .map_err(ApiError::from_error)?;
    let live = catalog::live_bundle_state(&reader, Path::new(project_root), &bundle.bundle)
        .map_err(ApiError::from_error)?;
    let refreshed = {
        let _publication = state.generated_publication.lock();
        ensure_bundle_reachable(state.inner(), &bundle_id)?;
        refresh_cached_generated_view(
            &state.generated,
            &bundle_id,
            &live.fingerprint,
            bundle
                .bundle
                .members
                .iter()
                .map(|member| member.relative_path.clone())
                .collect(),
        )
    };
    if let Some(refreshed) = refreshed {
        return Ok(refreshed);
    }
    let view = match state
        .store
        .find_latest_generated_view(&bundle_id, GenerationMode::Summary, "summary-v1")
        .map_err(ApiError::from_error)?
    {
        Some(result) if result.source_fingerprint == live.fingerprint => {
            GeneratedView::Current { result }
        }
        Some(result) => GeneratedView::Stale {
            changed_inputs: result.included_paths.clone(),
            result,
        },
        None => GeneratedView::NeverGenerated,
    };
    publish_generated_view(state.inner(), &bundle_id, view)
}

#[tauri::command]
fn request_summary(
    root_id: String,
    bundle_id: String,
    state: State<'_, RuntimeState>,
) -> Result<GeneratedView, ApiError> {
    let root = find_root(&state.store, &root_id)?;
    let index = current_index(&root_id, &state)?;
    let (project_root, bundle) = find_bundle(&index, &bundle_id)?;
    let paths = bundle_generation_paths(
        Path::new(project_root),
        &bundle
            .bundle
            .members
            .iter()
            .map(|member| member.relative_path.clone())
            .collect::<Vec<_>>(),
    );
    let reader = ContainedReader::approve(root.path(), ScanPolicy::default().max_file_bytes)
        .map_err(ApiError::from_error)?;
    let snapshot = build_generation_snapshot(
        &reader,
        &paths,
        GenerationMode::Summary,
        "summary-v1",
        &GenerationLimits::default(),
    )
    .map_err(ApiError::from_error)?;

    prepare_pi_directory(&state.pi_temp)?;
    let config = PiConfig::installed(state.pi_temp.clone());
    let nonce = format!(
        "{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    );
    if let PiCapability::Unavailable { reason } = probe_pi(&SystemCommandRunner, &config, &nonce) {
        return Err(ApiError::new("pi_capability_unavailable", reason));
    }

    if let Some(cached) = state
        .store
        .find_generated_view(
            &bundle_id,
            GenerationMode::Summary,
            snapshot.source_fingerprint.as_str(),
            &snapshot.prompt_version,
        )
        .map_err(ApiError::from_error)?
    {
        let view = GeneratedView::Current { result: cached };
        return publish_generated_view(state.inner(), &bundle_id, view);
    }

    let request_id = nonce;
    let prior = state
        .generated
        .lock()
        .remove(&bundle_id)
        .unwrap_or(GeneratedView::NeverGenerated);
    let prior_result = previous_result(&prior);
    let generating = start_generation(
        prior,
        request_id.clone(),
        snapshot.source_fingerprint.clone(),
    );
    let jobs = PiJobRunner::new(SystemCommandRunner, config);
    register_active_pi_request(
        state.inner(),
        &request_id,
        &root_id,
        &bundle_id,
        generating.clone(),
        jobs.cancellation_flag(&request_id),
    )?;
    let events = jobs.run(
        &request_id,
        snapshot.clone(),
        chrono::Utc::now().to_rfc3339(),
    );
    let next = match events.last() {
        Some(GenerationJobEvent::Completed { result, .. }) => {
            let completed = match catalog::live_bundle_state(
                &reader,
                Path::new(project_root),
                &bundle.bundle,
            ) {
                Ok(live) => generation_completed(
                    generating.clone(),
                    &request_id,
                    result.clone(),
                    &live.fingerprint,
                ),
                Err(error) => GeneratedView::Failed {
                    previous: Some(result.clone()),
                    failure: format!(
                        "Summary generated but sources could not be revalidated: {error}"
                    ),
                },
            };
            persist_and_publish_generated_view(
                state.inner(),
                &bundle_id,
                result,
                completed,
                prior_result,
            )
        }
        Some(GenerationJobEvent::Failed { failure, .. }) => publish_generated_view(
            state.inner(),
            &bundle_id,
            generation_failed(generating, &request_id, failure),
        ),
        Some(GenerationJobEvent::Cancelled { .. }) => publish_generated_view(
            state.inner(),
            &bundle_id,
            generation_failed(generating, &request_id, "Generation cancelled"),
        ),
        _ => publish_generated_view(
            state.inner(),
            &bundle_id,
            generation_failed(generating, &request_id, "Pi returned no terminal job event"),
        ),
    };
    state.pi_cancellations.lock().remove(&request_id);
    next
}

#[tauri::command]
fn cancel_summary(request_id: String, state: State<'_, RuntimeState>) -> bool {
    state
        .pi_cancellations
        .lock()
        .get(&request_id)
        .is_some_and(|request| {
            request
                .cancelled
                .store(true, std::sync::atomic::Ordering::Release);
            true
        })
}

fn current_index(
    root_id: &str,
    state: &State<'_, RuntimeState>,
) -> Result<IndexSnapshot, ApiError> {
    state
        .scans
        .current(root_id)
        .or_else(|| state.store.load_index(root_id).ok().flatten())
        .ok_or_else(|| ApiError::new("index_unavailable", "No usable index is available"))
}

fn register_active_pi_request(
    state: &RuntimeState,
    request_id: &str,
    root_id: &str,
    bundle_id: &str,
    generating: GeneratedView,
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<(), ApiError> {
    let _publication = state.generated_publication.lock();
    ensure_bundle_reachable(state, bundle_id)?;
    state
        .generated
        .lock()
        .insert(bundle_id.to_owned(), generating);
    state.pi_cancellations.lock().insert(
        request_id.to_owned(),
        ActivePiRequest {
            root_id: root_id.to_owned(),
            bundle_id: bundle_id.to_owned(),
            cancelled,
        },
    );
    Ok(())
}

fn publish_generated_view(
    state: &RuntimeState,
    bundle_id: &str,
    view: GeneratedView,
) -> Result<GeneratedView, ApiError> {
    let _publication = state.generated_publication.lock();
    ensure_bundle_reachable(state, bundle_id)?;
    state
        .generated
        .lock()
        .insert(bundle_id.to_owned(), view.clone());
    Ok(view)
}

fn persist_and_publish_generated_view(
    state: &RuntimeState,
    bundle_id: &str,
    result: &GeneratedResult,
    view: GeneratedView,
    previous: Option<GeneratedResult>,
) -> Result<GeneratedView, ApiError> {
    let _publication = state.generated_publication.lock();
    ensure_bundle_reachable(state, bundle_id)?;
    let view = match state.store.save_generated_view(bundle_id, result) {
        Ok(()) => view,
        Err(error) => GeneratedView::Failed {
            previous,
            failure: format!("Summary generated but cache storage failed: {error}"),
        },
    };
    ensure_bundle_reachable(state, bundle_id)?;
    state
        .generated
        .lock()
        .insert(bundle_id.to_owned(), view.clone());
    Ok(view)
}

fn ensure_bundle_reachable(state: &RuntimeState, bundle_id: &str) -> Result<(), ApiError> {
    let roots = state.store.list_roots().map_err(ApiError::from_error)?;
    for root in roots {
        let index = match state.scans.current(root.id()) {
            Some(index) => Some(index),
            None => state
                .store
                .load_index(root.id())
                .map_err(ApiError::from_error)?,
        };
        if index.as_ref().is_some_and(|index| {
            index
                .projects
                .iter()
                .flat_map(|project| &project.bundles)
                .any(|bundle| bundle.bundle.id == bundle_id)
        }) {
            return Ok(());
        }
    }
    state.generated.lock().remove(bundle_id);
    Err(ApiError::new(
        "root_or_bundle_unavailable",
        "The approved root or generated bundle is no longer available",
    ))
}

fn refresh_cached_generated_view(
    cache: &Mutex<BTreeMap<String, GeneratedView>>,
    bundle_id: &str,
    current_fingerprint: &backstage_core::SourceFingerprint,
    changed_inputs: Vec<String>,
) -> Option<GeneratedView> {
    let mut cache = cache.lock();
    let refreshed = sources_changed(
        cache.get(bundle_id).cloned()?,
        current_fingerprint,
        changed_inputs,
    );
    cache.insert(bundle_id.to_owned(), refreshed.clone());
    Some(refreshed)
}

fn find_bundle<'a>(
    index: &'a IndexSnapshot,
    bundle_id: &str,
) -> Result<(&'a str, &'a IndexedBundle), ApiError> {
    index
        .projects
        .iter()
        .find_map(|project| {
            project
                .bundles
                .iter()
                .find(|bundle| bundle.bundle.id == bundle_id)
                .map(|bundle| (project.project.root_path.as_str(), bundle))
        })
        .ok_or_else(|| ApiError::new("bundle_not_found", "Bundle is no longer indexed"))
}

fn prepare_pi_directory(pi_temp: &Path) -> Result<(), ApiError> {
    let agent_dir = pi_temp.join("agent");
    std::fs::create_dir_all(&agent_dir).map_err(ApiError::from_error)?;
    let settings = r#"{
  "retry": {"enabled": false, "maxRetries": 0, "provider": {"timeoutMs": 45000, "maxRetries": 0, "maxRetryDelayMs": 1000}},
  "compaction": {"enabled": false},
  "httpIdleTimeoutMs": 45000,
  "enableInstallTelemetry": false,
  "defaultProjectTrust": "never"
}"#;
    std::fs::write(agent_dir.join("settings.json"), settings).map_err(ApiError::from_error)?;
    let source_auth = directories::BaseDirs::new()
        .map(|dirs| dirs.home_dir().join(".pi/agent/auth.json"))
        .filter(|path| path.is_file())
        .ok_or_else(|| ApiError::new("pi_auth_unavailable", "Pi authentication is unavailable"))?;
    let target_auth = agent_dir.join("auth.json");
    if !target_auth.exists() {
        std::fs::copy(source_auth, target_auth).map_err(ApiError::from_error)?;
    }
    Ok(())
}

fn find_root(store: &SqliteStore, root_id: &str) -> Result<ApprovedRoot, ApiError> {
    list_approved_roots(store)?
        .into_iter()
        .find(|root| root.id() == root_id)
        .ok_or_else(|| ApiError::new("root_not_found", "Approved root is no longer available"))
}

pub fn run() {
    let paths = app_paths::AppPaths::system().expect("app-owned paths should be available");
    paths
        .ensure_exists()
        .expect("app-owned paths should initialize");
    let store = SqliteStore::open(paths.database_path())
        .or_else(|_| SqliteStore::in_memory())
        .expect("an in-memory app index should initialize");
    let scans = ScanCoordinator::default();
    if let Ok(roots) = store.list_roots() {
        for root in roots {
            if let Ok(Some(snapshot)) = store.load_index(root.id()) {
                scans.hydrate(snapshot);
            }
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(RuntimeState {
            store,
            scans,
            generated: Mutex::new(BTreeMap::new()),
            settings_mutation: Mutex::new(()),
            generated_publication: Mutex::new(()),
            scan_admission: Mutex::new(()),
            scan_cancellations: Mutex::new(BTreeMap::new()),
            pi_cancellations: Mutex::new(BTreeMap::new()),
            pi_temp: paths.cache_dir().join("pi"),
        })
        .invoke_handler(tauri::generate_handler![
            list_roots,
            approve_root,
            remove_root,
            list_patterns,
            add_pattern,
            remove_pattern,
            restore_default_patterns,
            scan_root,
            cancel_scan,
            get_index,
            get_artifact_detail,
            get_markdown_detail,
            copy_artifact_path,
            copy_markdown_path,
            copy_continuation_prompt,
            open_terminal,
            open_external,
            get_generated_view,
            request_summary,
            cancel_summary
        ])
        .run(tauri::generate_context!())
        .expect("Backstage application failed");
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, mpsc};
    use std::time::Duration;

    use backstage_core::{
        BundleKind, GeneratedResult, SourceFingerprint, canonical_planning_patterns,
    };
    use tempfile::TempDir;

    use super::*;

    fn test_runtime() -> RuntimeState {
        RuntimeState {
            store: SqliteStore::in_memory().expect("test store"),
            scans: ScanCoordinator::default(),
            generated: Mutex::new(BTreeMap::new()),
            settings_mutation: Mutex::new(()),
            generated_publication: Mutex::new(()),
            scan_admission: Mutex::new(()),
            scan_cancellations: Mutex::new(BTreeMap::new()),
            pi_cancellations: Mutex::new(BTreeMap::new()),
            pi_temp: std::env::temp_dir().join("backstage-test-pi"),
        }
    }

    fn empty_snapshot(root_id: &str, generation: u64, revision: u64) -> IndexSnapshot {
        IndexSnapshot {
            root_id: root_id.to_owned(),
            generation,
            indexed_at: "2026-08-14T00:00:00Z".to_owned(),
            configuration_revision: revision,
            projects: vec![],
            warnings: vec![],
        }
    }

    fn generated_result(text: &str) -> GeneratedResult {
        GeneratedResult {
            text: text.to_owned(),
            mode: GenerationMode::Summary,
            source_fingerprint: SourceFingerprint::from_trusted("sha256:test"),
            included_paths: vec!["PLAN.md".to_owned()],
            generated_at: "2026-08-14T00:00:00Z".to_owned(),
            model: None,
            prompt_version: "summary-v1".to_owned(),
        }
    }

    #[test]
    fn bounded_tasks_run_concurrently_and_return_failures_in_input_order() {
        let active = std::sync::atomic::AtomicUsize::new(0);
        let maximum = std::sync::atomic::AtomicUsize::new(0);
        let inputs = [0, 1, 2, 3];

        let failures = run_bounded_tasks(&inputs, 2, |input| {
            let now = active.fetch_add(1, std::sync::atomic::Ordering::AcqRel) + 1;
            maximum.fetch_max(now, std::sync::atomic::Ordering::AcqRel);
            std::thread::sleep(Duration::from_millis(25));
            active.fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
            input % 2 == 1
        });

        assert_eq!(maximum.load(std::sync::atomic::Ordering::Acquire), 2);
        assert_eq!(failures, vec![1, 3]);
    }

    #[test]
    fn pattern_mutation_persists_then_rescans_every_root_with_one_revision() {
        let state = test_runtime();
        let first_dir = TempDir::new().expect("first root");
        let second_dir = TempDir::new().expect("second root");
        std::fs::write(first_dir.path().join("custom.md"), "# First\n").expect("first plan");
        std::fs::write(second_dir.path().join("custom.md"), "# Second\n").expect("second plan");
        let first = approve_root_path(&state.store, first_dir.path()).expect("first approval");
        let second = approve_root_path(&state.store, second_dir.path()).expect("second approval");

        let mutation = add_pattern_runtime("^custom\\.md$", &state).expect("add pattern");

        assert_eq!(mutation.configuration_revision, 1);
        assert_eq!(mutation.indexes.len(), 2);
        assert!(mutation.failed_root_ids.is_empty());
        assert!(
            mutation
                .patterns
                .iter()
                .any(|pattern| pattern.expression() == "^custom\\.md$")
        );
        let indexed_roots = mutation
            .indexes
            .iter()
            .map(|index| index.root_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            indexed_roots,
            std::collections::BTreeSet::from([first.id(), second.id()])
        );
        assert!(mutation.indexes.iter().all(|index| {
            index.configuration_revision == mutation.configuration_revision
                && index
                    .projects
                    .iter()
                    .flat_map(|project| &project.bundles)
                    .any(|bundle| {
                        bundle.bundle.kind == BundleKind::PossibleArtifact
                            && bundle.bundle.name == "custom.md"
                    })
        }));
        let json = serde_json::to_value(&mutation).expect("serialize mutation");
        assert!(json.get("configurationRevision").is_some());
        assert!(json.get("failedRootIds").is_some());
        assert!(json.get("configuration_revision").is_none());
    }

    #[test]
    fn pattern_mutation_responses_are_serialized_and_revision_coherent() {
        let state = Arc::new(test_runtime());
        let root_dir = TempDir::new().expect("root");
        std::fs::write(root_dir.path().join("first.md"), "# First\n").expect("first plan");
        std::fs::write(root_dir.path().join("second.md"), "# Second\n").expect("second plan");
        approve_root_path(&state.store, root_dir.path()).expect("approval");
        let (first_paused_tx, first_paused_rx) = mpsc::sync_channel(0);
        let (resume_first_tx, resume_first_rx) = mpsc::sync_channel(0);
        let first_state = Arc::clone(&state);
        let first = std::thread::spawn(move || {
            add_pattern_runtime_with_response_seam("^first\\.md$", &first_state, || {
                first_paused_tx.send(()).expect("announce revision 1 seam");
                resume_first_rx.recv().expect("resume revision 1 response");
            })
        });
        first_paused_rx
            .recv()
            .expect("revision 1 reached its response seam");

        let (second_started_tx, second_started_rx) = mpsc::sync_channel(0);
        let (second_result_tx, second_result_rx) = mpsc::sync_channel(1);
        let second_state = Arc::clone(&state);
        let second = std::thread::spawn(move || {
            second_started_tx.send(()).expect("announce revision 2");
            second_result_tx
                .send(add_pattern_runtime("^second\\.md$", &second_state))
                .expect("return revision 2 response");
        });
        second_started_rx.recv().expect("revision 2 started");
        let interleaved = second_result_rx
            .recv_timeout(Duration::from_millis(250))
            .ok();
        let responses_interleaved = interleaved.is_some();

        resume_first_tx.send(()).expect("resume revision 1");
        let first_response = first
            .join()
            .expect("revision 1 thread")
            .expect("revision 1");
        let second_response = interleaved
            .unwrap_or_else(|| second_result_rx.recv().expect("revision 2 response"))
            .expect("revision 2");
        second.join().expect("revision 2 thread");

        assert!(
            !responses_interleaved,
            "revision 2 returned while revision 1 was still assembling its response"
        );
        assert_eq!(first_response.configuration_revision, 1);
        assert_eq!(first_response.indexes.len(), 1);
        assert!(first_response.indexes.iter().all(|index| {
            index.configuration_revision == 1
                && index
                    .projects
                    .iter()
                    .flat_map(|project| &project.bundles)
                    .any(|bundle| bundle.bundle.name == "first.md")
                && index
                    .projects
                    .iter()
                    .flat_map(|project| &project.bundles)
                    .all(|bundle| bundle.bundle.name != "second.md")
        }));
        assert!(
            first_response
                .patterns
                .iter()
                .any(|pattern| pattern.expression() == "^first\\.md$")
        );
        assert!(
            first_response
                .patterns
                .iter()
                .all(|pattern| pattern.expression() != "^second\\.md$")
        );
        assert_eq!(second_response.configuration_revision, 2);
        assert_eq!(second_response.indexes.len(), 1);
        assert!(
            second_response
                .patterns
                .iter()
                .any(|pattern| pattern.expression() == "^first\\.md$")
        );
        assert!(
            second_response
                .patterns
                .iter()
                .any(|pattern| pattern.expression() == "^second\\.md$")
        );
        assert!(second_response.indexes.iter().all(|index| {
            index.configuration_revision == 2
                && ["first.md", "second.md"].iter().all(|name| {
                    index
                        .projects
                        .iter()
                        .flat_map(|project| &project.bundles)
                        .any(|bundle| bundle.bundle.name == *name)
                })
        }));
    }

    #[test]
    fn remove_and_restore_pattern_mutations_rescan_with_their_committed_revisions() {
        let state = test_runtime();
        let root_dir = TempDir::new().expect("root");
        std::fs::write(root_dir.path().join("PLAN.md"), "# Default\n").expect("default plan");
        std::fs::write(root_dir.path().join("custom.md"), "# Custom\n").expect("custom plan");
        let root = approve_root_path(&state.store, root_dir.path()).expect("approval");
        let added = add_pattern_runtime("^custom\\.md$", &state).expect("add custom");
        let custom_id = added
            .patterns
            .iter()
            .find(|pattern| pattern.expression() == "^custom\\.md$")
            .expect("custom pattern")
            .id()
            .to_owned();

        let removed = remove_pattern_runtime(&custom_id, &state).expect("remove custom");

        assert_eq!(removed.configuration_revision, 2);
        assert_eq!(removed.indexes[0].configuration_revision, 2);
        assert!(
            removed.indexes[0]
                .projects
                .iter()
                .flat_map(|project| &project.bundles)
                .all(|bundle| bundle.bundle.name != "custom.md")
        );
        let default_id = removed
            .patterns
            .iter()
            .find(|pattern| pattern.matches_normalized_markdown_path("PLAN.md"))
            .expect("PLAN default")
            .id()
            .to_owned();
        let without_default = remove_pattern_runtime(&default_id, &state).expect("remove default");
        assert_eq!(without_default.configuration_revision, 3);

        let restored = restore_default_patterns_runtime(&state).expect("restore defaults");

        assert_eq!(restored.configuration_revision, 4);
        assert_eq!(restored.indexes[0].root_id, root.id());
        assert_eq!(restored.indexes[0].configuration_revision, 4);
        assert!(
            restored
                .patterns
                .iter()
                .any(|pattern| pattern.id() == default_id)
        );
    }

    #[test]
    fn invalid_pattern_mutation_changes_neither_configuration_nor_index() {
        let state = test_runtime();
        let root_dir = TempDir::new().expect("root");
        let root = approve_root_path(&state.store, root_dir.path()).expect("approval");
        let prior = empty_snapshot(root.id(), 9, 0);
        state.store.save_index(&prior).expect("persist prior index");
        state.scans.hydrate(prior.clone());
        let configuration = state.store.planning_configuration().expect("configuration");

        let error = add_pattern_runtime("(", &state).expect_err("invalid pattern");

        assert_eq!(error.code, "planning_pattern_invalid");
        assert_eq!(
            state
                .store
                .planning_configuration()
                .expect("unchanged config"),
            configuration
        );
        assert_eq!(state.scans.current(root.id()), Some(prior.clone()));
        assert_eq!(
            state.store.load_index(root.id()).expect("unchanged index"),
            Some(prior)
        );
    }

    #[test]
    fn pattern_cache_write_failure_marks_root_failed_and_retry_clears_it() {
        let state = test_runtime();
        let root_dir = TempDir::new().expect("root");
        std::fs::write(root_dir.path().join("custom.md"), "# Custom\n").expect("custom plan");
        let root = approve_root_path(&state.store, root_dir.path()).expect("approval");
        state.store.fail_next_index_save();

        let mutation =
            add_pattern_runtime("^custom\\.md$", &state).expect("configuration persists");

        let current = state.scans.current(root.id()).expect("in-memory index");
        assert_eq!(
            state
                .store
                .planning_configuration()
                .expect("committed configuration")
                .revision,
            1
        );
        assert_eq!(mutation.failed_root_ids, vec![root.id().to_owned()]);
        assert_eq!(mutation.indexes, vec![current.clone()]);
        assert!(
            state
                .store
                .load_index(root.id())
                .expect("cache lookup")
                .is_none()
        );
        assert_eq!(current.configuration_revision, 1);

        let configuration = state.store.planning_configuration().expect("configuration");
        let retried =
            rescan_after_pattern_mutation(configuration, &state).expect("successful retry");
        let retried_current = state.scans.current(root.id()).expect("retried index");

        assert!(retried.failed_root_ids.is_empty());
        assert_eq!(retried.indexes, vec![retried_current.clone()]);
        assert_eq!(
            state.store.load_index(root.id()).expect("cached retry"),
            Some(retried_current)
        );
    }

    #[test]
    fn failed_pattern_rescan_preserves_the_prior_snapshot() {
        let state = test_runtime();
        let root_dir = TempDir::new().expect("root");
        let root = approve_root_path(&state.store, root_dir.path()).expect("approval");
        let prior = empty_snapshot(root.id(), 3, 0);
        state.store.save_index(&prior).expect("persist prior index");
        state.scans.hydrate(prior.clone());
        std::fs::remove_dir_all(root_dir.path()).expect("make root unavailable");

        let mutation =
            add_pattern_runtime("^custom\\.md$", &state).expect("configuration persists");

        assert_eq!(mutation.configuration_revision, 1);
        assert_eq!(mutation.failed_root_ids, vec![root.id().to_owned()]);
        assert_eq!(mutation.indexes, vec![prior.clone()]);
        assert_eq!(state.scans.current(root.id()), Some(prior));
    }

    #[test]
    fn coordinated_runtime_removal_cancels_forgets_and_returns_authoritative_inventory() {
        let state = test_runtime();
        let root_dir = TempDir::new().expect("root");
        std::fs::write(root_dir.path().join("repository.md"), "# Untouched\n")
            .expect("repository file");
        let root = approve_root_path(&state.store, root_dir.path()).expect("approval");
        let current = empty_snapshot(root.id(), 1, 0);
        state.store.save_index(&current).expect("persist index");
        state.scans.hydrate(current);
        let delayed = state.scans.begin_for_revision(root.id(), 0);
        let cancellation = CancellationToken::new();
        state
            .scan_cancellations
            .lock()
            .insert(root.id().to_owned(), cancellation.clone());

        let inventory = remove_root_runtime(root.id(), &state).expect("remove root");

        assert!(cancellation.is_cancelled());
        assert!(inventory.roots.is_empty());
        assert!(inventory.indexes.is_empty());
        assert!(root_dir.path().join("repository.md").is_file());
        assert!(state.scans.current(root.id()).is_none());
        let mut delayed_snapshot = empty_snapshot(root.id(), delayed.generation, 0);
        delayed_snapshot.configuration_revision = delayed.configuration_revision;
        assert_eq!(
            state.scans.complete(&delayed, delayed_snapshot),
            CompletionDisposition::Superseded
        );
    }

    #[test]
    fn runtime_removal_retains_memory_only_index_reachability_after_cache_write_failure() {
        let state = test_runtime();
        let removed_dir = TempDir::new().expect("removed root");
        let retained_dir = TempDir::new().expect("retained root");
        let removed =
            approve_root_path(&state.store, removed_dir.path()).expect("removed approval");
        let retained =
            approve_root_path(&state.store, retained_dir.path()).expect("retained approval");
        state
            .store
            .save_index(&empty_snapshot(retained.id(), 1, 0))
            .expect("persist stale retained index");
        std::fs::write(retained_dir.path().join("PLAN.md"), "# Memory only\n")
            .expect("planning file");
        let reader =
            ContainedReader::approve(retained.path(), ScanPolicy::default().max_file_bytes)
                .expect("reader");
        let discovered =
            discover_projects(&reader, &ScanPolicy::default(), &CancellationToken::new());
        let current = catalog::build_index(
            &reader,
            discovered.projects,
            2,
            "2026-08-14T00:00:01Z".to_owned(),
            discovered.warnings,
        );
        let bundle_id = current.projects[0].bundles[0].bundle.id.clone();
        state.scans.hydrate(current.clone());
        let result = generated_result("memory-only summary");
        state
            .store
            .save_generated_view(&bundle_id, &result)
            .expect("generated view");
        let view = GeneratedView::Current { result };
        state
            .generated
            .lock()
            .insert(bundle_id.clone(), view.clone());

        let inventory = remove_root_runtime(removed.id(), &state).expect("remove root");

        assert_eq!(inventory.indexes, vec![current]);
        assert_eq!(state.generated.lock().get(&bundle_id), Some(&view));
        assert!(
            state
                .store
                .find_latest_generated_view(&bundle_id, GenerationMode::Summary, "summary-v1")
                .expect("generated lookup")
                .is_some()
        );
    }

    #[test]
    fn failed_runtime_removal_preserves_the_prior_coordinator_snapshot() {
        let app_data = TempDir::new().expect("app data");
        let database = app_data.path().join("index.sqlite3");
        let state = RuntimeState {
            store: SqliteStore::open(&database).expect("test store"),
            scans: ScanCoordinator::default(),
            generated: Mutex::new(BTreeMap::new()),
            settings_mutation: Mutex::new(()),
            generated_publication: Mutex::new(()),
            scan_admission: Mutex::new(()),
            scan_cancellations: Mutex::new(BTreeMap::new()),
            pi_cancellations: Mutex::new(BTreeMap::new()),
            pi_temp: app_data.path().join("pi"),
        };
        let root_dir = TempDir::new().expect("root");
        let root = approve_root_path(&state.store, root_dir.path()).expect("approval");
        let prior = empty_snapshot(root.id(), 7, 0);
        state.store.save_index(&prior).expect("persist index");
        state.scans.hydrate(prior.clone());
        let connection = rusqlite::Connection::open(&database).expect("injector connection");
        connection
            .execute_batch(
                "CREATE TRIGGER reject_runtime_root_delete BEFORE DELETE ON approved_roots
                 BEGIN SELECT RAISE(ABORT, 'injected removal failure'); END;",
            )
            .expect("failure trigger");

        let error = remove_root_runtime(root.id(), &state).expect_err("removal failure");

        assert_eq!(error.code, "operation_failed");
        assert_eq!(state.scans.current(root.id()), Some(prior));
        assert_eq!(state.store.list_roots().expect("retained root"), vec![root]);
    }

    #[test]
    fn failed_runtime_removal_preserves_active_scan_without_a_prior_snapshot() {
        let app_data = TempDir::new().expect("app data");
        let database = app_data.path().join("index.sqlite3");
        let state = RuntimeState {
            store: SqliteStore::open(&database).expect("test store"),
            scans: ScanCoordinator::default(),
            generated: Mutex::new(BTreeMap::new()),
            settings_mutation: Mutex::new(()),
            generated_publication: Mutex::new(()),
            scan_admission: Mutex::new(()),
            scan_cancellations: Mutex::new(BTreeMap::new()),
            pi_cancellations: Mutex::new(BTreeMap::new()),
            pi_temp: app_data.path().join("pi"),
        };
        let root_dir = TempDir::new().expect("root");
        let root = approve_root_path(&state.store, root_dir.path()).expect("approval");
        let permit = state.scans.begin_for_revision(root.id(), 0);
        assert!(permit.admitted);
        let cancellation = CancellationToken::new();
        state
            .scan_cancellations
            .lock()
            .insert(root.id().to_owned(), cancellation.clone());
        let connection = rusqlite::Connection::open(&database).expect("injector connection");
        connection
            .execute_batch(
                "CREATE TRIGGER reject_active_root_delete BEFORE DELETE ON approved_roots
                 BEGIN SELECT RAISE(ABORT, 'injected removal failure'); END;",
            )
            .expect("failure trigger");

        let error = remove_root_runtime(root.id(), &state).expect_err("removal failure");

        assert_eq!(error.code, "operation_failed");
        assert_eq!(
            state.store.list_roots().expect("retained approval"),
            vec![root]
        );
        assert!(!cancellation.is_cancelled());
        assert!(
            state
                .scan_cancellations
                .lock()
                .get(&permit.root_id)
                .is_some_and(|active| Arc::ptr_eq(&active.0, &cancellation.0))
        );
        let snapshot = empty_snapshot(&permit.root_id, permit.generation, 0);
        assert_eq!(
            state.scans.complete(&permit, snapshot.clone()),
            CompletionDisposition::Accepted
        );
        assert_eq!(state.scans.current(&permit.root_id), Some(snapshot));
    }

    #[test]
    fn a_stale_revision_does_not_cancel_or_replace_the_current_scan_token() {
        let state = test_runtime();
        let root_dir = TempDir::new().expect("root");
        let root = approve_root_path(&state.store, root_dir.path()).expect("approval");
        let current = state.scans.begin_for_revision(root.id(), 2);
        assert!(current.admitted);
        let current_cancellation = CancellationToken::new();
        state
            .scan_cancellations
            .lock()
            .insert(root.id().to_owned(), current_cancellation.clone());
        let stale_configuration = PlanningPatternConfiguration {
            revision: 1,
            patterns: canonical_planning_patterns(),
        };

        let error = scan_root_with_configuration(&root, &stale_configuration, &state)
            .expect_err("stale revision must not scan");

        assert_eq!(error.code, "scan_superseded");
        assert!(!current_cancellation.is_cancelled());
        assert!(
            state
                .scan_cancellations
                .lock()
                .get(root.id())
                .is_some_and(|active| Arc::ptr_eq(&active.0, &current_cancellation.0))
        );
    }

    #[test]
    fn scan_paused_after_initial_approval_check_cannot_register_or_read_after_removal() {
        let state = Arc::new(test_runtime());
        let root_dir = TempDir::new().expect("root");
        std::fs::write(root_dir.path().join("PLAN.md"), "# Must not be read\n")
            .expect("planning file");
        let root = approve_root_path(&state.store, root_dir.path()).expect("approval");
        let configuration = state.store.planning_configuration().expect("configuration");
        let (paused_tx, paused_rx) = mpsc::sync_channel(0);
        let (resume_tx, resume_rx) = mpsc::sync_channel(0);
        let scan_state = Arc::clone(&state);
        let scan_root = root.clone();
        let scan = std::thread::spawn(move || {
            scan_root_with_configuration_after_initial_check(
                &scan_root,
                &configuration,
                &scan_state,
                || {
                    paused_tx.send(()).expect("announce pause");
                    resume_rx.recv().expect("resume scan");
                },
            )
        });
        paused_rx.recv().expect("scan reached admission seam");

        remove_root_runtime(root.id(), &state).expect("remove root");
        std::fs::remove_dir_all(root_dir.path()).expect("make any post-removal read fail");
        resume_tx.send(()).expect("resume scan");
        let error = scan
            .join()
            .expect("scan thread")
            .expect_err("removed root must not be admitted");

        assert_eq!(error.code, "root_not_found");
        assert!(!state.scan_cancellations.lock().contains_key(root.id()));
        assert!(state.scans.current(root.id()).is_none());
    }

    #[test]
    fn a_stale_pattern_root_snapshot_cannot_rescan_a_removed_approval() {
        let state = test_runtime();
        let root_dir = TempDir::new().expect("root");
        let root = approve_root_path(&state.store, root_dir.path()).expect("approval");
        let configuration = state.store.planning_configuration().expect("configuration");
        state
            .store
            .remove_root_state(root.id())
            .expect("remove approval");
        state.scans.forget(root.id());

        let error = scan_root_with_configuration(&root, &configuration, &state)
            .expect_err("stale root must not scan");

        assert_eq!(error.code, "root_not_found");
        assert!(state.scans.current(root.id()).is_none());
    }

    #[test]
    fn coordinated_runtime_removal_reports_explicit_not_found() {
        let state = test_runtime();

        let error = remove_root_runtime("root_unknown", &state).expect_err("unknown root");

        assert_eq!(error.code, "root_not_found");
    }

    #[test]
    fn delayed_summary_completion_after_sole_root_removal_cannot_resurrect_generated_data() {
        let state = test_runtime();
        let root_dir = TempDir::new().expect("root");
        std::fs::write(root_dir.path().join("PLAN.md"), "# Plan\n").expect("planning file");
        let root = approve_root_path(&state.store, root_dir.path()).expect("approval");
        let configuration = state.store.planning_configuration().expect("configuration");
        scan_root_with_configuration(&root, &configuration, &state).expect("scan");
        let bundle_id = state.scans.current(root.id()).expect("index").projects[0].bundles[0]
            .bundle
            .id
            .clone();
        let request_id = "request_delayed";
        let cancellation = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        state.pi_cancellations.lock().insert(
            request_id.to_owned(),
            ActivePiRequest {
                root_id: root.id().to_owned(),
                bundle_id: bundle_id.clone(),
                cancelled: std::sync::Arc::clone(&cancellation),
            },
        );
        state.generated.lock().insert(
            bundle_id.clone(),
            start_generation(
                GeneratedView::NeverGenerated,
                request_id,
                SourceFingerprint::from_trusted("sha256:test"),
            ),
        );

        remove_root_runtime(root.id(), &state).expect("remove root");

        assert!(cancellation.load(std::sync::atomic::Ordering::Acquire));
        assert!(!state.pi_cancellations.lock().contains_key(request_id));
        assert!(!state.generated.lock().contains_key(&bundle_id));

        let result = generated_result("delayed summary");
        let error = persist_and_publish_generated_view(
            &state,
            &bundle_id,
            &result,
            GeneratedView::Current {
                result: result.clone(),
            },
            None,
        )
        .expect_err("removed bundle must reject delayed completion");

        assert_eq!(error.code, "root_or_bundle_unavailable");
        assert!(!state.generated.lock().contains_key(&bundle_id));
        assert!(
            state
                .store
                .find_latest_generated_view(&bundle_id, GenerationMode::Summary, "summary-v1")
                .expect("generated lookup")
                .is_none()
        );
    }

    #[test]
    fn removed_request_may_publish_when_its_bundle_remains_reachable_from_another_root() {
        let state = test_runtime();
        let owner_dir = TempDir::new().expect("owner root");
        let overlapping_dir = TempDir::new().expect("overlapping root");
        std::fs::write(owner_dir.path().join("PLAN.md"), "# Shared plan\n").expect("planning file");
        let owner = approve_root_path(&state.store, owner_dir.path()).expect("owner approval");
        let overlapping =
            approve_root_path(&state.store, overlapping_dir.path()).expect("overlap approval");
        let configuration = state.store.planning_configuration().expect("configuration");
        scan_root_with_configuration(&owner, &configuration, &state).expect("owner scan");
        let owner_index = state.scans.current(owner.id()).expect("owner index");
        let bundle_id = owner_index.projects[0].bundles[0].bundle.id.clone();
        let mut overlapping_index = owner_index;
        overlapping_index.root_id = overlapping.id().to_owned();
        overlapping_index.generation += 1;
        state.scans.hydrate(overlapping_index.clone());
        state
            .store
            .save_index(&overlapping_index)
            .expect("overlap index");
        let cancellation = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        state.pi_cancellations.lock().insert(
            "request_overlap".to_owned(),
            ActivePiRequest {
                root_id: owner.id().to_owned(),
                bundle_id: bundle_id.clone(),
                cancelled: std::sync::Arc::clone(&cancellation),
            },
        );

        remove_root_runtime(owner.id(), &state).expect("remove owner root");
        let result = generated_result("overlap summary");
        let view = GeneratedView::Current {
            result: result.clone(),
        };
        let published =
            persist_and_publish_generated_view(&state, &bundle_id, &result, view.clone(), None)
                .expect("overlap remains reachable");

        assert!(cancellation.load(std::sync::atomic::Ordering::Acquire));
        assert_eq!(published, view);
        assert_eq!(state.generated.lock().get(&bundle_id), Some(&view));
    }

    #[test]
    fn cached_result_publication_rechecks_bundle_reachability() {
        let state = test_runtime();
        let root_dir = TempDir::new().expect("root");
        std::fs::write(root_dir.path().join("PLAN.md"), "# Plan\n").expect("planning file");
        let root = approve_root_path(&state.store, root_dir.path()).expect("approval");
        let configuration = state.store.planning_configuration().expect("configuration");
        scan_root_with_configuration(&root, &configuration, &state).expect("scan");
        let bundle_id = state.scans.current(root.id()).expect("index").projects[0].bundles[0]
            .bundle
            .id
            .clone();
        remove_root_runtime(root.id(), &state).expect("remove root");
        let cached = GeneratedView::Current {
            result: generated_result("cached summary"),
        };

        let error = publish_generated_view(&state, &bundle_id, cached)
            .expect_err("removed bundle must reject cached publication");

        assert_eq!(error.code, "root_or_bundle_unavailable");
        assert!(!state.generated.lock().contains_key(&bundle_id));
    }

    #[test]
    fn cached_generated_view_refresh_does_not_relock_cache() {
        let old_fingerprint = SourceFingerprint::from_trusted("sha256:old");
        let current_fingerprint = SourceFingerprint::from_trusted("sha256:current");
        let cached = GeneratedView::Current {
            result: GeneratedResult {
                text: "Summary".to_owned(),
                mode: GenerationMode::Summary,
                source_fingerprint: old_fingerprint,
                included_paths: vec!["tasks.md".to_owned()],
                generated_at: "2026-08-13T12:00:00Z".to_owned(),
                model: None,
                prompt_version: "summary-v1".to_owned(),
            },
        };
        let cache = Arc::new(Mutex::new(BTreeMap::from([(
            "bundle_1".to_owned(),
            cached,
        )])));
        let worker_cache = Arc::clone(&cache);
        let (completed, completion) = mpsc::channel();

        std::thread::spawn(move || {
            let refreshed = refresh_cached_generated_view(
                &worker_cache,
                "bundle_1",
                &current_fingerprint,
                vec!["tasks.md".to_owned()],
            );
            completed
                .send(refreshed)
                .expect("test receiver remains open");
        });

        let refreshed = completion
            .recv_timeout(Duration::from_secs(1))
            .expect("cached refresh must not deadlock")
            .expect("cached view exists");
        assert!(matches!(refreshed, GeneratedView::Stale { .. }));
        assert_eq!(cache.lock().get("bundle_1"), Some(&refreshed));
    }
}
