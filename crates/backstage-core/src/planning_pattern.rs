use regex::Regex;
use serde::Serialize;
use sha2::{Digest, Sha256};

pub const MAX_PLANNING_PATTERN_BYTES: usize = 512;
pub const MAX_PLANNING_PATTERNS: usize = 64;

const CANONICAL_DEFAULT_EXPRESSIONS: [&str; 3] = [
    r"(?:^|/)(?:PLAN|plan)\.md$",
    r"(?:^|/)(?:TDD|tdd)\.md$",
    r"(?:^|/)(?:ROADMAP|roadmap)\.md$",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanningPatternProvenance {
    Default,
    Custom,
}

impl PlanningPatternProvenance {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Custom => "custom",
        }
    }

    pub fn parse(value: &str) -> Result<Self, PlanningPatternError> {
        match value {
            "default" => Ok(Self::Default),
            "custom" => Ok(Self::Custom),
            _ => Err(PlanningPatternError::InvalidProvenance(value.to_owned())),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanningPattern {
    id: String,
    expression: String,
    ordinal: u32,
    provenance: PlanningPatternProvenance,
    #[serde(skip)]
    regex: Regex,
}

impl PartialEq for PlanningPattern {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.expression == other.expression
            && self.ordinal == other.ordinal
            && self.provenance == other.provenance
    }
}

impl Eq for PlanningPattern {}

impl PlanningPattern {
    pub fn custom(expression: impl AsRef<str>, ordinal: u32) -> Result<Self, PlanningPatternError> {
        Self::new(expression, ordinal, PlanningPatternProvenance::Custom)
    }

    pub fn persisted(
        expression: impl AsRef<str>,
        ordinal: u32,
        provenance: PlanningPatternProvenance,
    ) -> Result<Self, PlanningPatternError> {
        Self::new(expression, ordinal, provenance)
    }

    fn new(
        expression: impl AsRef<str>,
        ordinal: u32,
        provenance: PlanningPatternProvenance,
    ) -> Result<Self, PlanningPatternError> {
        let expression = expression.as_ref().trim();
        if expression.is_empty() {
            return Err(PlanningPatternError::EmptyExpression);
        }
        let bytes = expression.len();
        if bytes > MAX_PLANNING_PATTERN_BYTES {
            return Err(PlanningPatternError::ExpressionTooLong {
                bytes,
                max: MAX_PLANNING_PATTERN_BYTES,
            });
        }
        let regex = Regex::new(expression).map_err(|error| PlanningPatternError::InvalidRegex {
            message: error.to_string(),
        })?;
        Ok(Self {
            id: stable_pattern_id(expression),
            expression: expression.to_owned(),
            ordinal,
            provenance,
            regex,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn expression(&self) -> &str {
        &self.expression
    }

    pub fn ordinal(&self) -> u32 {
        self.ordinal
    }

    pub fn provenance(&self) -> PlanningPatternProvenance {
        self.provenance
    }

    pub fn matches_normalized_markdown_path(&self, path: &str) -> bool {
        is_markdown_path(path) && self.regex.is_match(path)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanningPatternConfiguration {
    pub revision: u64,
    pub patterns: Vec<PlanningPattern>,
}

pub fn canonical_planning_patterns() -> Vec<PlanningPattern> {
    CANONICAL_DEFAULT_EXPRESSIONS
        .into_iter()
        .enumerate()
        .map(|(ordinal, expression)| {
            PlanningPattern::new(
                expression,
                ordinal as u32,
                PlanningPatternProvenance::Default,
            )
            .expect("canonical planning patterns are valid")
        })
        .collect()
}

pub fn validate_planning_pattern_count(count: usize) -> Result<(), PlanningPatternError> {
    if count > MAX_PLANNING_PATTERNS {
        Err(PlanningPatternError::TooManyPatterns {
            count,
            max: MAX_PLANNING_PATTERNS,
        })
    } else {
        Ok(())
    }
}

pub fn matching_planning_patterns<'a>(
    path: &str,
    patterns: &'a [PlanningPattern],
) -> Vec<&'a PlanningPattern> {
    let normalized = normalize_project_relative_path(path);
    if !is_markdown_path(&normalized) {
        return vec![];
    }
    let mut matches = patterns
        .iter()
        .filter(|pattern| pattern.matches_normalized_markdown_path(&normalized))
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| {
        left.id()
            .cmp(right.id())
            .then_with(|| left.expression().cmp(right.expression()))
    });
    matches
}

pub fn normalize_project_relative_path(path: &str) -> String {
    let mut normalized = path.replace('\\', "/");
    while let Some(stripped) = normalized.strip_prefix("./") {
        normalized = stripped.to_owned();
    }
    normalized
}

fn is_markdown_path(path: &str) -> bool {
    path.rsplit_once('.')
        .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("md"))
}

fn stable_pattern_id(expression: &str) -> String {
    let digest = Sha256::digest(expression.as_bytes());
    format!("pattern_{digest:x}")[..32].to_owned()
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PlanningPatternError {
    #[error("planning pattern must not be empty")]
    EmptyExpression,
    #[error("planning pattern is {bytes} UTF-8 bytes; maximum is {max}")]
    ExpressionTooLong { bytes: usize, max: usize },
    #[error("planning pattern is not valid Rust regex: {message}")]
    InvalidRegex { message: String },
    #[error("planning pattern count is {count}; maximum is {max}")]
    TooManyPatterns { count: usize, max: usize },
    #[error("planning pattern provenance is invalid: {0}")]
    InvalidProvenance(String),
}
