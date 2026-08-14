use backstage_core::{
    PlanningPattern, PlanningPatternError, PlanningPatternProvenance, canonical_planning_patterns,
    matching_planning_patterns, validate_planning_pattern_count,
};

#[test]
fn custom_patterns_are_trimmed_validated_and_have_stable_ids() {
    let first = PlanningPattern::custom("  ^docs/plans/.*\\.md$  ", 7).expect("valid pattern");
    let reordered = PlanningPattern::custom("^docs/plans/.*\\.md$", 1).expect("valid pattern");

    assert_eq!(first.expression(), "^docs/plans/.*\\.md$");
    assert_eq!(first.id(), reordered.id());
    assert_eq!(first.ordinal(), 7);
    assert_eq!(first.provenance(), PlanningPatternProvenance::Custom);
}

#[test]
fn pattern_validation_rejects_empty_oversized_and_invalid_regexes() {
    assert_eq!(
        PlanningPattern::custom("  ", 0).expect_err("reject empty"),
        PlanningPatternError::EmptyExpression
    );
    assert_eq!(
        PlanningPattern::custom("é".repeat(257), 0).expect_err("reject 514 UTF-8 bytes"),
        PlanningPatternError::ExpressionTooLong {
            bytes: 514,
            max: 512
        }
    );
    assert!(matches!(
        PlanningPattern::custom("(", 0),
        Err(PlanningPatternError::InvalidRegex { .. })
    ));
}

#[test]
fn pattern_count_is_bounded_but_an_empty_set_and_broad_pattern_are_valid() {
    validate_planning_pattern_count(0).expect("empty set is valid");
    validate_planning_pattern_count(64).expect("limit is valid");
    assert_eq!(
        validate_planning_pattern_count(65).expect_err("reject over limit"),
        PlanningPatternError::TooManyPatterns { count: 65, max: 64 }
    );
    PlanningPattern::custom(".*", 0).expect("broad regex is valid");
}

#[test]
fn canonical_defaults_match_only_supported_names_at_any_depth() {
    let defaults = canonical_planning_patterns();
    assert_eq!(defaults.len(), 3);
    assert!(
        defaults
            .iter()
            .all(|pattern| pattern.provenance() == PlanningPatternProvenance::Default)
    );

    for path in [
        "PLAN.md",
        "plan.md",
        "docs/TDD.md",
        "nested/deeper/tdd.md",
        "ROADMAP.md",
        "plans/roadmap.md",
    ] {
        assert_eq!(
            matching_planning_patterns(path, &defaults).len(),
            1,
            "{path}"
        );
    }
    for path in [
        "MY_PLAN.md",
        "Plan.md",
        "ROADMAP.md.bak",
        "docs/roadmaps.md",
        "PLAN.txt",
        "docs/PLAN.txt",
    ] {
        assert!(
            matching_planning_patterns(path, &defaults).is_empty(),
            "{path}"
        );
    }
}

#[test]
fn only_markdown_paths_are_matched_and_match_order_ignores_display_ordinal() {
    let broad = PlanningPattern::custom(".*", 99).expect("broad");
    assert!(matching_planning_patterns("notes.txt", std::slice::from_ref(&broad)).is_empty());

    let exact = PlanningPattern::custom("^docs/PLAN\\.md$", 0).expect("exact");
    let first_order = [broad.clone(), exact.clone()];
    let second_order = [exact, broad];
    let first = matching_planning_patterns("./docs\\PLAN.md", &first_order);
    let second = matching_planning_patterns("docs/PLAN.md", &second_order);
    let first_ids = first.iter().map(|pattern| pattern.id()).collect::<Vec<_>>();
    let second_ids = second
        .iter()
        .map(|pattern| pattern.id())
        .collect::<Vec<_>>();

    assert_eq!(first_ids, second_ids);
    assert_eq!(first_ids.len(), 2);
}
