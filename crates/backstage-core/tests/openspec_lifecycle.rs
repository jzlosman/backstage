use backstage_core::{
    OpenSpecCustody, OpenSpecPrimaryStatus, assess_openspec_status, parse_openspec_tasks,
};

#[test]
fn custody_and_progress_are_assessed_as_separate_facts() {
    let active = parse_openspec_tasks("- [x] Done\n- [ ] Open\n");
    let done = parse_openspec_tasks("- [x] Done\n");
    let unavailable = parse_openspec_tasks("Tasks are tracked elsewhere.\n");

    assert_eq!(
        assess_openspec_status(&OpenSpecCustody::Current, &active),
        OpenSpecPrimaryStatus::Active
    );
    assert_eq!(
        assess_openspec_status(&OpenSpecCustody::Current, &done),
        OpenSpecPrimaryStatus::Done
    );
    assert_eq!(
        assess_openspec_status(&OpenSpecCustody::Current, &unavailable),
        OpenSpecPrimaryStatus::Active
    );

    for progress in [&active, &done, &unavailable] {
        assert_eq!(
            assess_openspec_status(
                &OpenSpecCustody::Archived {
                    archived_on: Some("2026-08-13".to_owned()),
                },
                progress,
            ),
            OpenSpecPrimaryStatus::Archived
        );
    }
}

#[test]
fn custody_serialization_is_explicit() {
    assert_eq!(
        serde_json::to_value(OpenSpecCustody::Current).expect("serialize current custody"),
        serde_json::json!({ "status": "current" })
    );
    assert_eq!(
        serde_json::to_value(OpenSpecCustody::Archived {
            archived_on: Some("2026-08-13".to_owned()),
        })
        .expect("serialize archived custody"),
        serde_json::json!({ "status": "archived", "archivedOn": "2026-08-13" })
    );
}
