use backstage_app_lib::storage::SqliteStore;
use backstage_core::{GeneratedResult, GenerationMode, SourceFingerprint};

fn result(fingerprint: &str, prompt_version: &str) -> GeneratedResult {
    GeneratedResult {
        text: "Generated summary".to_owned(),
        mode: GenerationMode::Summary,
        source_fingerprint: SourceFingerprint::from_trusted(fingerprint),
        included_paths: vec!["proposal.md".to_owned(), "tasks.md".to_owned()],
        generated_at: "2026-08-13T12:00:00Z".to_owned(),
        model: Some("openai-codex/gpt-5.6-sol".to_owned()),
        prompt_version: prompt_version.to_owned(),
    }
}

#[test]
fn generated_cache_round_trips_text_paths_time_model_mode_and_prompt_version() {
    let store = SqliteStore::in_memory().expect("memory store");
    let expected = result("sha256:a", "summary-v1");

    store
        .save_generated_view("bundle_1", &expected)
        .expect("save generated view");

    assert_eq!(
        store
            .find_generated_view(
                "bundle_1",
                GenerationMode::Summary,
                "sha256:a",
                "summary-v1"
            )
            .expect("find generated view"),
        Some(expected.clone())
    );
    assert_eq!(
        store
            .find_latest_generated_view("bundle_1", GenerationMode::Summary, "summary-v1")
            .expect("find latest generated view"),
        Some(expected)
    );
}

#[test]
fn cache_reuse_requires_same_bundle_mode_fingerprint_and_prompt_version() {
    let store = SqliteStore::in_memory().expect("memory store");
    store
        .save_generated_view("bundle_1", &result("sha256:a", "summary-v1"))
        .expect("save generated view");

    assert!(
        store
            .find_generated_view(
                "bundle_1",
                GenerationMode::Summary,
                "sha256:b",
                "summary-v1"
            )
            .expect("query")
            .is_none()
    );
    assert!(
        store
            .find_generated_view(
                "bundle_1",
                GenerationMode::Summary,
                "sha256:a",
                "summary-v2"
            )
            .expect("query")
            .is_none()
    );
    assert!(
        store
            .find_generated_view(
                "bundle_2",
                GenerationMode::Summary,
                "sha256:a",
                "summary-v1"
            )
            .expect("query")
            .is_none()
    );
}
