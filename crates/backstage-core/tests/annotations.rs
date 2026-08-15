use backstage_core::{
    AnnotationCommand, AnnotationRejection, Decision, Disposition, Priority, SubjectId,
    SupersessionGraph, WorkRecordAnnotation, transition_annotation,
};

fn subject(id: &str) -> SubjectId {
    SubjectId::from_trusted(format!("subject_{id}"))
}

#[test]
fn annotation_defaults_are_sparse_and_independent() {
    let annotation = WorkRecordAnnotation::default();

    assert_eq!(annotation.decision, Decision::Undecided);
    assert_eq!(annotation.disposition, Disposition::Applicable);
    assert!(!annotation.favorite);
    assert!(!annotation.todo);
    assert_eq!(annotation.priority, None);
}

#[test]
fn decision_disposition_and_markers_update_independently() {
    let current = WorkRecordAnnotation {
        decision: Decision::Approved,
        disposition: Disposition::Obsolete,
        favorite: true,
        todo: false,
        priority: Some(Priority::High),
    };
    let graph = SupersessionGraph::new(vec![], 100);

    let rejected = transition_annotation(
        &subject("a"),
        &current,
        AnnotationCommand::SetDecision(Decision::Rejected),
        &graph,
    )
    .expect("change decision");
    assert_eq!(rejected.decision, Decision::Rejected);
    assert_eq!(rejected.disposition, Disposition::Obsolete);
    assert!(rejected.favorite);
    assert_eq!(rejected.priority, Some(Priority::High));

    let todo = transition_annotation(
        &subject("a"),
        &rejected,
        AnnotationCommand::SetTodo(true),
        &graph,
    )
    .expect("set todo");
    let medium = transition_annotation(
        &subject("a"),
        &todo,
        AnnotationCommand::SetPriority(Some(Priority::Medium)),
        &graph,
    )
    .expect("set priority");
    assert!(medium.todo);
    assert!(medium.favorite);
    assert_eq!(medium.priority, Some(Priority::Medium));
    assert_eq!(medium.decision, Decision::Rejected);
}

#[test]
fn valid_supersession_can_transition_to_obsolete_without_losing_decision() {
    let current = WorkRecordAnnotation {
        decision: Decision::Approved,
        ..WorkRecordAnnotation::default()
    };
    let graph = SupersessionGraph::new(vec![], 100);
    let replacement = subject("b");

    let superseded = transition_annotation(
        &subject("a"),
        &current,
        AnnotationCommand::SetDisposition(Disposition::Superseded {
            replacement: replacement.clone(),
        }),
        &graph,
    )
    .expect("valid replacement");
    assert_eq!(
        superseded.disposition,
        Disposition::Superseded { replacement }
    );
    assert_eq!(superseded.decision, Decision::Approved);

    let obsolete = transition_annotation(
        &subject("a"),
        &superseded,
        AnnotationCommand::SetDisposition(Disposition::Obsolete),
        &graph,
    )
    .expect("clear replacement");
    assert_eq!(obsolete.disposition, Disposition::Obsolete);
    assert_eq!(obsolete.decision, Decision::Approved);
}

#[test]
fn supersession_rejects_self_reference_without_changing_current_state() {
    let current = WorkRecordAnnotation {
        favorite: true,
        ..WorkRecordAnnotation::default()
    };
    let record = subject("a");
    let error = transition_annotation(
        &record,
        &current,
        AnnotationCommand::SetDisposition(Disposition::Superseded {
            replacement: record.clone(),
        }),
        &SupersessionGraph::new(vec![], 100),
    )
    .expect_err("self supersession must fail");

    assert_eq!(
        error,
        AnnotationRejection::SelfSupersession { subject: record }
    );
    assert_eq!(current.disposition, Disposition::Applicable);
    assert!(current.favorite);
}

#[test]
fn supersession_rejects_direct_and_transitive_cycles_with_the_conflicting_chain() {
    let a = subject("a");
    let b = subject("b");
    let c = subject("c");
    let graph = SupersessionGraph::new(vec![(b.clone(), c.clone()), (c.clone(), a.clone())], 100);

    let error = transition_annotation(
        &a,
        &WorkRecordAnnotation::default(),
        AnnotationCommand::SetDisposition(Disposition::Superseded {
            replacement: b.clone(),
        }),
        &graph,
    )
    .expect_err("cycle must fail");

    assert_eq!(
        error,
        AnnotationRejection::SupersessionCycle {
            chain: vec![a, b, c, subject("a")],
        }
    );
}

#[test]
fn supersession_validation_is_bounded() {
    let a = subject("a");
    let b = subject("b");
    let c = subject("c");
    let graph = SupersessionGraph::new(vec![(b.clone(), c)], 1);

    let error = transition_annotation(
        &a,
        &WorkRecordAnnotation::default(),
        AnnotationCommand::SetDisposition(Disposition::Superseded { replacement: b }),
        &graph,
    )
    .expect_err("bounded traversal must stop");

    assert_eq!(
        error,
        AnnotationRejection::GraphLimitExceeded { max_nodes: 1 }
    );
}
