use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{
    AnswerSupportReason, AnswerSupportStatus, FailureLabel, ProjectId, RetrievalMode, Trace,
    TraceId, TraceStatus,
};

pub const TRACE_INGESTION_SCHEMA_VERSION: &str = "1";
pub const TRACE_INGESTION_MAPPER_VERSION: &str = "1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TraceIngestionSource {
    Native,
    OtlpHttp,
}

impl TraceIngestionSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::OtlpHttp => "otlp_http",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TraceIngestionPrivacyMode {
    MetadataOnly,
    SnippetsAllowed,
    FullLocalOnly,
}

impl TraceIngestionPrivacyMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MetadataOnly => "metadata_only",
            Self::SnippetsAllowed => "snippets_allowed",
            Self::FullLocalOnly => "full_local_only",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TraceMappingStatus {
    Complete,
    PartiallyMapped,
}

impl TraceMappingStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::PartiallyMapped => "partially_mapped",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TraceIngestionDisposition {
    Created,
    Updated,
    Unchanged,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ImportedSpanOperation {
    Retrieval,
    Embedding,
    Reranking,
    Generation,
    Tool,
    Eval,
    Other,
}

impl ImportedSpanOperation {
    pub const fn canonical_label(self) -> &'static str {
        match self {
            Self::Retrieval => "Retrieval",
            Self::Embedding => "Embedding",
            Self::Reranking => "Reranking",
            Self::Generation => "Generation",
            Self::Tool => "Tool",
            Self::Eval => "Evaluation",
            Self::Other => "Other operation",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ImportedSpanKind {
    Internal,
    Server,
    Client,
    Producer,
    Consumer,
    #[default]
    Unspecified,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ImportedSpanStatus {
    Succeeded,
    Warning,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TraceIngestionModelConfig {
    pub provider: Option<String>,
    pub generation_model: Option<String>,
    pub embedding_model: Option<String>,
    pub ranker: Option<String>,
    pub configuration_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImportedEvidence {
    pub external_chunk_id: String,
    pub document_label: Option<String>,
    pub rank: u32,
    pub score: f32,
    pub lexical_score: Option<f32>,
    pub semantic_score: Option<f32>,
    pub citation_label: Option<String>,
    pub snippet: Option<String>,
    #[serde(default)]
    pub answer_support_status: AnswerSupportStatus,
    #[serde(default)]
    pub answer_support_reason: AnswerSupportReason,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImportedSpan {
    pub external_span_id: String,
    pub parent_span_id: Option<String>,
    pub operation: ImportedSpanOperation,
    #[serde(default)]
    pub kind: ImportedSpanKind,
    pub name: String,
    #[serde(with = "crate::wire_time")]
    pub started_at: OffsetDateTime,
    #[serde(with = "crate::wire_time::option")]
    pub completed_at: Option<OffsetDateTime>,
    pub latency_ms: u64,
    pub status: ImportedSpanStatus,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub error_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TraceIngestionMetadata {
    pub source: TraceIngestionSource,
    pub external_trace_id: String,
    pub schema_version: String,
    pub mapper_version: String,
    pub mapping_status: TraceMappingStatus,
    pub privacy_mode: TraceIngestionPrivacyMode,
    pub service_name: Option<String>,
    pub service_version: Option<String>,
    pub deployment_environment: Option<String>,
    pub instrumentation_scope_name: Option<String>,
    pub instrumentation_scope_version: Option<String>,
    #[serde(default)]
    pub known_failure_labels: Vec<FailureLabel>,
    #[serde(default)]
    pub status_supplied: bool,
    #[serde(default)]
    pub limitations: Vec<String>,
    pub prompt: Option<String>,
    pub retrieval_mode: Option<RetrievalMode>,
    pub top_k: Option<u32>,
    pub model_config: Option<TraceIngestionModelConfig>,
    #[serde(default)]
    pub evidence: Vec<ImportedEvidence>,
    #[serde(default)]
    pub spans: Vec<ImportedSpan>,
    pub evaluation_passed: Option<bool>,
    pub evaluation_label: Option<String>,
    #[serde(default)]
    pub timestamps_supplied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NativeTraceIngestionRequest {
    pub schema_version: String,
    pub project_id: ProjectId,
    pub external_trace_id: String,
    pub privacy_mode: TraceIngestionPrivacyMode,
    pub query: Option<String>,
    pub prompt: Option<String>,
    pub answer: Option<String>,
    pub retrieval_mode: Option<RetrievalMode>,
    pub top_k: Option<u32>,
    pub model_config: Option<TraceIngestionModelConfig>,
    #[serde(default)]
    pub retrieved_evidence: Vec<ImportedEvidence>,
    #[serde(default)]
    pub spans: Vec<ImportedSpan>,
    #[serde(default)]
    pub failure_labels: Vec<FailureLabel>,
    pub evaluation_passed: Option<bool>,
    pub evaluation_label: Option<String>,
    #[serde(default, with = "crate::wire_time::option")]
    pub started_at: Option<OffsetDateTime>,
    #[serde(default, with = "crate::wire_time::option")]
    pub completed_at: Option<OffsetDateTime>,
    pub latency_ms: Option<u64>,
    pub status: Option<TraceStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceIngestionResponse {
    pub trace_id: TraceId,
    pub external_trace_id: String,
    pub disposition: TraceIngestionDisposition,
    pub mapping_status: TraceMappingStatus,
    pub accepted_span_count: u32,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportedTraceUpsertResult {
    pub trace: Trace,
    pub disposition: TraceIngestionDisposition,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TraceMergeError {
    ExistingNotImported,
    IncomingNotImported,
    IdentityImmutable,
    InvalidStoredMapperVersion,
    InvalidIncomingMapperVersion,
    DuplicateEvidenceRank,
}

impl fmt::Display for TraceMergeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ExistingNotImported => "existing trace is not imported",
            Self::IncomingNotImported => "incoming trace is not imported",
            Self::IdentityImmutable => "import identity, schema, and privacy mode are immutable",
            Self::InvalidStoredMapperVersion => "stored mapper version is invalid",
            Self::InvalidIncomingMapperVersion => "incoming mapper version is invalid",
            Self::DuplicateEvidenceRank => "imported evidence ranks must be unique",
        })
    }
}

impl std::error::Error for TraceMergeError {}

pub fn merge_imported_trace(
    existing: &Trace,
    mut incoming: Trace,
) -> Result<Trace, TraceMergeError> {
    let incoming_had_diagnosis = incoming.diagnosis.is_some();
    let incoming_premerge_status = incoming.status;
    let old = existing
        .ingestion
        .as_ref()
        .ok_or(TraceMergeError::ExistingNotImported)?;
    let new = incoming
        .ingestion
        .as_mut()
        .ok_or(TraceMergeError::IncomingNotImported)?;
    let incoming_status_was_supplied = new.status_supplied;
    let incoming_status_signal = incoming_status_was_supplied
        || new.mapping_status == TraceMappingStatus::PartiallyMapped
        || !new.known_failure_labels.is_empty()
        || new.evaluation_passed == Some(false)
        || new
            .spans
            .iter()
            .any(|span| span.status != ImportedSpanStatus::Succeeded);
    if old.source != new.source
        || old.external_trace_id != new.external_trace_id
        || old.schema_version != new.schema_version
        || old.privacy_mode != new.privacy_mode
    {
        return Err(TraceMergeError::IdentityImmutable);
    }
    let old_mapper = old
        .mapper_version
        .parse::<u32>()
        .map_err(|_| TraceMergeError::InvalidStoredMapperVersion)?;
    let incoming_mapper = new
        .mapper_version
        .parse::<u32>()
        .map_err(|_| TraceMergeError::InvalidIncomingMapperVersion)?;
    if incoming_mapper < old_mapper {
        new.mapper_version.clone_from(&old.mapper_version);
    }

    incoming.id = existing.id;
    if new.timestamps_supplied {
        incoming.started_at = existing.started_at.min(incoming.started_at);
        incoming.completed_at = match (existing.completed_at, incoming.completed_at) {
            (Some(left), Some(right)) => Some(left.max(right)),
            (left, right) => left.or(right),
        };
    } else {
        incoming.started_at = existing.started_at;
        incoming.completed_at = existing.completed_at;
    }
    if incoming.input.is_empty() {
        incoming.input = existing.input.clone();
    }
    if incoming.output.is_none() {
        incoming.output.clone_from(&existing.output);
    }
    if new.prompt.is_none() {
        new.prompt.clone_from(&old.prompt);
    }
    if new.retrieval_mode.is_none() {
        new.retrieval_mode = old.retrieval_mode;
    }
    if new.top_k.is_none() {
        new.top_k = old.top_k;
    }
    if new.model_config.is_none() {
        new.model_config.clone_from(&old.model_config);
    }
    if new.service_name.is_none() {
        new.service_name.clone_from(&old.service_name);
    }
    if new.service_version.is_none() {
        new.service_version.clone_from(&old.service_version);
    }
    if new.deployment_environment.is_none() {
        new.deployment_environment
            .clone_from(&old.deployment_environment);
    }
    if new.instrumentation_scope_name.is_none() {
        new.instrumentation_scope_name
            .clone_from(&old.instrumentation_scope_name);
    }
    if new.instrumentation_scope_version.is_none() {
        new.instrumentation_scope_version
            .clone_from(&old.instrumentation_scope_version);
    }
    if new.evaluation_passed.is_none() {
        new.evaluation_passed = old.evaluation_passed;
    }
    if new.evaluation_label.is_none() {
        new.evaluation_label.clone_from(&old.evaluation_label);
    }
    let mut known_labels = old
        .known_failure_labels
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    known_labels.extend(new.known_failure_labels.iter().copied());
    new.known_failure_labels = known_labels.into_iter().collect();
    new.status_supplied |= old.status_supplied;

    let mut evidence = old
        .evidence
        .iter()
        .cloned()
        .map(|value| (value.external_chunk_id.clone(), value))
        .collect::<BTreeMap<_, _>>();
    evidence.extend(
        new.evidence
            .drain(..)
            .map(|value| (value.external_chunk_id.clone(), value)),
    );
    new.evidence = evidence.into_values().collect();
    new.evidence.sort_by(|left, right| {
        left.rank
            .cmp(&right.rank)
            .then_with(|| left.external_chunk_id.cmp(&right.external_chunk_id))
    });
    if new
        .evidence
        .windows(2)
        .any(|pair| pair[0].rank == pair[1].rank)
    {
        return Err(TraceMergeError::DuplicateEvidenceRank);
    }

    let mut spans = old
        .spans
        .iter()
        .cloned()
        .map(|value| (value.external_span_id.clone(), value))
        .collect::<BTreeMap<_, _>>();
    spans.extend(
        new.spans
            .drain(..)
            .map(|value| (value.external_span_id.clone(), value)),
    );
    new.spans = spans.into_values().collect();
    new.spans.sort_by(|left, right| {
        left.started_at
            .cmp(&right.started_at)
            .then_with(|| left.external_span_id.cmp(&right.external_span_id))
    });

    match new.privacy_mode {
        TraceIngestionPrivacyMode::MetadataOnly => {
            incoming.input.clear();
            incoming.output = None;
            new.prompt = None;
            for evidence in &mut new.evidence {
                evidence.document_label = None;
                evidence.snippet = None;
            }
            canonicalize_span_names(&mut new.spans);
        }
        TraceIngestionPrivacyMode::SnippetsAllowed => {
            incoming.input.clear();
            incoming.output = None;
            new.prompt = None;
            for evidence in &mut new.evidence {
                evidence.document_label = None;
            }
            canonicalize_span_names(&mut new.spans);
        }
        TraceIngestionPrivacyMode::FullLocalOnly => {}
    }

    let mut limitations = old
        .limitations
        .iter()
        .chain(&new.limitations)
        .filter(|value| !is_derived_limitation(value))
        .cloned()
        .collect::<BTreeSet<_>>();
    if incoming.input.is_empty() {
        limitations.insert("query_not_retained".to_owned());
    }
    if new.evidence.is_empty() {
        limitations.insert("retrieval_evidence_missing".to_owned());
    }
    if new.evidence.iter().all(|value| value.snippet.is_none()) {
        limitations.insert("evidence_content_not_retained".to_owned());
    }
    let span_ids = new
        .spans
        .iter()
        .map(|span| span.external_span_id.as_str())
        .collect::<BTreeSet<_>>();
    if new.spans.iter().any(|span| {
        span.parent_span_id
            .as_deref()
            .is_some_and(|parent| !span_ids.contains(parent))
    }) {
        limitations.insert("orphan_parent_span".to_owned());
    }
    new.limitations = limitations.into_iter().collect();
    new.mapping_status = if new.limitations.is_empty() {
        TraceMappingStatus::Complete
    } else {
        TraceMappingStatus::PartiallyMapped
    };

    incoming.evidence_strength = new
        .evidence
        .iter()
        .map(|value| evidence_strength(value.score))
        .min_by_key(|value| match value {
            crate::EvidenceStrength::Strong => 0,
            crate::EvidenceStrength::Medium => 1,
            crate::EvidenceStrength::Weak => 2,
        })
        .or(existing.evidence_strength);
    let diagnosis_eligible = new.privacy_mode != TraceIngestionPrivacyMode::MetadataOnly
        && !incoming.input.is_empty()
        && !new.evidence.is_empty()
        && new.evidence.iter().all(|value| value.snippet.is_some());
    if diagnosis_eligible && !incoming_had_diagnosis {
        incoming.diagnosis.clone_from(&existing.diagnosis);
        if !incoming_status_signal {
            incoming.summary.clone_from(&existing.summary);
        }
    } else if !diagnosis_eligible {
        incoming.diagnosis = None;
    }

    let mut labels = new
        .known_failure_labels
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if incoming_had_diagnosis {
        labels.extend(incoming.failure_labels.iter().copied());
    } else if diagnosis_eligible {
        labels.extend(existing.failure_labels.iter().copied());
    }
    incoming.failure_labels = labels.into_iter().collect();
    let incoming_explicit_or_diagnosed_status =
        (incoming_status_was_supplied || incoming_had_diagnosis).then_some(incoming.status);
    let incoming_status = derive_imported_trace_status(new, incoming_explicit_or_diagnosed_status);
    incoming.status = worst_status(existing.status, incoming_status);
    incoming.reruns.clone_from(&existing.reruns);
    if existing.status != incoming_premerge_status
        && worst_status(existing.status, incoming_premerge_status) == existing.status
    {
        incoming.summary.clone_from(&existing.summary);
    }
    Ok(incoming)
}

pub fn derive_imported_trace_status(
    metadata: &TraceIngestionMetadata,
    explicit_or_diagnosed_status: Option<TraceStatus>,
) -> TraceStatus {
    if explicit_or_diagnosed_status == Some(TraceStatus::Failed)
        || metadata.evaluation_passed == Some(false)
        || metadata
            .spans
            .iter()
            .any(|span| span.status == ImportedSpanStatus::Failed)
    {
        TraceStatus::Failed
    } else if explicit_or_diagnosed_status == Some(TraceStatus::Warning)
        || !metadata.known_failure_labels.is_empty()
        || metadata.mapping_status == TraceMappingStatus::PartiallyMapped
        || metadata
            .spans
            .iter()
            .any(|span| span.status == ImportedSpanStatus::Warning)
    {
        TraceStatus::Warning
    } else {
        TraceStatus::Completed
    }
}

fn canonicalize_span_names(spans: &mut [ImportedSpan]) {
    for span in spans {
        span.name = span.operation.canonical_label().to_owned();
    }
}

fn evidence_strength(score: f32) -> crate::EvidenceStrength {
    if score >= 0.75 {
        crate::EvidenceStrength::Strong
    } else if score >= 0.4 {
        crate::EvidenceStrength::Medium
    } else {
        crate::EvidenceStrength::Weak
    }
}

fn is_derived_limitation(value: &str) -> bool {
    matches!(
        value,
        "query_not_retained"
            | "retrieval_evidence_missing"
            | "evidence_content_not_retained"
            | "orphan_parent_span"
    )
}

fn worst_status(left: TraceStatus, right: TraceStatus) -> TraceStatus {
    match (left, right) {
        (TraceStatus::Failed, _) | (_, TraceStatus::Failed) => TraceStatus::Failed,
        (TraceStatus::Warning, _) | (_, TraceStatus::Warning) => TraceStatus::Warning,
        _ => TraceStatus::Completed,
    }
}
