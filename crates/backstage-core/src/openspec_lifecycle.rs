use serde::{Deserialize, Serialize};

use crate::OpenSpecProgress;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum OpenSpecCustody {
    #[default]
    Current,
    Archived {
        #[serde(rename = "archivedOn")]
        archived_on: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenSpecPrimaryStatus {
    Active,
    Done,
    Archived,
}

pub fn assess_openspec_status(
    custody: &OpenSpecCustody,
    progress: &OpenSpecProgress,
) -> OpenSpecPrimaryStatus {
    match custody {
        OpenSpecCustody::Archived { .. } => OpenSpecPrimaryStatus::Archived,
        OpenSpecCustody::Current => match progress {
            OpenSpecProgress::Available(progress) if progress.remaining_count == 0 => {
                OpenSpecPrimaryStatus::Done
            }
            OpenSpecProgress::Available(_) | OpenSpecProgress::Unavailable(_) => {
                OpenSpecPrimaryStatus::Active
            }
        },
    }
}
