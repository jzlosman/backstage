use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{SourceFingerprint, WorkRecordAnnotation};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordLocator {
    pub project_id: String,
    pub format_id: String,
    pub adapter_record_key: String,
}

impl RecordLocator {
    pub fn new(
        project_id: impl Into<String>,
        format_id: impl Into<String>,
        adapter_record_key: impl Into<String>,
    ) -> Self {
        Self {
            project_id: project_id.into(),
            format_id: format_id.into(),
            adapter_record_key: normalize_relative(&adapter_record_key.into()),
        }
    }

    pub fn subject_id(&self) -> SubjectId {
        SubjectId::for_locator(self)
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SubjectId(String);

impl SubjectId {
    pub fn for_locator(locator: &RecordLocator) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"backstage-work-record-subject-v1\0");
        hash_field(&mut hasher, locator.project_id.as_bytes());
        hash_field(&mut hasher, locator.format_id.as_bytes());
        hash_field(&mut hasher, locator.adapter_record_key.as_bytes());
        let value = format!("subject_{:x}", hasher.finalize());
        Self(value[..32].to_owned())
    }

    pub fn from_trusted(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterDescriptor {
    adapter_id: String,
    format_id: String,
    version: u32,
    precedence: u16,
}

impl AdapterDescriptor {
    pub fn new(
        adapter_id: impl Into<String>,
        format_id: impl Into<String>,
        version: u32,
        precedence: u16,
    ) -> Self {
        Self {
            adapter_id: adapter_id.into(),
            format_id: format_id.into(),
            version,
            precedence,
        }
    }

    pub fn adapter_id(&self) -> &str {
        &self.adapter_id
    }

    pub fn format_id(&self) -> &str {
        &self.format_id
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn precedence(&self) -> u16 {
        self.precedence
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecognitionLevel {
    Recognized,
    Possible,
    Plain,
}

impl RecognitionLevel {
    pub fn priority(self) -> u8 {
        match self {
            Self::Recognized => 0,
            Self::Possible => 1,
            Self::Plain => 2,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkRecordRecognition {
    pub level: RecognitionLevel,
    pub adapter_id: String,
    pub adapter_version: u32,
    pub evidence: Vec<String>,
}

impl WorkRecordRecognition {
    pub fn new(
        level: RecognitionLevel,
        descriptor: &AdapterDescriptor,
        mut evidence: Vec<String>,
    ) -> Self {
        evidence.sort();
        evidence.dedup();
        Self {
            level,
            adapter_id: descriptor.adapter_id.clone(),
            adapter_version: descriptor.version,
            evidence,
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceClaim {
    pub relative_path: String,
}

impl SourceClaim {
    pub fn new(relative_path: impl Into<String>) -> Self {
        Self {
            relative_path: normalize_relative(&relative_path.into()),
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkRecordSource {
    pub relative_path: String,
    #[serde(with = "crate::optional_u128_decimal_string")]
    pub source_modified_unix_nanos: Option<u128>,
}

impl WorkRecordSource {
    pub fn new(relative_path: impl Into<String>, source_modified_unix_nanos: Option<u128>) -> Self {
        Self {
            relative_path: normalize_relative(&relative_path.into()),
            source_modified_unix_nanos,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FactProvenance {
    pub adapter_id: String,
    pub source_paths: Vec<String>,
}

impl FactProvenance {
    pub fn new(adapter_id: impl Into<String>, source_paths: Vec<String>) -> Self {
        let mut source_paths = source_paths
            .into_iter()
            .map(|path| normalize_relative(&path))
            .collect::<Vec<_>>();
        source_paths.sort();
        source_paths.dedup();
        Self {
            adapter_id: adapter_id.into(),
            source_paths,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum FactValue {
    Text(String),
    Count(u64),
    Boolean(bool),
    Date(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryFact {
    pub key: String,
    pub label: String,
    pub value: FactValue,
    pub provenance: FactProvenance,
}

impl SummaryFact {
    pub fn new(
        key: impl Into<String>,
        label: impl Into<String>,
        value: FactValue,
        provenance: FactProvenance,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            value,
            provenance,
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkRecordWarning {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}

impl WorkRecordWarning {
    pub fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        source_path: Option<impl Into<String>>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            source_path: source_path
                .map(Into::into)
                .map(|path| normalize_relative(&path)),
            line: None,
        }
    }

    pub fn without_source(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            source_path: None,
            line: None,
        }
    }

    pub fn with_line(mut self, line: u32) -> Self {
        self.line = Some(line);
        self
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capability {
    pub id: String,
    pub label: String,
}

impl Capability {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkRecord {
    pub subject_id: SubjectId,
    pub locator: RecordLocator,
    pub display_name: String,
    pub recognition: WorkRecordRecognition,
    pub sources: Vec<WorkRecordSource>,
    pub facts: Vec<SummaryFact>,
    pub warnings: Vec<WorkRecordWarning>,
    pub capabilities: Vec<Capability>,
    #[serde(default)]
    pub annotation: WorkRecordAnnotation,
    #[serde(with = "crate::optional_u128_decimal_string")]
    pub source_modified_unix_nanos: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<SourceFingerprint>,
}

impl WorkRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        locator: RecordLocator,
        display_name: impl Into<String>,
        recognition: WorkRecordRecognition,
        mut sources: Vec<WorkRecordSource>,
        mut facts: Vec<SummaryFact>,
        mut warnings: Vec<WorkRecordWarning>,
        mut capabilities: Vec<Capability>,
    ) -> Self {
        sources.sort();
        sources.dedup();
        facts.sort_by(|left, right| {
            left.key
                .cmp(&right.key)
                .then_with(|| left.label.cmp(&right.label))
        });
        warnings.sort();
        warnings.dedup();
        capabilities.sort();
        capabilities.dedup();
        let source_modified_unix_nanos = sources
            .iter()
            .filter_map(|source| source.source_modified_unix_nanos)
            .max();
        Self {
            subject_id: locator.subject_id(),
            locator,
            display_name: display_name.into(),
            recognition,
            sources,
            facts,
            warnings,
            capabilities,
            annotation: WorkRecordAnnotation::default(),
            source_modified_unix_nanos,
            fingerprint: None,
        }
    }

    pub fn with_fingerprint(mut self, fingerprint: SourceFingerprint) -> Self {
        self.fingerprint = Some(fingerprint);
        self
    }

    pub fn with_annotation(mut self, annotation: WorkRecordAnnotation) -> Self {
        self.annotation = annotation;
        self
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceReference {
    pub relative_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}

impl SourceReference {
    pub fn new(relative_path: impl Into<String>, line: Option<u32>) -> Self {
        Self {
            relative_path: normalize_relative(&relative_path.into()),
            line,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredItem {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
    pub source: SourceReference,
    #[serde(default)]
    pub facts: Vec<SummaryFact>,
    #[serde(default)]
    pub relationships: Vec<StructuredRelationship>,
}

impl StructuredItem {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        markdown: Option<String>,
        source: SourceReference,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            markdown,
            source,
            facts: vec![],
            relationships: vec![],
        }
    }

    pub fn with_facts(mut self, mut facts: Vec<SummaryFact>) -> Self {
        facts.sort_by(|left, right| left.key.cmp(&right.key));
        self.facts = facts;
        self
    }

    pub fn with_relationships(mut self, mut relationships: Vec<StructuredRelationship>) -> Self {
        relationships.sort();
        self.relationships = relationships;
        self
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredRelationship {
    pub kind: String,
    pub target_subject_id: SubjectId,
    pub label: String,
}

impl StructuredRelationship {
    pub fn new(
        kind: impl Into<String>,
        target_subject_id: SubjectId,
        label: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            target_subject_id,
            label: label.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StructuredBlock {
    MarkdownSection {
        id: String,
        title: String,
        markdown: String,
        source: SourceReference,
    },
    FactRegister {
        id: String,
        title: String,
        facts: Vec<SummaryFact>,
    },
    Progress {
        id: String,
        label: String,
        completed: u64,
        total: u64,
    },
    ItemCollection {
        id: String,
        title: String,
        items: Vec<StructuredItem>,
    },
    RelationshipList {
        id: String,
        title: String,
        relationships: Vec<StructuredRelationship>,
    },
    EmptyState {
        id: String,
        message: String,
    },
    Warning {
        id: String,
        warning: WorkRecordWarning,
    },
    SourceList {
        id: String,
        title: String,
        sources: Vec<SourceReference>,
    },
}

impl StructuredBlock {
    pub fn markdown_section(
        id: impl Into<String>,
        title: impl Into<String>,
        markdown: impl Into<String>,
        source: SourceReference,
    ) -> Self {
        Self::MarkdownSection {
            id: id.into(),
            title: title.into(),
            markdown: markdown.into(),
            source,
        }
    }

    pub fn fact_register(
        id: impl Into<String>,
        title: impl Into<String>,
        mut facts: Vec<SummaryFact>,
    ) -> Self {
        facts.sort_by(|left, right| left.key.cmp(&right.key));
        Self::FactRegister {
            id: id.into(),
            title: title.into(),
            facts,
        }
    }

    pub fn progress(
        id: impl Into<String>,
        label: impl Into<String>,
        completed: u64,
        total: u64,
    ) -> Self {
        Self::Progress {
            id: id.into(),
            label: label.into(),
            completed,
            total,
        }
    }

    pub fn item_collection(
        id: impl Into<String>,
        title: impl Into<String>,
        items: Vec<StructuredItem>,
    ) -> Self {
        Self::ItemCollection {
            id: id.into(),
            title: title.into(),
            items,
        }
    }

    pub fn relationship_list(
        id: impl Into<String>,
        title: impl Into<String>,
        mut relationships: Vec<StructuredRelationship>,
    ) -> Self {
        relationships.sort();
        Self::RelationshipList {
            id: id.into(),
            title: title.into(),
            relationships,
        }
    }

    pub fn empty_state(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::EmptyState {
            id: id.into(),
            message: message.into(),
        }
    }

    pub fn warning(id: impl Into<String>, warning: WorkRecordWarning) -> Self {
        Self::Warning {
            id: id.into(),
            warning,
        }
    }

    pub fn source_list(
        id: impl Into<String>,
        title: impl Into<String>,
        mut sources: Vec<SourceReference>,
    ) -> Self {
        sources.sort();
        sources.dedup();
        Self::SourceList {
            id: id.into(),
            title: title.into(),
            sources,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityView {
    pub capability: Capability,
    pub blocks: Vec<StructuredBlock>,
}

impl CapabilityView {
    pub fn new(capability: Capability, blocks: Vec<StructuredBlock>) -> Self {
        Self { capability, blocks }
    }
}

fn hash_field(hasher: &mut Sha256, value: &[u8]) {
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value);
}

fn normalize_relative(path: &str) -> String {
    let mut normalized = path.replace('\\', "/");
    while let Some(stripped) = normalized.strip_prefix("./") {
        normalized = stripped.to_owned();
    }
    normalized
}
