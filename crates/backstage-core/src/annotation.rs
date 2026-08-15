use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::SubjectId;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    #[default]
    Undecided,
    Approved,
    Rejected,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Disposition {
    #[default]
    Applicable,
    Obsolete,
    Superseded {
        replacement: SubjectId,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    Medium,
    High,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkRecordAnnotation {
    pub decision: Decision,
    pub disposition: Disposition,
    pub favorite: bool,
    pub todo: bool,
    pub priority: Option<Priority>,
}

impl Default for WorkRecordAnnotation {
    fn default() -> Self {
        Self {
            decision: Decision::Undecided,
            disposition: Disposition::Applicable,
            favorite: false,
            todo: false,
            priority: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", content = "value", rename_all = "snake_case")]
pub enum AnnotationCommand {
    SetDecision(Decision),
    SetDisposition(Disposition),
    SetFavorite(bool),
    SetTodo(bool),
    SetPriority(Option<Priority>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupersessionGraph {
    edges: BTreeMap<SubjectId, SubjectId>,
    max_nodes: usize,
}

impl SupersessionGraph {
    pub fn new(edges: Vec<(SubjectId, SubjectId)>, max_nodes: usize) -> Self {
        Self {
            edges: edges.into_iter().collect(),
            max_nodes,
        }
    }

    pub fn edges(&self) -> impl Iterator<Item = (&SubjectId, &SubjectId)> {
        self.edges.iter()
    }
}

pub fn transition_annotation(
    subject: &SubjectId,
    current: &WorkRecordAnnotation,
    command: AnnotationCommand,
    graph: &SupersessionGraph,
) -> Result<WorkRecordAnnotation, AnnotationRejection> {
    let mut next = current.clone();
    match command {
        AnnotationCommand::SetDecision(decision) => next.decision = decision,
        AnnotationCommand::SetDisposition(disposition) => {
            if let Disposition::Superseded { replacement } = &disposition {
                validate_supersession(subject, replacement, graph)?;
            }
            next.disposition = disposition;
        }
        AnnotationCommand::SetFavorite(favorite) => next.favorite = favorite,
        AnnotationCommand::SetTodo(todo) => next.todo = todo,
        AnnotationCommand::SetPriority(priority) => next.priority = priority,
    }
    Ok(next)
}

fn validate_supersession(
    subject: &SubjectId,
    replacement: &SubjectId,
    graph: &SupersessionGraph,
) -> Result<(), AnnotationRejection> {
    if subject == replacement {
        return Err(AnnotationRejection::SelfSupersession {
            subject: subject.clone(),
        });
    }

    let mut chain = vec![subject.clone()];
    let mut current = replacement.clone();
    let mut visited = BTreeSet::new();
    loop {
        if current == *subject {
            chain.push(current);
            return Err(AnnotationRejection::SupersessionCycle { chain });
        }
        if visited.len() >= graph.max_nodes {
            return Err(AnnotationRejection::GraphLimitExceeded {
                max_nodes: graph.max_nodes,
            });
        }
        if !visited.insert(current.clone()) {
            return Ok(());
        }
        chain.push(current.clone());
        let Some(next) = graph.edges.get(&current) else {
            return Ok(());
        };
        current = next.clone();
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum AnnotationRejection {
    #[error("a Work Record cannot supersede itself")]
    SelfSupersession { subject: SubjectId },
    #[error("supersession would create a cycle: {chain:?}")]
    SupersessionCycle { chain: Vec<SubjectId> },
    #[error("supersession validation exceeded its {max_nodes}-subject bound")]
    GraphLimitExceeded { max_nodes: usize },
}
