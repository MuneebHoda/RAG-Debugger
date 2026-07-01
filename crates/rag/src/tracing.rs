use std::collections::HashSet;

use rag_debugger_core::{
    DebuggerConfig, DiagnosisOutcome, EvidenceStrength, ExtractiveAnswerStatus, FailureLabel,
    ProjectId, RetrievalConfig, RetrievalQueryRequest, RetrievalQueryResponse, Trace, TraceId,
    TraceRerunComparison, TraceRerunId, TraceSpan, TraceSpanDetail, TraceSpanId, TraceSpanKind,
    TraceSpanStatus, TraceStatus,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::diagnosis::{compare_diagnoses, diagnose_retrieval, legacy_failure_labels};
use crate::retrieval::ensure_response_answerability;

pub fn build_trace_from_retrieval(
    project_id: ProjectId,
    response: RetrievalQueryResponse,
    retrieval_config: &RetrievalConfig,
    debugger_config: &DebuggerConfig,
) -> Trace {
    let response = ensure_response_answerability(response, retrieval_config, debugger_config);
    let now = OffsetDateTime::now_utc();
    let evidence_strength = strongest_evidence(&response);
    let diagnosis = response
        .diagnosis
        .clone()
        .unwrap_or_else(|| diagnose_retrieval(&response, debugger_config, None));
    let failure_labels = legacy_failure_labels(&diagnosis);
    let status = trace_status(diagnosis.outcome);
    let summary = diagnosis.summary.clone();
    let spans = build_spans(now, &response, evidence_strength);

    Trace {
        id: TraceId(Uuid::now_v7()),
        project_id,
        input: response.run.query.clone(),
        output: Some(response.answer.text.clone()),
        started_at: response.run.created_at,
        completed_at: Some(now),
        retrieval_runs: Vec::new(),
        generation: None,
        failure_labels,
        source_run_id: Some(response.run.id),
        summary,
        status,
        evidence_strength: Some(evidence_strength),
        spans,
        retrieval: Some(response),
        reruns: Vec::new(),
        diagnosis: Some(diagnosis),
    }
}

pub fn ensure_trace_diagnosis(
    mut trace: Trace,
    retrieval_config: &RetrievalConfig,
    debugger_config: &DebuggerConfig,
) -> Trace {
    let Some(retrieval) = trace.retrieval.take() else {
        return trace;
    };
    let retrieval = ensure_response_answerability(retrieval, retrieval_config, debugger_config);
    if let Some(diagnosis) = retrieval.diagnosis.clone() {
        trace.failure_labels = legacy_failure_labels(&diagnosis);
        trace.status = trace_status(diagnosis.outcome);
        trace.evidence_strength = Some(strongest_evidence(&retrieval));
        trace.summary = diagnosis.summary.clone();
        trace.diagnosis = Some(diagnosis);
    }
    trace.retrieval = Some(retrieval);
    trace
}

pub fn build_rerun_comparison(
    original: &RetrievalQueryResponse,
    request: RetrievalQueryRequest,
    response: RetrievalQueryResponse,
    retrieval_config: &RetrievalConfig,
    debugger_config: &DebuggerConfig,
) -> TraceRerunComparison {
    let original =
        ensure_response_answerability(original.clone(), retrieval_config, debugger_config);
    let response = ensure_response_answerability(response, retrieval_config, debugger_config);
    let original_top_score = original.hits.first().map_or(0.0, |hit| hit.score);
    let rerun_top_score = response.hits.first().map_or(0.0, |hit| hit.score);
    let latency_delta_ms = response.run.latency_ms as i64 - original.run.latency_ms as i64;
    let original_chunk_ids = original
        .hits
        .iter()
        .map(|hit| hit.chunk.id)
        .collect::<HashSet<_>>();
    let rerun_chunk_ids = response
        .hits
        .iter()
        .map(|hit| hit.chunk.id)
        .collect::<HashSet<_>>();
    let overlap_count = original_chunk_ids.intersection(&rerun_chunk_ids).count() as u32;
    let changed_rank_count = response
        .hits
        .iter()
        .filter(|hit| {
            original
                .hits
                .iter()
                .find(|original_hit| original_hit.chunk.id == hit.chunk.id)
                .is_some_and(|original_hit| original_hit.rank != hit.rank)
        })
        .count() as u32;

    let diagnosis = compare_diagnoses(&original, &response);
    TraceRerunComparison {
        id: TraceRerunId(Uuid::now_v7()),
        request,
        response,
        score_delta: rerun_top_score - original_top_score,
        latency_delta_ms,
        overlap_count,
        changed_rank_count,
        diagnosis,
        created_at: OffsetDateTime::now_utc(),
    }
}

pub fn diagnose_failure_labels(response: &RetrievalQueryResponse) -> Vec<FailureLabel> {
    let response = ensure_response_answerability(
        response.clone(),
        &RetrievalConfig::default(),
        &DebuggerConfig::default(),
    );
    let diagnosis = response
        .diagnosis
        .clone()
        .unwrap_or_else(|| diagnose_retrieval(&response, &DebuggerConfig::default(), None));
    legacy_failure_labels(&diagnosis)
}

fn build_spans(
    timestamp: OffsetDateTime,
    response: &RetrievalQueryResponse,
    evidence_strength: EvidenceStrength,
) -> Vec<TraceSpan> {
    let top_score = response.hits.first().map_or(0.0, |hit| hit.score);
    vec![
        TraceSpan {
            id: TraceSpanId(Uuid::now_v7()),
            kind: TraceSpanKind::QueryInput,
            title: "Query input".to_owned(),
            description: "Captured the user question, retrieval mode, top-k, and active filters."
                .to_owned(),
            started_at: response.run.created_at,
            completed_at: Some(response.run.created_at),
            latency_ms: 0,
            status: TraceSpanStatus::Succeeded,
            detail: TraceSpanDetail::QueryInput {
                top_k: response.run.top_k,
                retrieval_mode: response.run.retrieval_mode,
                source_filter_count: 0,
                document_filter_count: 0,
            },
        },
        TraceSpan {
            id: TraceSpanId(Uuid::now_v7()),
            kind: TraceSpanKind::Retrieval,
            title: "Retrieval ranking".to_owned(),
            description: "Scored chunks with the selected retrieval mode and recorded ranking signals."
                .to_owned(),
            started_at: response.run.created_at,
            completed_at: Some(timestamp),
            latency_ms: response.run.latency_ms,
            status: if response.hits.is_empty() {
                TraceSpanStatus::Warning
            } else {
                TraceSpanStatus::Succeeded
            },
            detail: TraceSpanDetail::Retrieval {
                hit_count: response.hits.len() as u32,
                top_score,
                embedding_readiness: response.embedding_status.readiness,
            },
        },
        TraceSpan {
            id: TraceSpanId(Uuid::now_v7()),
            kind: TraceSpanKind::EvidenceSummary,
            title: "Evidence summary".to_owned(),
            description: "Built a cited extractive answer from the strongest non-duplicate evidence."
                .to_owned(),
            started_at: timestamp,
            completed_at: Some(timestamp),
            latency_ms: 0,
            status: if response.answer.status == ExtractiveAnswerStatus::Answered {
                TraceSpanStatus::Succeeded
            } else {
                TraceSpanStatus::Warning
            },
            detail: TraceSpanDetail::EvidenceSummary {
                answer_status: answer_status_label(response.answer.status).to_owned(),
                citation_count: response.answer.citations.len() as u32,
                strongest_evidence: evidence_strength,
            },
        },
        TraceSpan {
            id: TraceSpanId(Uuid::now_v7()),
            kind: TraceSpanKind::EvalCheck,
            title: "Eval check".to_owned(),
            description: "No eval case is linked to this trace yet; save the query as an eval to watch regressions."
                .to_owned(),
            started_at: timestamp,
            completed_at: Some(timestamp),
            latency_ms: 0,
            status: TraceSpanStatus::Warning,
            detail: TraceSpanDetail::EvalCheck {
                checked: false,
                passed: None,
                message: "Trace saved before a matching eval was attached.".to_owned(),
            },
        },
    ]
}

fn trace_status(outcome: DiagnosisOutcome) -> TraceStatus {
    match outcome {
        DiagnosisOutcome::Strong => TraceStatus::Completed,
        DiagnosisOutcome::Mixed | DiagnosisOutcome::Weak => TraceStatus::Warning,
        DiagnosisOutcome::Failing => TraceStatus::Failed,
    }
}

fn strongest_evidence(response: &RetrievalQueryResponse) -> EvidenceStrength {
    response
        .hits
        .iter()
        .map(|hit| hit.evidence_strength)
        .min_by_key(|strength| match strength {
            EvidenceStrength::Strong => 0,
            EvidenceStrength::Medium => 1,
            EvidenceStrength::Weak => 2,
        })
        .unwrap_or(EvidenceStrength::Weak)
}

fn answer_status_label(status: ExtractiveAnswerStatus) -> &'static str {
    match status {
        ExtractiveAnswerStatus::Answered => "answered",
        ExtractiveAnswerStatus::InsufficientEvidence => "insufficient_evidence",
    }
}

#[cfg(test)]
mod tests {
    use rag_debugger_core::{
        AnswerSupportStatus, ByteRange, ChunkId, ChunkPreview, ChunkSplitReason, ChunkingStrategy,
        Document, DocumentId, DocumentProfile, EmbeddingModelInfo, ExtractionQuality,
        ExtractiveAnswer, RetrievalCitation, RetrievalEmbeddingReadiness, RetrievalEmbeddingStatus,
        RetrievalMatchedTerm, RetrievalMode, RetrievalQualityFlag, RetrievalQueryHit,
        RetrievalQueryRun, RetrievalQueryRunId, RetrievalScoreBreakdown, Source, SourceId,
        SourceKind, SourceSyncPolicy,
    };

    use super::*;

    #[test]
    fn missing_embeddings_create_embedding_failure_labels() {
        let response = response_with_status(RetrievalEmbeddingReadiness::Missing, Vec::new());

        let labels = diagnose_failure_labels(&response);

        assert!(labels.contains(&FailureLabel::MissingEmbeddingIndex));
        assert!(labels.contains(&FailureLabel::BadEmbedding));
        assert!(!labels.contains(&FailureLabel::MissingDocument));
    }

    #[test]
    fn trace_from_empty_retrieval_is_failed_and_explainable() {
        let response = response_with_status(RetrievalEmbeddingReadiness::Ready, Vec::new());
        let trace = build_trace_from_retrieval(
            ProjectId(Uuid::now_v7()),
            response,
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        );

        assert_eq!(trace.status, TraceStatus::Failed);
        assert_eq!(trace.spans.len(), 4);
        assert!(trace
            .failure_labels
            .contains(&FailureLabel::MissingDocument));
    }

    #[test]
    fn legacy_trace_is_enriched_with_answerability_on_read() {
        let response =
            response_with_status(RetrievalEmbeddingReadiness::Ready, vec![quality_hit()]);
        let mut trace = build_trace_from_retrieval(
            ProjectId(Uuid::now_v7()),
            response,
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        );
        trace.diagnosis = None;
        trace.retrieval.as_mut().expect("retrieval").diagnosis = None;

        let enriched = ensure_trace_diagnosis(
            trace,
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        );

        assert!(enriched.diagnosis.is_some());
        assert!(enriched
            .retrieval
            .as_ref()
            .and_then(|retrieval| retrieval.diagnosis.as_ref())
            .is_some());
        assert_ne!(
            enriched.retrieval.as_ref().expect("retrieval").hits[0]
                .answer_support
                .status,
            AnswerSupportStatus::Unassessed
        );
    }

    #[test]
    fn quality_failure_labels_are_deterministic_and_deduplicated() {
        let mut hit = quality_hit();
        hit.evidence_strength = EvidenceStrength::Weak;
        hit.quality_flags = vec![
            RetrievalQualityFlag::Duplicate,
            RetrievalQualityFlag::HeadingOnly,
        ];
        let response = response_with_status(RetrievalEmbeddingReadiness::Ready, vec![hit]);

        let first = diagnose_failure_labels(&response);
        let second = diagnose_failure_labels(&response);

        assert_eq!(first, second);
        assert_eq!(
            first,
            vec![
                FailureLabel::UnsupportedQuestion,
                FailureLabel::WeakEvidence,
                FailureLabel::BadRanking,
                FailureLabel::DuplicateEvidence,
                FailureLabel::BadChunking,
                FailureLabel::HeadingOnlyEvidence,
            ]
        );
    }

    #[test]
    fn rerun_comparison_counts_overlap_and_score_delta() {
        let original = response_with_status(RetrievalEmbeddingReadiness::Ready, Vec::new());
        let rerun = response_with_status(RetrievalEmbeddingReadiness::Ready, Vec::new());
        let request = RetrievalQueryRequest {
            query: "gpu indexing".to_owned(),
            top_k: 5,
            retrieval_mode: RetrievalMode::Lexical,
            source_ids: Vec::new(),
            document_ids: Vec::new(),
        };

        let comparison = build_rerun_comparison(
            &original,
            request,
            rerun,
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        );

        assert_eq!(comparison.overlap_count, 0);
        assert_eq!(comparison.score_delta, 0.0);
    }

    fn response_with_status(
        readiness: RetrievalEmbeddingReadiness,
        hits: Vec<rag_debugger_core::RetrievalQueryHit>,
    ) -> RetrievalQueryResponse {
        RetrievalQueryResponse {
            run: RetrievalQueryRun {
                id: RetrievalQueryRunId(Uuid::now_v7()),
                query: "gpu indexing".to_owned(),
                top_k: 5,
                retrieval_mode: RetrievalMode::Hybrid,
                latency_ms: 12,
                created_at: OffsetDateTime::now_utc(),
            },
            answer: ExtractiveAnswer {
                status: ExtractiveAnswerStatus::InsufficientEvidence,
                text: "Not enough local evidence.".to_owned(),
                citations: Vec::new(),
            },
            hits,
            embedding_status: RetrievalEmbeddingStatus {
                readiness,
                required: true,
                model: EmbeddingModelInfo {
                    provider: "local".to_owned(),
                    model_name: "local-hash-v1".to_owned(),
                    dimension: 384,
                },
                total_chunks: 10,
                indexed_chunks: 0,
                missing_chunks: 10,
                stale_chunks: 0,
            },
            diagnosis: None,
        }
    }

    fn quality_hit() -> RetrievalQueryHit {
        let source_id = SourceId(Uuid::now_v7());
        let document_id = DocumentId(Uuid::now_v7());
        let chunk_id = ChunkId(Uuid::now_v7());
        let source = Source {
            id: source_id,
            project_id: ProjectId(Uuid::now_v7()),
            name: "Public fixture corpus".to_owned(),
            kind: SourceKind::FileSet {
                root_hint: "fixtures/corpora".to_owned(),
            },
            sync_policy: SourceSyncPolicy::Manual,
            chunking: Default::default(),
        };
        let document = Document {
            id: document_id,
            source_id,
            path: "technical_docs/gpu-indexing.md".to_owned(),
            mime_type: Some("text/markdown".to_owned()),
            checksum: "document-checksum".to_owned(),
            byte_size: 64,
            profile: DocumentProfile::TechnicalDocs,
            extraction_quality: ExtractionQuality::High,
            warnings: Vec::new(),
        };
        let chunk = ChunkPreview {
            id: chunk_id,
            document_id,
            ordinal: 0,
            text: "GPU indexing".to_owned(),
            token_count: 2,
            byte_range: ByteRange { start: 0, end: 12 },
            checksum: "chunk-checksum".to_owned(),
            strategy: ChunkingStrategy::SmartSections,
            section_title: Some("GPU indexing".to_owned()),
            split_reason: ChunkSplitReason::DocumentEnd,
            quality_flags: Vec::new(),
            is_duplicate: false,
            text_density: 1.0,
            evidence_score_hint: 0.8,
        };

        RetrievalQueryHit {
            rank: 1,
            score: 1.0,
            chunk,
            document,
            source,
            matched_terms: vec![RetrievalMatchedTerm {
                term: "indexing".to_owned(),
                count: 1,
            }],
            score_breakdown: RetrievalScoreBreakdown::zero(),
            normalized_score_breakdown: RetrievalScoreBreakdown::zero(),
            snippet: "GPU indexing".to_owned(),
            citation: RetrievalCitation {
                label: "[1]".to_owned(),
                chunk_id,
                document_id,
                document_path: "technical_docs/gpu-indexing.md".to_owned(),
                chunk_ordinal: 0,
                section_title: Some("GPU indexing".to_owned()),
                checksum_prefix: "chunk-checksu".to_owned(),
                snippet: "GPU indexing".to_owned(),
            },
            quality_flags: Vec::new(),
            evidence_strength: EvidenceStrength::Strong,
            duplicate_count: 1,
            answer_support: Default::default(),
        }
    }
}
