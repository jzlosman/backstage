use backstage_core::{
    ArtifactRecognition, BundleKind, DetectorEvidence, EvidenceKind, MarkdownDocument,
    classify_project,
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
fn candidate_named_markdown_is_possible_with_visible_evidence() {
    let evidence = vec![DetectorEvidence::new(
        "PLAN.md",
        EvidenceKind::CandidateName,
        "Filename matches configured planning candidate PLAN.md",
    )];

    let bundles = classify_project("project_1", "Workbench", evidence);

    assert_eq!(bundles.len(), 1);
    assert_eq!(bundles[0].kind, BundleKind::PossibleArtifact);
    assert_eq!(
        bundles[0].recognition,
        ArtifactRecognition::Possible {
            reason: "Filename matches configured planning candidate PLAN.md".to_owned()
        }
    );
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
