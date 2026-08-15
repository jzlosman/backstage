#![forbid(unsafe_code)]

//! Pure domain model for Backstage.
//! This crate performs no filesystem, process, clock, storage, clipboard,
//! launcher, or Tauri I/O.

mod annotation;
mod artifact;
mod format_adapter;
mod format_adapters;
mod format_registry;
mod generated;
mod handoff;
mod markdown_syntax;
mod openspec_adapter;
mod openspec_lifecycle;
mod openspec_view;
mod path;
mod planning_pattern;
mod progress;
mod serde_u128;
mod snapshot;
mod wayfinder_adapter;
mod work_record;

pub use annotation::{
    AnnotationCommand, AnnotationRejection, Decision, Disposition, Priority, SupersessionGraph,
    WorkRecordAnnotation, transition_annotation,
};
pub use artifact::{
    ArtifactBundle, ArtifactMember, ArtifactRecognition, BundleKind, DetectorEvidence,
    EvidenceKind, MarkdownDocument, classify_project, is_supported_openspec_member,
};
pub use format_adapter::{
    AdapterFailure, AdapterHandoff, AdapterSummary, DetectedRecord, PlanningFormatAdapter,
    ProjectSourceInventory, RecordSourceCapture, SourceCaptureFailure, SourceInventoryEntry,
};
pub use format_adapters::{MarkdownAdapter, PlanningPatternAdapter};
pub use format_registry::{DetectedWorkRecord, FormatRegistry, RegistryDetection};
pub use generated::{
    GeneratedResult, GeneratedView, GenerationMode, generation_completed, generation_failed,
    previous_result, sources_changed, start_generation,
};
pub use handoff::{HandoffContext, continuation_prompt};
pub use openspec_adapter::OpenSpecAdapter;
pub use openspec_lifecycle::{OpenSpecCustody, OpenSpecPrimaryStatus, assess_openspec_status};
pub use openspec_view::{
    OpenSpecOverviewKind, OpenSpecOverviewSection, OpenSpecSource, OpenSpecTaskGroup, OpenSpecView,
    build_openspec_view,
};
pub use path::{ApprovedRoot, ArtifactPath, DomainError};
pub use planning_pattern::{
    MAX_PLANNING_PATTERN_BYTES, MAX_PLANNING_PATTERNS, PlanningPattern,
    PlanningPatternConfiguration, PlanningPatternError, PlanningPatternProvenance,
    canonical_planning_patterns, matching_planning_patterns, normalize_project_relative_path,
    validate_planning_pattern_count,
};
pub use progress::{
    OpenSpecProgress, ParseWarning, ParserProvenance, ProgressFallback, SourceLocation, TaskFact,
    TaskProgress, parse_openspec_tasks,
};
pub use serde_u128::option_decimal_string as optional_u128_decimal_string;
pub use snapshot::{
    SnapshotError, SourceFingerprint, SourceObservation, SourceSnapshot,
    fingerprint_complete_snapshots, fingerprint_snapshots,
};
pub use wayfinder_adapter::{
    ParsedWayfinderMap, WayfinderFrontier, WayfinderLocalAdapter, WayfinderMapSection,
    WayfinderMapSectionKind, WayfinderTicket, WayfinderTicketStatus, calculate_wayfinder_frontier,
    canonical_wayfinder_ticket_number, parse_wayfinder_map, parse_wayfinder_ticket,
};
pub use work_record::{
    AdapterDescriptor, Capability, CapabilityView, FactProvenance, FactValue, RecognitionLevel,
    RecordLocator, SourceClaim, SourceReference, StructuredBlock, StructuredItem,
    StructuredRelationship, SubjectId, SummaryFact, WorkRecord, WorkRecordRecognition,
    WorkRecordSource, WorkRecordWarning,
};
