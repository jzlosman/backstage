#![forbid(unsafe_code)]

//! Pure domain model for Backstage.
//! This crate performs no filesystem, process, clock, storage, clipboard,
//! launcher, or Tauri I/O.

mod artifact;
mod generated;
mod handoff;
mod markdown_syntax;
mod openspec_view;
mod path;
mod progress;
mod snapshot;

pub use artifact::{
    ArtifactBundle, ArtifactMember, ArtifactRecognition, BundleKind, DetectorEvidence,
    EvidenceKind, MarkdownDocument, classify_project,
};
pub use generated::{
    GeneratedResult, GeneratedView, GenerationMode, generation_completed, generation_failed,
    previous_result, sources_changed, start_generation,
};
pub use handoff::{HandoffContext, continuation_prompt};
pub use openspec_view::{
    OpenSpecOverviewKind, OpenSpecOverviewSection, OpenSpecSource, OpenSpecTaskGroup, OpenSpecView,
    build_openspec_view,
};
pub use path::{ApprovedRoot, ArtifactPath, DomainError};
pub use progress::{
    OpenSpecProgress, ParseWarning, ParserProvenance, ProgressFallback, SourceLocation, TaskFact,
    TaskProgress, parse_openspec_tasks,
};
pub use snapshot::{
    SnapshotError, SourceFingerprint, SourceObservation, SourceSnapshot,
    fingerprint_complete_snapshots, fingerprint_snapshots,
};
