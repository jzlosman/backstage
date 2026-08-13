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
    ApprovedRoot, GeneratedView, GenerationMode, generation_completed, generation_failed,
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
use storage::SqliteStore;

pub struct RuntimeState {
    store: SqliteStore,
    scans: ScanCoordinator,
    generated: Mutex<BTreeMap<String, GeneratedView>>,
    scan_cancellations: Mutex<BTreeMap<String, CancellationToken>>,
    pi_cancellations: Mutex<BTreeMap<String, std::sync::Arc<std::sync::atomic::AtomicBool>>>,
    pi_temp: PathBuf,
}

#[tauri::command]
fn list_roots(state: State<'_, RuntimeState>) -> Result<Vec<ApprovedRoot>, ApiError> {
    list_approved_roots(&state.store)
}

#[tauri::command]
fn approve_root(path: String, state: State<'_, RuntimeState>) -> Result<ApprovedRoot, ApiError> {
    approve_root_path(&state.store, path)
}

#[tauri::command]
fn remove_root(root_id: String, state: State<'_, RuntimeState>) -> Result<(), ApiError> {
    remove_approved_root(&state.store, &root_id)
}

#[tauri::command]
fn scan_root(root_id: String, state: State<'_, RuntimeState>) -> Result<DiscoveryResult, ApiError> {
    let permit = state.scans.begin(&root_id);
    let root = find_root(&state.store, &root_id)?;
    let policy = ScanPolicy::default();
    let reader = ContainedReader::approve(root.path(), policy.max_file_bytes)
        .map_err(ApiError::from_error)?;
    let cancellation = CancellationToken::new();
    if let Some(previous) = state
        .scan_cancellations
        .lock()
        .insert(root_id.clone(), cancellation.clone())
    {
        previous.cancel();
    }
    let discovered = discover_projects(&reader, &policy, &cancellation);
    let index = catalog::build_index_controlled(
        &reader,
        discovered.projects.clone(),
        permit.generation,
        chrono::Utc::now().to_rfc3339(),
        discovered.warnings.clone(),
        &policy,
        &cancellation,
    );
    if discovered.cancelled {
        state.scans.cancel(&permit);
    } else if state.scans.complete(&permit, index.clone()) == CompletionDisposition::Accepted
        && let Err(error) = state.store.save_index(&index)
    {
        let mut result = discovered;
        result.warnings.push(discovery::ScanWarning {
            code: "cache_write_failed".to_owned(),
            path: root.path().to_owned(),
            message: format!("The new index is usable in memory but could not be cached: {error}"),
        });
        return Ok(result);
    }
    let mut cancellations = state.scan_cancellations.lock();
    if cancellations
        .get(&root_id)
        .is_some_and(|active| std::sync::Arc::ptr_eq(&active.0, &cancellation.0))
    {
        cancellations.remove(&root_id);
    }
    Ok(discovered)
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
    if let Some(refreshed) = refresh_cached_generated_view(
        &state.generated,
        &bundle_id,
        &live.fingerprint,
        bundle
            .bundle
            .members
            .iter()
            .map(|member| member.relative_path.clone())
            .collect(),
    ) {
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
    state.generated.lock().insert(bundle_id, view.clone());
    Ok(view)
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
        state.generated.lock().insert(bundle_id, view.clone());
        return Ok(view);
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
    state
        .generated
        .lock()
        .insert(bundle_id.clone(), generating.clone());
    let jobs = PiJobRunner::new(SystemCommandRunner, config);
    state
        .pi_cancellations
        .lock()
        .insert(request_id.clone(), jobs.cancellation_flag(&request_id));
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
            if let Err(error) = state.store.save_generated_view(&bundle_id, result) {
                GeneratedView::Failed {
                    previous: prior_result,
                    failure: format!("Summary generated but cache storage failed: {error}"),
                }
            } else {
                completed
            }
        }
        Some(GenerationJobEvent::Failed { failure, .. }) => {
            generation_failed(generating, &request_id, failure)
        }
        Some(GenerationJobEvent::Cancelled { .. }) => {
            generation_failed(generating, &request_id, "Generation cancelled")
        }
        _ => generation_failed(generating, &request_id, "Pi returned no terminal job event"),
    };
    state.pi_cancellations.lock().remove(&request_id);
    state.generated.lock().insert(bundle_id, next.clone());
    Ok(next)
}

#[tauri::command]
fn cancel_summary(request_id: String, state: State<'_, RuntimeState>) -> bool {
    state
        .pi_cancellations
        .lock()
        .get(&request_id)
        .is_some_and(|cancelled| {
            cancelled.store(true, std::sync::atomic::Ordering::Release);
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
            scan_cancellations: Mutex::new(BTreeMap::new()),
            pi_cancellations: Mutex::new(BTreeMap::new()),
            pi_temp: paths.cache_dir().join("pi"),
        })
        .invoke_handler(tauri::generate_handler![
            list_roots,
            approve_root,
            remove_root,
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

    use backstage_core::{GeneratedResult, SourceFingerprint};

    use super::*;

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
