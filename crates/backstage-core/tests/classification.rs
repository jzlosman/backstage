use backstage_core::{
    ArtifactRecognition, BundleKind, DetectorEvidence, EvidenceKind, MarkdownDocument,
    OpenSpecCustody, classify_project,
};

#[test]
fn openspec_members_are_grouped_into_one_recognized_change_bundle() {
    let evidence = vec![
        DetectorEvidence::new(
            "openspec/changes/ship-search/proposal.md",
            EvidenceKind::OpenSpecMember,
            "OpenSpec change material",
        ),
        DetectorEvidence::new(
            "openspec/changes/ship-search/tasks.md",
            EvidenceKind::OpenSpecMember,
            "OpenSpec change material",
        ),
        DetectorEvidence::new(
            "openspec/changes/ship-search/specs/search/spec.md",
            EvidenceKind::OpenSpecMember,
            "OpenSpec change material",
        ),
    ];

    let bundles = classify_project("project_1", "Workbench", evidence);

    assert_eq!(bundles.len(), 1);
    assert_eq!(bundles[0].name, "ship-search");
    assert_eq!(bundles[0].kind, BundleKind::OpenSpecChange);
    assert_eq!(
        bundles[0].recognition,
        ArtifactRecognition::Recognized {
            detector: "openspec-v1".to_owned()
        }
    );
    assert_eq!(bundles[0].members.len(), 3);
    assert!(bundles[0].id.starts_with("bundle_"));
}

#[test]
fn planning_pattern_match_is_possible_with_visible_evidence() {
    let evidence = vec![DetectorEvidence::new(
        "PLAN.md",
        EvidenceKind::PlanningPatternMatch,
        "Path matches configured planning pattern for PLAN.md",
    )];

    let bundles = classify_project("project_1", "Workbench", evidence);

    assert_eq!(bundles.len(), 1);
    assert_eq!(bundles[0].kind, BundleKind::PossibleArtifact);
    assert_eq!(
        bundles[0].recognition,
        ArtifactRecognition::Possible {
            reason: "Path matches configured planning pattern for PLAN.md".to_owned()
        }
    );
}

#[test]
fn multiple_pattern_evidence_for_one_path_produces_one_order_independent_candidate() {
    let evidence = vec![
        DetectorEvidence::new(
            "docs/PLAN.md",
            EvidenceKind::PlanningPatternMatch,
            "Planning pattern pattern_b (`.*`) matched docs/PLAN.md",
        ),
        DetectorEvidence::new(
            "docs/PLAN.md",
            EvidenceKind::PlanningPatternMatch,
            "Planning pattern pattern_a (`^docs/PLAN\\.md$`) matched docs/PLAN.md",
        ),
    ];

    let first = classify_project("project_1", "Workbench", evidence.clone());
    let second = classify_project(
        "project_1",
        "Workbench",
        evidence.into_iter().rev().collect(),
    );

    assert_eq!(first, second);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].members.len(), 1);
    assert_eq!(
        first[0].recognition,
        ArtifactRecognition::Possible {
            reason: "Planning pattern pattern_a (`^docs/PLAN\\.md$`) matched docs/PLAN.md"
                .to_owned()
        }
    );
}

#[test]
fn openspec_recognition_takes_precedence_over_matching_planning_evidence() {
    let path = "openspec/changes/search/tasks.md";
    let bundles = classify_project(
        "project_1",
        "Workbench",
        vec![
            DetectorEvidence::new(
                path,
                EvidenceKind::PlanningPatternMatch,
                "Planning pattern pattern_broad (`.*`) matched path",
            ),
            DetectorEvidence::new(
                path,
                EvidenceKind::OpenSpecMember,
                "OpenSpec change material",
            ),
        ],
    );

    assert_eq!(bundles.len(), 1);
    assert_eq!(bundles[0].kind, BundleKind::OpenSpecChange);
}

#[test]
fn ordinary_markdown_is_not_classified() {
    let evidence = vec![DetectorEvidence::new(
        "README.md",
        EvidenceKind::OrdinaryMarkdown,
        "No supported detector matched",
    )];

    assert!(classify_project("project_1", "Workbench", evidence).is_empty());
}

#[test]
fn markdown_document_identity_is_stable_and_project_scoped() {
    let first = MarkdownDocument::new("project_a", "Alpha", r"docs\Guide.MD", Some(42));
    let refreshed = MarkdownDocument::new("project_a", "Alpha", "docs/Guide.MD", Some(84));
    let other_project = MarkdownDocument::new("project_b", "Beta", "docs/Guide.MD", Some(42));

    assert_eq!(first.relative_path, "docs/Guide.MD");
    assert_eq!(first.id, refreshed.id);
    assert_ne!(first.id, other_project.id);
    assert!(first.id.starts_with("artifact_"));
}

#[test]
fn source_modified_timestamp_serializes_losslessly_and_accepts_legacy_numbers_and_null() {
    let timestamp = u128::MAX;
    let document = MarkdownDocument::new("project_a", "Alpha", "docs/Guide.md", Some(timestamp));

    let payload = serde_json::to_value(&document).expect("serialize timestamp");

    assert_eq!(
        payload["sourceModifiedUnixNanos"],
        serde_json::Value::String(timestamp.to_string())
    );

    let mut legacy_number = payload.clone();
    legacy_number["sourceModifiedUnixNanos"] = serde_json::json!(42);
    let restored: MarkdownDocument =
        serde_json::from_value(legacy_number).expect("deserialize legacy numeric timestamp");
    assert_eq!(restored.source_modified_unix_nanos, Some(42));

    let mut null_timestamp = payload;
    null_timestamp["sourceModifiedUnixNanos"] = serde_json::Value::Null;
    let restored: MarkdownDocument =
        serde_json::from_value(null_timestamp).expect("deserialize null timestamp");
    assert_eq!(restored.source_modified_unix_nanos, None);
}

#[test]
fn archived_openspec_members_keep_custody_display_name_and_full_directory_identity() {
    let bundles = classify_project(
        "project_1",
        "Workbench",
        vec![
            DetectorEvidence::new(
                "openspec/changes/ship-search/tasks.md",
                EvidenceKind::OpenSpecMember,
                "OpenSpec change material",
            ),
            DetectorEvidence::new(
                "openspec/changes/archive/2026-08-13-ship-search/tasks.md",
                EvidenceKind::OpenSpecMember,
                "OpenSpec change material",
            ),
            DetectorEvidence::new(
                "openspec/changes/archive/2026-08-12-ship-search/tasks.md",
                EvidenceKind::OpenSpecMember,
                "OpenSpec change material",
            ),
        ],
    );

    assert_eq!(bundles.len(), 3);
    assert!(bundles.iter().all(|bundle| bundle.name == "ship-search"));
    let ids = bundles
        .iter()
        .map(|bundle| bundle.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ids.len(), 3);
    assert!(bundles.iter().any(|bundle| {
        bundle.custody == Some(OpenSpecCustody::Current)
            && bundle.members[0].relative_path == "openspec/changes/ship-search/tasks.md"
    }));
    assert!(bundles.iter().any(|bundle| {
        bundle.custody
            == Some(OpenSpecCustody::Archived {
                archived_on: Some("2026-08-13".to_owned()),
            })
            && bundle.members[0].relative_path
                == "openspec/changes/archive/2026-08-13-ship-search/tasks.md"
    }));
}

#[test]
fn malformed_archive_prefix_remains_archived_without_date_or_name_stripping() {
    let bundles = classify_project(
        "project_1",
        "Workbench",
        vec![DetectorEvidence::new(
            "openspec/changes/archive/2026-02-30-ship-search/tasks.md",
            EvidenceKind::OpenSpecMember,
            "OpenSpec change material",
        )],
    );

    assert_eq!(bundles[0].name, "2026-02-30-ship-search");
    assert_eq!(
        bundles[0].custody,
        Some(OpenSpecCustody::Archived { archived_on: None })
    );
}

#[test]
fn unsupported_or_untrusted_openspec_member_paths_are_not_bundled() {
    let paths = [
        "openspec/changes/archive/tasks.md",
        "openspec/changes/change/nested/tasks.md",
        "openspec/changes/change/specs/capability/notes.md",
        "openspec/changes/change/specs/spec.md",
        "openspec/changes/archive/2026-08-13-change/specs/spec.md",
        "openspec/changes/change/../../outside/tasks.md",
        "openspec/changes//tasks.md",
    ];
    let evidence = paths
        .into_iter()
        .map(|path| {
            DetectorEvidence::new(
                path,
                EvidenceKind::OpenSpecMember,
                "OpenSpec change material",
            )
        })
        .collect();

    assert!(classify_project("project_1", "Workbench", evidence).is_empty());
}

#[test]
fn legacy_openspec_bundle_without_custody_defaults_to_current() {
    let bundle = classify_project(
        "project_1",
        "Workbench",
        vec![DetectorEvidence::new(
            "openspec/changes/search/tasks.md",
            EvidenceKind::OpenSpecMember,
            "OpenSpec change material",
        )],
    )
    .remove(0);
    let mut payload = serde_json::to_value(bundle).expect("serialize bundle");
    payload
        .as_object_mut()
        .expect("bundle object")
        .remove("custody");

    let restored: backstage_core::ArtifactBundle =
        serde_json::from_value(payload).expect("deserialize legacy bundle");

    assert_eq!(restored.custody, Some(OpenSpecCustody::Current));
}

#[test]
fn project_identity_keeps_identically_named_changes_separate() {
    let evidence = vec![DetectorEvidence::new(
        "openspec/changes/search/tasks.md",
        EvidenceKind::OpenSpecMember,
        "OpenSpec change material",
    )];

    let first = classify_project("project_a", "Alpha", evidence.clone());
    let second = classify_project("project_b", "Beta", evidence);

    assert_ne!(first[0].id, second[0].id);
    assert_eq!(first[0].project_id, "project_a");
    assert_eq!(second[0].project_id, "project_b");
}
