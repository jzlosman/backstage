use backstage_core::{
    GeneratedResult, GeneratedView, GenerationMode, SourceObservation, SourceSnapshot,
    fingerprint_snapshots, generation_completed, generation_failed, sources_changed,
    start_generation,
};

fn fingerprint(content: &str) -> backstage_core::SourceFingerprint {
    let observation = SourceObservation {
        byte_len: content.len() as u64,
        modified_unix_nanos: None,
    };
    let snapshot = SourceSnapshot::from_observations(
        "tasks.md",
        content.as_bytes().to_vec(),
        observation,
        observation,
    )
    .expect("snapshot");
    fingerprint_snapshots(&[snapshot])
}

fn result(content: &str, text: &str) -> GeneratedResult {
    GeneratedResult {
        text: text.to_owned(),
        mode: GenerationMode::Summary,
        source_fingerprint: fingerprint(content),
        included_paths: vec!["tasks.md".to_owned()],
        generated_at: "2026-08-13T12:00:00Z".to_owned(),
        model: Some("openai-codex/gpt-5.6-sol".to_owned()),
        prompt_version: "summary-v1".to_owned(),
    }
}

#[test]
fn generation_moves_from_never_generated_to_current() {
    let source = fingerprint("before");
    let generating = start_generation(GeneratedView::NeverGenerated, "request-1", source.clone());
    let completed = generation_completed(
        generating,
        "request-1",
        result("before", "summary"),
        &source,
    );

    assert!(matches!(completed, GeneratedView::Current { .. }));
}

#[test]
fn regeneration_preserves_previous_content_while_running_and_after_failure() {
    let prior = result("before", "prior summary");
    let stale = GeneratedView::Stale {
        result: prior.clone(),
        changed_inputs: vec!["tasks.md".to_owned()],
    };

    let generating = start_generation(stale, "request-2", fingerprint("after"));
    let GeneratedView::Generating { previous, .. } = &generating else {
        panic!("should generate")
    };
    assert_eq!(previous.as_ref(), Some(&prior));

    let failed = generation_failed(generating, "request-2", "timeout");
    let GeneratedView::Failed { previous, failure } = failed else {
        panic!("should fail")
    };
    assert_eq!(previous, Some(prior));
    assert_eq!(failure, "timeout");
}

#[test]
fn source_change_during_generation_makes_returned_result_stale() {
    let generating = start_generation(
        GeneratedView::NeverGenerated,
        "request-1",
        fingerprint("before"),
    );

    let completed = generation_completed(
        generating,
        "request-1",
        result("before", "summary"),
        &fingerprint("after"),
    );

    assert!(matches!(completed, GeneratedView::Stale { .. }));
}

#[test]
fn superseded_completion_cannot_replace_the_newer_request() {
    let source = fingerprint("source");
    let first = start_generation(GeneratedView::NeverGenerated, "request-1", source.clone());
    let second = start_generation(first, "request-2", source.clone());

    let unchanged = generation_completed(
        second.clone(),
        "request-1",
        result("source", "old"),
        &source,
    );

    assert_eq!(unchanged, second);
}

#[test]
fn a_current_view_becomes_stale_only_when_fingerprint_changes() {
    let current = GeneratedView::Current {
        result: result("same", "summary"),
    };

    assert!(matches!(
        sources_changed(current.clone(), &fingerprint("same"), vec![]),
        GeneratedView::Current { .. }
    ));
    assert!(matches!(
        sources_changed(
            current,
            &fingerprint("changed"),
            vec!["tasks.md".to_owned()]
        ),
        GeneratedView::Stale { .. }
    ));
}
