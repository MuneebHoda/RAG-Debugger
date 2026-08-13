use std::collections::{HashMap, HashSet};

use rag_debugger_core::*;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
    diagnosis::{diagnose_retrieval, legacy_failure_labels},
    retrieval::ensure_response_answerability,
};

pub const MAX_EVIDENCE: usize = 100;
pub const MAX_SPANS: usize = 256;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ImportValidationError {
    UnsupportedSchema,
    PrivacyModeNotPermitted,
    InvalidIdentifier,
    InvalidLabel,
    InvalidContent,
    InvalidNumber,
    CollectionLimit,
    DuplicateEvidence,
    DuplicateSpan,
    InvalidSpanHierarchy,
}

impl ImportValidationError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnsupportedSchema => "unsupported_schema_version",
            Self::PrivacyModeNotPermitted => "privacy_mode_not_permitted",
            Self::InvalidIdentifier => "invalid_identifier",
            Self::InvalidLabel => "invalid_label",
            Self::InvalidContent => "invalid_content",
            Self::InvalidNumber => "invalid_number",
            Self::CollectionLimit => "collection_limit_exceeded",
            Self::DuplicateEvidence => "duplicate_evidence_id",
            Self::DuplicateSpan => "duplicate_span_id",
            Self::InvalidSpanHierarchy => "invalid_span_hierarchy",
        }
    }
}

pub fn build_native_trace(
    project: &Project,
    mut request: NativeTraceIngestionRequest,
    retrieval_config: &RetrievalConfig,
    debugger_config: &DebuggerConfig,
) -> Result<Trace, ImportValidationError> {
    validate_native_request(project, &request)?;
    normalize_labels(&mut request);
    apply_privacy(&mut request);

    let timestamps_supplied = request.started_at.is_some() || request.completed_at.is_some();
    let now = OffsetDateTime::now_utc();
    let started_at = request.started_at.unwrap_or(now);
    let completed_at = request.completed_at.or_else(|| {
        request
            .latency_ms
            .and_then(|value| i64::try_from(value).ok())
            .and_then(|value| started_at.checked_add(Duration::milliseconds(value)))
    });
    let mut limitations = Vec::new();
    if request.query.is_none() {
        limitations.push("query_not_retained".to_owned());
    }
    if request.retrieved_evidence.is_empty() {
        limitations.push("retrieval_evidence_missing".to_owned());
    }
    if request
        .retrieved_evidence
        .iter()
        .all(|evidence| evidence.snippet.is_none())
    {
        limitations.push("evidence_content_not_retained".to_owned());
    }
    if request.privacy_mode != TraceIngestionPrivacyMode::FullLocalOnly && !request.spans.is_empty()
    {
        limitations.push("span_names_not_retained".to_owned());
    }
    limitations.sort();
    let mapping_status = if limitations.is_empty() {
        TraceMappingStatus::Complete
    } else {
        TraceMappingStatus::PartiallyMapped
    };
    let evidence_strength = request
        .retrieved_evidence
        .iter()
        .map(|evidence| strength(evidence.score))
        .min_by_key(|value| match value {
            EvidenceStrength::Strong => 0,
            EvidenceStrength::Medium => 1,
            EvidenceStrength::Weak => 2,
        })
        .unwrap_or(EvidenceStrength::Weak);
    let requested_status = request.status;
    let status_supplied = request.status.is_some();
    let known_failure_labels = request.failure_labels.clone();
    let metadata = TraceIngestionMetadata {
        source: TraceIngestionSource::Native,
        external_trace_id: request.external_trace_id,
        schema_version: request.schema_version,
        mapper_version: TRACE_INGESTION_MAPPER_VERSION.to_owned(),
        mapping_status,
        privacy_mode: request.privacy_mode,
        service_name: None,
        service_version: None,
        deployment_environment: None,
        instrumentation_scope_name: None,
        instrumentation_scope_version: None,
        known_failure_labels,
        status_supplied,
        limitations,
        prompt: request.prompt,
        retrieval_mode: request.retrieval_mode,
        top_k: request.top_k,
        model_config: request.model_config,
        evidence: request.retrieved_evidence,
        spans: request.spans,
        evaluation_passed: request.evaluation_passed,
        evaluation_label: request.evaluation_label,
        timestamps_supplied,
    };
    let derived_status = derive_imported_trace_status(&metadata, requested_status);
    let mut trace = Trace {
        id: TraceId(Uuid::now_v7()),
        project_id: project.id,
        input: request.query.unwrap_or_default(),
        output: request.answer,
        started_at,
        completed_at,
        retrieval_runs: Vec::new(),
        generation: None,
        failure_labels: request.failure_labels,
        source_run_id: None,
        summary: String::new(),
        status: derived_status,
        evidence_strength: Some(evidence_strength),
        spans: Vec::new(),
        retrieval: None,
        reruns: Vec::new(),
        diagnosis: None,
        ingestion: Some(metadata),
    };
    trace.summary = imported_summary(trace.status, mapping_status);
    add_diagnosis(&mut trace, retrieval_config, debugger_config);
    Ok(trace)
}

pub fn add_diagnosis(
    trace: &mut Trace,
    retrieval_config: &RetrievalConfig,
    debugger_config: &DebuggerConfig,
) {
    let Some(metadata) = trace.ingestion.as_ref() else {
        return;
    };
    let known_failure_labels = metadata.known_failure_labels.clone();
    if trace.input.is_empty()
        || metadata.privacy_mode == TraceIngestionPrivacyMode::MetadataOnly
        || metadata.evidence.is_empty()
        || metadata
            .evidence
            .iter()
            .any(|evidence| evidence.snippet.is_none())
    {
        trace.diagnosis = None;
        trace.failure_labels = known_failure_labels;
        trace.summary = imported_summary(trace.status, metadata.mapping_status);
        return;
    }
    let response = synthetic_response(trace, metadata);
    let response = ensure_response_answerability(response, retrieval_config, debugger_config);
    let mut diagnosis = response
        .diagnosis
        .clone()
        .unwrap_or_else(|| diagnose_retrieval(&response, debugger_config, None));
    diagnosis.score_explanations.clear();
    let mut failure_labels = known_failure_labels
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    failure_labels.extend(legacy_failure_labels(&diagnosis));
    trace.failure_labels = failure_labels.into_iter().collect();
    let diagnosis_status = match diagnosis.outcome {
        DiagnosisOutcome::Strong => TraceStatus::Completed,
        DiagnosisOutcome::Mixed | DiagnosisOutcome::Weak => TraceStatus::Warning,
        DiagnosisOutcome::Failing => TraceStatus::Failed,
    };
    trace.status = retain_worst_status(trace.status, diagnosis_status);
    trace.summary = if trace.status == diagnosis_status {
        diagnosis.summary.clone()
    } else {
        imported_summary(trace.status, metadata.mapping_status)
    };
    trace.diagnosis = Some(diagnosis);
}

pub fn validate_native_request(
    project: &Project,
    request: &NativeTraceIngestionRequest,
) -> Result<(), ImportValidationError> {
    if request.schema_version != TRACE_INGESTION_SCHEMA_VERSION {
        return Err(ImportValidationError::UnsupportedSchema);
    }
    if !privacy_permitted(project.privacy_mode, request.privacy_mode) {
        return Err(ImportValidationError::PrivacyModeNotPermitted);
    }
    validate_id(&request.external_trace_id)?;
    if request.retrieved_evidence.len() > MAX_EVIDENCE || request.spans.len() > MAX_SPANS {
        return Err(ImportValidationError::CollectionLimit);
    }
    validate_content(request.query.as_deref(), 8_192)?;
    validate_content(request.prompt.as_deref(), 8_192)?;
    validate_content(request.answer.as_deref(), 8_192)?;
    validate_label(request.evaluation_label.as_deref(), 256)?;
    if request
        .top_k
        .is_some_and(|value| !(1..=25).contains(&value))
    {
        return Err(ImportValidationError::InvalidNumber);
    }
    if request
        .latency_ms
        .is_some_and(|value| i64::try_from(value).is_err())
    {
        return Err(ImportValidationError::InvalidNumber);
    }
    if let (Some(start), Some(latency)) = (request.started_at, request.latency_ms) {
        let latency = i64::try_from(latency).map_err(|_| ImportValidationError::InvalidNumber)?;
        if start.checked_add(Duration::milliseconds(latency)).is_none() {
            return Err(ImportValidationError::InvalidNumber);
        }
    }
    if let (Some(start), Some(end)) = (request.started_at, request.completed_at) {
        if end < start {
            return Err(ImportValidationError::InvalidNumber);
        }
    }
    if let Some(config) = &request.model_config {
        for value in [
            config.provider.as_deref(),
            config.generation_model.as_deref(),
            config.embedding_model.as_deref(),
            config.ranker.as_deref(),
            config.configuration_label.as_deref(),
        ] {
            validate_label(value, 256)?;
        }
    }
    let mut evidence_ids = HashSet::new();
    let mut ranks = HashSet::new();
    for evidence in &request.retrieved_evidence {
        validate_id(&evidence.external_chunk_id)?;
        validate_label(evidence.document_label.as_deref(), 512)?;
        validate_label(evidence.citation_label.as_deref(), 256)?;
        validate_content(evidence.snippet.as_deref(), 280)?;
        if !(1..=100).contains(&evidence.rank)
            || !valid_score(evidence.score)
            || evidence
                .lexical_score
                .is_some_and(|value| !valid_score(value))
            || evidence
                .semantic_score
                .is_some_and(|value| !valid_score(value))
        {
            return Err(ImportValidationError::InvalidNumber);
        }
        if !evidence_ids.insert(&evidence.external_chunk_id) || !ranks.insert(evidence.rank) {
            return Err(ImportValidationError::DuplicateEvidence);
        }
    }
    validate_spans(&request.spans)
}

fn validate_spans(spans: &[ImportedSpan]) -> Result<(), ImportValidationError> {
    let mut by_id = HashMap::new();
    for span in spans {
        validate_id(&span.external_span_id)?;
        if let Some(parent) = &span.parent_span_id {
            validate_id(parent)?;
            if parent == &span.external_span_id {
                return Err(ImportValidationError::InvalidSpanHierarchy);
            }
        }
        validate_label(Some(&span.name), 256)?;
        for value in [
            span.provider.as_deref(),
            span.model.as_deref(),
            span.error_type.as_deref(),
        ] {
            validate_label(value, 256)?;
        }
        if span.completed_at.is_some_and(|end| end < span.started_at) {
            return Err(ImportValidationError::InvalidNumber);
        }
        if by_id.insert(span.external_span_id.as_str(), span).is_some() {
            return Err(ImportValidationError::DuplicateSpan);
        }
    }
    for span in spans {
        let mut seen = HashSet::new();
        let mut current = Some(span.external_span_id.as_str());
        while let Some(id) = current {
            if !seen.insert(id) {
                return Err(ImportValidationError::InvalidSpanHierarchy);
            }
            current = by_id
                .get(id)
                .and_then(|value| value.parent_span_id.as_deref());
        }
    }
    Ok(())
}

fn apply_privacy(request: &mut NativeTraceIngestionRequest) {
    match request.privacy_mode {
        TraceIngestionPrivacyMode::MetadataOnly => {
            request.query = None;
            request.prompt = None;
            request.answer = None;
            for evidence in &mut request.retrieved_evidence {
                evidence.document_label = None;
                evidence.snippet = None;
            }
        }
        TraceIngestionPrivacyMode::SnippetsAllowed => {
            request.query = None;
            request.prompt = None;
            request.answer = None;
            for evidence in &mut request.retrieved_evidence {
                evidence.document_label = None;
            }
        }
        TraceIngestionPrivacyMode::FullLocalOnly => {}
    }
    if request.privacy_mode != TraceIngestionPrivacyMode::FullLocalOnly {
        for span in &mut request.spans {
            span.name = span.operation.canonical_label().to_owned();
        }
    }
}

fn normalize_labels(request: &mut NativeTraceIngestionRequest) {
    normalize_optional_label(&mut request.evaluation_label);
    if let Some(config) = &mut request.model_config {
        normalize_optional_label(&mut config.provider);
        normalize_optional_label(&mut config.generation_model);
        normalize_optional_label(&mut config.embedding_model);
        normalize_optional_label(&mut config.ranker);
        normalize_optional_label(&mut config.configuration_label);
    }
    for evidence in &mut request.retrieved_evidence {
        normalize_optional_label(&mut evidence.document_label);
        normalize_optional_label(&mut evidence.citation_label);
    }
    for span in &mut request.spans {
        span.name = span.name.trim().to_owned();
        normalize_optional_label(&mut span.provider);
        normalize_optional_label(&mut span.model);
        normalize_optional_label(&mut span.error_type);
    }
}

fn normalize_optional_label(value: &mut Option<String>) {
    if let Some(value) = value {
        *value = value.trim().to_owned();
    }
}

fn privacy_permitted(project: PrivacyMode, requested: TraceIngestionPrivacyMode) -> bool {
    match project {
        PrivacyMode::LocalOnly => true,
        PrivacyMode::ExplicitSnippetSync => requested != TraceIngestionPrivacyMode::FullLocalOnly,
        PrivacyMode::RedactedCloudSync => requested == TraceIngestionPrivacyMode::MetadataOnly,
    }
}

fn validate_id(value: &str) -> Result<(), ImportValidationError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(ImportValidationError::InvalidIdentifier);
    }
    Ok(())
}

fn validate_label(value: Option<&str>, max: usize) -> Result<(), ImportValidationError> {
    if value.is_some_and(|value| {
        value.trim().is_empty() || value.len() > max || value.chars().any(char::is_control)
    }) {
        return Err(ImportValidationError::InvalidLabel);
    }
    Ok(())
}

fn validate_content(value: Option<&str>, max: usize) -> Result<(), ImportValidationError> {
    if value.is_some_and(|value| value.len() > max || value.chars().any(|c| c == '\0')) {
        return Err(ImportValidationError::InvalidContent);
    }
    Ok(())
}

fn valid_score(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

pub(crate) fn strength(score: f32) -> EvidenceStrength {
    if score >= 0.75 {
        EvidenceStrength::Strong
    } else if score >= 0.4 {
        EvidenceStrength::Medium
    } else {
        EvidenceStrength::Weak
    }
}

fn imported_summary(status: TraceStatus, mapping: TraceMappingStatus) -> String {
    match (status, mapping) {
        (TraceStatus::Failed, _) => "Imported trace contains a failed operation.".to_owned(),
        (_, TraceMappingStatus::PartiallyMapped) => {
            "Imported trace is only partially mapped; diagnosis is limited.".to_owned()
        }
        (TraceStatus::Warning, _) => "Imported trace contains warning signals.".to_owned(),
        _ => "Imported trace completed with mapped retrieval evidence.".to_owned(),
    }
}

fn retain_worst_status(left: TraceStatus, right: TraceStatus) -> TraceStatus {
    match (left, right) {
        (TraceStatus::Failed, _) | (_, TraceStatus::Failed) => TraceStatus::Failed,
        (TraceStatus::Warning, _) | (_, TraceStatus::Warning) => TraceStatus::Warning,
        _ => TraceStatus::Completed,
    }
}

fn synthetic_response(trace: &Trace, metadata: &TraceIngestionMetadata) -> RetrievalQueryResponse {
    let source_id = SourceId(correlation_uuid(&metadata.external_trace_id, "source"));
    let source = Source {
        id: source_id,
        project_id: trace.project_id,
        name: "Imported trace evidence".to_owned(),
        kind: SourceKind::FileSet {
            root_hint: "imported".to_owned(),
        },
        sync_policy: SourceSyncPolicy::Manual,
        chunking: ChunkingConfig::default(),
    };
    let hits = metadata
        .evidence
        .iter()
        .map(|evidence| {
            let document_id = DocumentId(correlation_uuid(
                &metadata.external_trace_id,
                evidence.document_label.as_deref().unwrap_or("document"),
            ));
            let chunk_id = ChunkId(correlation_uuid(
                &metadata.external_trace_id,
                &evidence.external_chunk_id,
            ));
            let snippet = evidence.snippet.clone().unwrap_or_default();
            let citation = RetrievalCitation {
                label: evidence
                    .citation_label
                    .clone()
                    .unwrap_or_else(|| format!("E{}", evidence.rank)),
                chunk_id,
                document_id,
                document_path: evidence
                    .document_label
                    .clone()
                    .unwrap_or_else(|| "Imported evidence".to_owned()),
                chunk_ordinal: evidence.rank - 1,
                section_title: None,
                checksum_prefix: "imported".to_owned(),
                snippet: snippet.clone(),
            };
            RetrievalQueryHit {
                rank: evidence.rank,
                score: evidence.score,
                chunk: ChunkPreview {
                    id: chunk_id,
                    document_id,
                    ordinal: evidence.rank - 1,
                    text: snippet.clone(),
                    token_count: snippet.split_whitespace().count() as u32,
                    byte_range: ByteRange {
                        start: 0,
                        end: snippet.len() as u64,
                    },
                    checksum: "imported".to_owned(),
                    strategy: ChunkingStrategy::Structured,
                    section_title: None,
                    split_reason: ChunkSplitReason::DocumentEnd,
                    quality_flags: Vec::new(),
                    is_duplicate: false,
                    text_density: 1.0,
                    evidence_score_hint: evidence.score,
                },
                document: Document {
                    id: document_id,
                    source_id,
                    path: citation.document_path.clone(),
                    mime_type: None,
                    checksum: "imported".to_owned(),
                    byte_size: snippet.len() as u64,
                    profile: DocumentProfile::General,
                    extraction_quality: ExtractionQuality::Unknown,
                    warnings: Vec::new(),
                },
                source: source.clone(),
                matched_terms: Vec::new(),
                score_breakdown: RetrievalScoreBreakdown {
                    semantic: evidence.semantic_score.unwrap_or(0.0),
                    lexical: evidence.lexical_score.unwrap_or(0.0),
                    phrase: 0.0,
                    section: 0.0,
                    path: 0.0,
                    metadata: 0.0,
                },
                normalized_score_breakdown: RetrievalScoreBreakdown {
                    semantic: evidence.semantic_score.unwrap_or(0.0),
                    lexical: evidence.lexical_score.unwrap_or(0.0),
                    phrase: 0.0,
                    section: 0.0,
                    path: 0.0,
                    metadata: 0.0,
                },
                snippet,
                citation,
                quality_flags: Vec::new(),
                evidence_strength: strength(evidence.score),
                duplicate_count: 0,
                answer_support: AnswerSupportAssessment {
                    status: evidence.answer_support_status,
                    reason: evidence.answer_support_reason,
                    matched_body_term_count: 0,
                    query_term_count: 0,
                    body_term_coverage: 0.0,
                },
            }
        })
        .collect::<Vec<_>>();
    let citations = hits
        .iter()
        .filter(|hit| {
            metadata
                .evidence
                .iter()
                .any(|evidence| evidence.rank == hit.rank && evidence.citation_label.is_some())
        })
        .map(|hit| hit.citation.clone())
        .collect::<Vec<_>>();
    RetrievalQueryResponse {
        run: RetrievalQueryRun {
            id: RetrievalQueryRunId(Uuid::now_v7()),
            query: trace.input.clone(),
            top_k: metadata.top_k.unwrap_or(hits.len().max(1) as u32),
            retrieval_mode: metadata.retrieval_mode.unwrap_or_default(),
            latency_ms: trace.completed_at.map_or(0, |end| {
                (end - trace.started_at).whole_milliseconds().max(0) as u64
            }),
            created_at: trace.started_at,
        },
        answer: ExtractiveAnswer {
            status: if trace
                .output
                .as_deref()
                .is_some_and(|answer| !answer.is_empty())
            {
                ExtractiveAnswerStatus::Answered
            } else {
                ExtractiveAnswerStatus::InsufficientEvidence
            },
            text: trace.output.clone().unwrap_or_default(),
            citations,
        },
        hits,
        embedding_status: RetrievalEmbeddingStatus {
            readiness: RetrievalEmbeddingReadiness::Ready,
            required: false,
            model: EmbeddingModelInfo::default(),
            total_chunks: metadata.evidence.len() as u32,
            indexed_chunks: metadata.evidence.len() as u32,
            missing_chunks: 0,
            stale_chunks: 0,
        },
        diagnosis: None,
    }
}

fn correlation_uuid(trace_id: &str, value: &str) -> Uuid {
    let mut bytes = [0_u8; 16];
    for (index, byte) in trace_id.bytes().chain(value.bytes()).enumerate() {
        let slot = index % 16;
        bytes[slot] = bytes[slot].wrapping_mul(31).wrapping_add(byte);
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reports::{
        build_trace_debug_report, render_debug_report_markdown, DebugReportBuildContext,
    };

    #[test]
    fn metadata_privacy_strips_all_content_before_trace_creation() {
        let project = project(PrivacyMode::LocalOnly);
        let mut request = request(TraceIngestionPrivacyMode::MetadataOnly);
        request.query = Some("private query".to_owned());
        request.prompt = Some("private prompt".to_owned());
        request.answer = Some("private answer".to_owned());
        request.retrieved_evidence[0].snippet = Some("private snippet".to_owned());
        request.spans = vec![span("span-1", None)];
        request.spans[0].name = "private span name".to_owned();

        let trace = build_native_trace(
            &project,
            request,
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("valid metadata import");

        assert!(trace.input.is_empty());
        assert!(trace.output.is_none());
        let imported = trace.ingestion.expect("imported metadata");
        assert!(imported.prompt.is_none());
        assert!(imported.evidence[0].snippet.is_none());
        assert!(imported.evidence[0].document_label.is_none());
        assert_eq!(imported.spans[0].name, "Retrieval");
        assert!(imported
            .limitations
            .iter()
            .any(|value| value == "span_names_not_retained"));
        assert!(trace.diagnosis.is_none());
    }

    #[test]
    fn full_local_import_runs_existing_diagnosis() {
        let trace = build_native_trace(
            &project(PrivacyMode::LocalOnly),
            request(TraceIngestionPrivacyMode::FullLocalOnly),
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("valid snippet import");
        assert!(trace
            .diagnosis
            .as_ref()
            .is_some_and(|diagnosis| diagnosis.score_explanations.is_empty()));
    }

    #[test]
    fn failed_evaluation_makes_a_new_import_failed() {
        let mut request = request(TraceIngestionPrivacyMode::FullLocalOnly);
        request.evaluation_passed = Some(false);

        let trace = build_native_trace(
            &project(PrivacyMode::LocalOnly),
            request,
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("failed evaluation import");

        assert_eq!(trace.status, TraceStatus::Failed);
        assert_eq!(trace.summary, "Imported trace contains a failed operation.");
    }

    #[test]
    fn warning_span_makes_a_new_import_warning() {
        let mut request = request(TraceIngestionPrivacyMode::FullLocalOnly);
        let mut warning = span("warning-span", None);
        warning.status = ImportedSpanStatus::Warning;
        request.spans = vec![warning];

        let trace = build_native_trace(
            &project(PrivacyMode::LocalOnly),
            request,
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("warning span import");

        assert_eq!(trace.status, TraceStatus::Warning);
        assert_eq!(trace.summary, "Imported trace contains warning signals.");
    }

    #[test]
    fn explicit_completed_status_cannot_hide_failure_signals() {
        let mut request = request(TraceIngestionPrivacyMode::FullLocalOnly);
        let mut failed = span("failed-span", None);
        failed.status = ImportedSpanStatus::Failed;
        request.spans = vec![failed];
        request.status = Some(TraceStatus::Completed);

        let trace = build_native_trace(
            &project(PrivacyMode::LocalOnly),
            request,
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("contradictory status import");

        assert_eq!(trace.status, TraceStatus::Failed);
    }

    #[test]
    fn supported_import_keeps_weak_candidates_as_secondary_warnings() {
        let mut request = request(TraceIngestionPrivacyMode::FullLocalOnly);
        request.top_k = Some(2);
        request.retrieved_evidence.push(ImportedEvidence {
            external_chunk_id: "chunk-2".to_owned(),
            document_label: Some("other.md".to_owned()),
            rank: 2,
            score: 0.1,
            lexical_score: Some(0.1),
            semantic_score: Some(0.1),
            citation_label: None,
            snippet: Some("An unrelated lower-ranked candidate.".to_owned()),
            answer_support_status: AnswerSupportStatus::Unassessed,
            answer_support_reason: AnswerSupportReason::Unassessed,
        });
        let trace = build_native_trace(
            &project(PrivacyMode::LocalOnly),
            request,
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("valid mixed import");
        let diagnosis = trace.diagnosis.expect("diagnosis");

        assert!(diagnosis.primary_issue.is_none());
        assert!(diagnosis
            .failures
            .iter()
            .any(|failure| failure.code == DiagnosisFailureCode::WeakEvidence));
    }

    #[test]
    fn validation_rejects_privacy_bounds_and_invalid_hierarchy() {
        let local = project(PrivacyMode::LocalOnly);
        let mut invalid = request(TraceIngestionPrivacyMode::SnippetsAllowed);
        invalid.retrieved_evidence[0].score = f32::NAN;
        assert_eq!(
            validate_native_request(&local, &invalid),
            Err(ImportValidationError::InvalidNumber)
        );

        let mut duplicate = request(TraceIngestionPrivacyMode::SnippetsAllowed);
        duplicate
            .retrieved_evidence
            .push(duplicate.retrieved_evidence[0].clone());
        assert_eq!(
            validate_native_request(&local, &duplicate),
            Err(ImportValidationError::DuplicateEvidence)
        );

        let mut cycle = request(TraceIngestionPrivacyMode::SnippetsAllowed);
        cycle.spans = vec![span("one", Some("two")), span("two", Some("one"))];
        assert_eq!(
            validate_native_request(&local, &cycle),
            Err(ImportValidationError::InvalidSpanHierarchy)
        );

        let mut invalid_timestamp = request(TraceIngestionPrivacyMode::SnippetsAllowed);
        let mut invalid_span = span("bad-time", None);
        invalid_span.completed_at = Some(invalid_span.started_at - Duration::milliseconds(1));
        invalid_timestamp.spans = vec![invalid_span];
        assert_eq!(
            validate_native_request(&local, &invalid_timestamp),
            Err(ImportValidationError::InvalidNumber)
        );

        let cloud = project(PrivacyMode::RedactedCloudSync);
        assert_eq!(
            validate_native_request(&cloud, &request(TraceIngestionPrivacyMode::SnippetsAllowed)),
            Err(ImportValidationError::PrivacyModeNotPermitted)
        );
    }

    #[test]
    fn privacy_matrix_discards_only_content_forbidden_by_the_selected_mode() {
        let local = project(PrivacyMode::LocalOnly);
        let mut snippets = request(TraceIngestionPrivacyMode::SnippetsAllowed);
        snippets.prompt = Some("private prompt".to_owned());
        let snippets = build_native_trace(
            &local,
            snippets,
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("snippet import");
        let snippet_metadata = snippets.ingestion.as_ref().expect("snippet metadata");
        assert!(snippets.input.is_empty());
        assert!(snippets.output.is_none());
        assert!(snippet_metadata.prompt.is_none());
        assert!(snippet_metadata.evidence[0].document_label.is_none());
        assert!(snippet_metadata.evidence[0].snippet.is_some());
        assert!(snippets.diagnosis.is_none());

        let mut unsafe_existing = snippets.clone();
        unsafe_existing.input = "legacy unsafe query".to_owned();
        unsafe_existing.output = Some("legacy unsafe answer".to_owned());
        let unsafe_metadata = unsafe_existing.ingestion.as_mut().expect("unsafe metadata");
        unsafe_metadata.prompt = Some("legacy unsafe prompt".to_owned());
        unsafe_metadata.evidence[0].document_label = Some("legacy-private.md".to_owned());
        let merged = merge_imported_trace(&unsafe_existing, snippets).expect("privacy scrub merge");
        let merged_metadata = merged.ingestion.as_ref().expect("merged metadata");
        assert!(merged.input.is_empty());
        assert!(merged.output.is_none());
        assert!(merged_metadata.prompt.is_none());
        assert!(merged_metadata.evidence[0].document_label.is_none());
        assert!(merged_metadata.evidence[0].snippet.is_some());

        let mut full = request(TraceIngestionPrivacyMode::FullLocalOnly);
        full.prompt = Some("local prompt".to_owned());
        let full = build_native_trace(
            &local,
            full,
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("full-local import");
        assert_eq!(
            full.ingestion
                .as_ref()
                .expect("full metadata")
                .prompt
                .as_deref(),
            Some("local prompt")
        );
    }

    #[test]
    fn incremental_span_requests_preserve_omitted_content_status_and_diagnosis() {
        let project = project(PrivacyMode::LocalOnly);
        let existing = build_native_trace(
            &project,
            request(TraceIngestionPrivacyMode::FullLocalOnly),
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("initial import");
        let mut incremental = request(TraceIngestionPrivacyMode::FullLocalOnly);
        incremental.query = None;
        incremental.answer = None;
        incremental.retrieved_evidence.clear();
        incremental.spans = vec![span("later-span", None)];
        let incremental = build_native_trace(
            &project,
            incremental,
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("incremental import");
        let mut merged = merge_imported_trace(&existing, incremental).expect("merge import");

        assert_eq!(merged.input, existing.input);
        assert_eq!(merged.output, existing.output);
        assert_eq!(merged.status, existing.status);
        assert!(merged.diagnosis.is_some());
        add_diagnosis(
            &mut merged,
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        );
        assert!(merged.diagnosis.is_some());
    }

    #[test]
    fn retries_raise_status_monotonically_and_harmless_retries_are_unchanged() {
        let project = project(PrivacyMode::LocalOnly);
        let existing = build_native_trace(
            &project,
            request(TraceIngestionPrivacyMode::FullLocalOnly),
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("initial import");
        assert_eq!(
            merge_imported_trace(&existing, existing.clone()).expect("harmless retry"),
            existing
        );

        let mut failed_eval_request = request(TraceIngestionPrivacyMode::FullLocalOnly);
        failed_eval_request.evaluation_passed = Some(false);
        let failed_eval = build_native_trace(
            &project,
            failed_eval_request,
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("failed evaluation retry");
        let failed = merge_imported_trace(&existing, failed_eval).expect("merge failed eval");
        assert_eq!(failed.status, TraceStatus::Failed);

        let completed = build_native_trace(
            &project,
            request(TraceIngestionPrivacyMode::FullLocalOnly),
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("completed retry");
        let retained_failure =
            merge_imported_trace(&failed, completed).expect("failed aggregate remains failed");
        assert_eq!(retained_failure.status, TraceStatus::Failed);
        assert_eq!(retained_failure.summary, failed.summary);

        let mut warning_request = request(TraceIngestionPrivacyMode::FullLocalOnly);
        let mut warning = span("warning-span", None);
        warning.status = ImportedSpanStatus::Warning;
        warning_request.spans = vec![warning];
        let warning = build_native_trace(
            &project,
            warning_request,
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("warning retry");
        assert_eq!(
            merge_imported_trace(&existing, warning)
                .expect("merge warning span")
                .status,
            TraceStatus::Warning
        );
    }

    #[test]
    fn merged_imports_keep_evidence_rank_and_span_time_order() {
        let project = project(PrivacyMode::LocalOnly);
        let now = OffsetDateTime::now_utc();
        let mut existing_request = request(TraceIngestionPrivacyMode::FullLocalOnly);
        existing_request.retrieved_evidence[0].external_chunk_id = "chunk-z".to_owned();
        existing_request.retrieved_evidence[0].rank = 2;
        let mut later = span("span-z", None);
        later.started_at = now + Duration::seconds(1);
        later.completed_at = Some(later.started_at);
        existing_request.spans = vec![later];
        let existing = build_native_trace(
            &project,
            existing_request,
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("initial import");

        let mut incoming_request = request(TraceIngestionPrivacyMode::FullLocalOnly);
        incoming_request.retrieved_evidence[0].external_chunk_id = "chunk-a".to_owned();
        incoming_request.retrieved_evidence[0].rank = 1;
        let mut earlier = span("span-a", None);
        earlier.started_at = now;
        earlier.completed_at = Some(now);
        incoming_request.spans = vec![earlier];
        let incoming = build_native_trace(
            &project,
            incoming_request,
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("incremental import");

        let merged = merge_imported_trace(&existing, incoming).expect("merge import");
        let metadata = merged.ingestion.expect("merged metadata");
        assert_eq!(
            metadata
                .evidence
                .iter()
                .map(|value| value.external_chunk_id.as_str())
                .collect::<Vec<_>>(),
            vec!["chunk-a", "chunk-z"]
        );
        assert_eq!(
            metadata
                .spans
                .iter()
                .map(|value| value.external_span_id.as_str())
                .collect::<Vec<_>>(),
            vec!["span-a", "span-z"]
        );
    }

    #[test]
    fn imported_report_permissions_follow_ingestion_privacy() {
        let project = project(PrivacyMode::LocalOnly);
        let context = |privacy_mode| DebugReportBuildContext {
            report_id: DebugReportId(Uuid::now_v7()),
            workspace_id: WorkspaceId(Uuid::now_v7()),
            project_id: project.id,
            privacy_mode,
            created_at: OffsetDateTime::now_utc(),
        };
        let mut snippets_request = request(TraceIngestionPrivacyMode::SnippetsAllowed);
        let mut weak_evidence = snippets_request.retrieved_evidence[0].clone();
        weak_evidence.external_chunk_id = "chunk-2".to_owned();
        weak_evidence.rank = 2;
        weak_evidence.score = 0.1;
        snippets_request.retrieved_evidence.push(weak_evidence);
        let snippets = build_native_trace(
            &project,
            snippets_request,
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("valid snippet import");
        let report =
            build_trace_debug_report(context(DebugReportPrivacyMode::SnippetsAllowed), &snippets)
                .expect("snippet report");
        assert_eq!(
            report.evidence[0].external_evidence_id.as_deref(),
            Some("chunk-1")
        );
        assert_eq!(report.subject, "Imported trace trace-1");
        assert_eq!(
            report.evidence[0].evidence_strength,
            Some(EvidenceStrength::Strong)
        );
        assert_eq!(
            report.evidence[1].evidence_strength,
            Some(EvidenceStrength::Weak)
        );
        assert!(report.evidence[0].document_path.is_none());
        assert!(report.evidence[0].snippet.is_some());
        let markdown = render_debug_report_markdown(&report).expect("imported report markdown");
        assert!(markdown.contains("chunk-1"));
        assert!(!markdown.contains("When is the index published?"));
        assert!(!markdown.contains("After validation."));
        assert!(!markdown.contains("policy.md"));

        let metadata = build_native_trace(
            &project,
            request(TraceIngestionPrivacyMode::MetadataOnly),
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("valid metadata import");
        assert!(build_trace_debug_report(
            context(DebugReportPrivacyMode::SnippetsAllowed),
            &metadata,
        )
        .is_err());

        let full_local = build_native_trace(
            &project,
            request(TraceIngestionPrivacyMode::FullLocalOnly),
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("valid full-local import");
        assert!(build_trace_debug_report(
            context(DebugReportPrivacyMode::MetadataOnly),
            &full_local,
        )
        .is_err());
    }

    fn project(privacy_mode: PrivacyMode) -> Project {
        let now = OffsetDateTime::now_utc();
        Project {
            id: ProjectId(Uuid::now_v7()),
            name: "test".to_owned(),
            privacy_mode,
            created_at: now,
            updated_at: now,
        }
    }

    fn request(privacy_mode: TraceIngestionPrivacyMode) -> NativeTraceIngestionRequest {
        NativeTraceIngestionRequest {
            schema_version: "1".to_owned(),
            project_id: ProjectId(Uuid::now_v7()),
            external_trace_id: "trace-1".to_owned(),
            privacy_mode,
            query: Some("When is the index published?".to_owned()),
            prompt: None,
            answer: Some("After validation.".to_owned()),
            retrieval_mode: Some(RetrievalMode::Hybrid),
            top_k: Some(1),
            model_config: None,
            retrieved_evidence: vec![ImportedEvidence {
                external_chunk_id: "chunk-1".to_owned(),
                document_label: Some("policy.md".to_owned()),
                rank: 1,
                score: 0.9,
                lexical_score: Some(0.8),
                semantic_score: Some(0.9),
                citation_label: Some("E1".to_owned()),
                snippet: Some("The index is published after validation.".to_owned()),
                answer_support_status: AnswerSupportStatus::Supported,
                answer_support_reason: AnswerSupportReason::DirectBodySupport,
            }],
            spans: Vec::new(),
            failure_labels: Vec::new(),
            evaluation_passed: None,
            evaluation_label: None,
            started_at: None,
            completed_at: None,
            latency_ms: Some(20),
            status: None,
        }
    }

    fn span(id: &str, parent: Option<&str>) -> ImportedSpan {
        let now = OffsetDateTime::now_utc();
        ImportedSpan {
            external_span_id: id.to_owned(),
            parent_span_id: parent.map(str::to_owned),
            operation: ImportedSpanOperation::Retrieval,
            kind: ImportedSpanKind::Internal,
            name: "Retrieval".to_owned(),
            started_at: now,
            completed_at: Some(now),
            latency_ms: 0,
            status: ImportedSpanStatus::Succeeded,
            provider: None,
            model: None,
            input_tokens: None,
            output_tokens: None,
            error_type: None,
        }
    }
}
