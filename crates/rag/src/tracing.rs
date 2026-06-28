use std::collections::HashSet;

use rag_debugger_core::{
    EvidenceStrength, ExtractiveAnswerStatus, FailureLabel, ProjectId, RetrievalEmbeddingReadiness,
    RetrievalQualityFlag, RetrievalQueryRequest, RetrievalQueryResponse, Trace, TraceId,
    TraceRerunComparison, TraceRerunId, TraceSpan, TraceSpanDetail, TraceSpanId, TraceSpanKind,
    TraceSpanStatus, TraceStatus,
};
use time::OffsetDateTime;
use uuid::Uuid;

pub fn build_trace_from_retrieval(
    project_id: ProjectId,
    response: RetrievalQueryResponse,
) -> Trace {
    let now = OffsetDateTime::now_utc();
    let evidence_strength = strongest_evidence(&response);
    let failure_labels = diagnose_failure_labels(&response);
    let status = trace_status(&response, &failure_labels);
    let summary = trace_summary(&response, evidence_strength, &failure_labels);
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
    }
}

pub fn build_rerun_comparison(
    original: &RetrievalQueryResponse,
    request: RetrievalQueryRequest,
    response: RetrievalQueryResponse,
) -> TraceRerunComparison {
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

    TraceRerunComparison {
        id: TraceRerunId(Uuid::now_v7()),
        request,
        response,
        score_delta: rerun_top_score - original_top_score,
        latency_delta_ms,
        overlap_count,
        changed_rank_count,
        created_at: OffsetDateTime::now_utc(),
    }
}

pub fn diagnose_failure_labels(response: &RetrievalQueryResponse) -> Vec<FailureLabel> {
    let mut labels = Vec::new();

    if response.hits.is_empty() {
        labels.push(FailureLabel::MissingDocument);
    }

    if response.embedding_status.required {
        match response.embedding_status.readiness {
            RetrievalEmbeddingReadiness::Missing => {
                labels.push(FailureLabel::MissingEmbeddingIndex);
                labels.push(FailureLabel::BadEmbedding);
            }
            RetrievalEmbeddingReadiness::Partial => labels.push(FailureLabel::BadEmbedding),
            RetrievalEmbeddingReadiness::NotRequired | RetrievalEmbeddingReadiness::Ready => {}
        }
    }

    if response.answer.status == ExtractiveAnswerStatus::InsufficientEvidence {
        labels.push(FailureLabel::WeakEvidence);
    }

    if response
        .hits
        .iter()
        .any(|hit| hit.evidence_strength == EvidenceStrength::Weak)
    {
        labels.push(FailureLabel::BadRanking);
    }

    if response
        .hits
        .iter()
        .any(|hit| hit.quality_flags.contains(&RetrievalQualityFlag::Duplicate))
    {
        labels.push(FailureLabel::DuplicateEvidence);
        labels.push(FailureLabel::BadChunking);
    }

    if response.hits.iter().any(|hit| {
        hit.quality_flags
            .contains(&RetrievalQualityFlag::HeadingOnly)
    }) {
        labels.push(FailureLabel::HeadingOnlyEvidence);
        labels.push(FailureLabel::BadChunking);
    }

    dedupe_failure_labels(labels)
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

fn trace_status(response: &RetrievalQueryResponse, labels: &[FailureLabel]) -> TraceStatus {
    if response.hits.is_empty()
        || labels.contains(&FailureLabel::MissingEmbeddingIndex)
        || labels.contains(&FailureLabel::MissingDocument)
    {
        TraceStatus::Failed
    } else if labels.is_empty() {
        TraceStatus::Completed
    } else {
        TraceStatus::Warning
    }
}

fn trace_summary(
    response: &RetrievalQueryResponse,
    evidence_strength: EvidenceStrength,
    labels: &[FailureLabel],
) -> String {
    if response.hits.is_empty() {
        return "No evidence was retrieved for this query, so CorpusLab saved a failed trace for diagnosis."
            .to_owned();
    }

    if labels.is_empty() {
        return format!(
            "Retrieved {} evidence chunks with {:?} strongest evidence.",
            response.hits.len(),
            evidence_strength
        );
    }

    format!(
        "Retrieved {} chunks, but CorpusLab found {} quality signal(s) that need review.",
        response.hits.len(),
        labels.len()
    )
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

fn dedupe_failure_labels(labels: Vec<FailureLabel>) -> Vec<FailureLabel> {
    let mut deduped = Vec::new();
    for label in labels {
        if !deduped.contains(&label) {
            deduped.push(label);
        }
    }
    deduped
}

#[cfg(test)]
mod tests {
    use rag_debugger_core::{
        ByteRange, ChunkId, ChunkPreview, ChunkSplitReason, ChunkingStrategy, Document, DocumentId,
        DocumentProfile, EmbeddingModelInfo, ExtractionQuality, ExtractiveAnswer,
        RetrievalCitation, RetrievalEmbeddingStatus, RetrievalMatchedTerm, RetrievalMode,
        RetrievalQueryHit, RetrievalQueryRun, RetrievalQueryRunId, RetrievalScoreBreakdown, Source,
        SourceId, SourceKind, SourceSyncPolicy,
    };

    use super::*;

    #[test]
    fn missing_embeddings_create_embedding_failure_labels() {
        let response = response_with_status(RetrievalEmbeddingReadiness::Missing, Vec::new());

        let labels = diagnose_failure_labels(&response);

        assert!(labels.contains(&FailureLabel::MissingEmbeddingIndex));
        assert!(labels.contains(&FailureLabel::BadEmbedding));
        assert!(labels.contains(&FailureLabel::MissingDocument));
    }

    #[test]
    fn trace_from_empty_retrieval_is_failed_and_explainable() {
        let response = response_with_status(RetrievalEmbeddingReadiness::Ready, Vec::new());
        let trace = build_trace_from_retrieval(ProjectId(Uuid::now_v7()), response);

        assert_eq!(trace.status, TraceStatus::Failed);
        assert_eq!(trace.spans.len(), 4);
        assert!(trace
            .failure_labels
            .contains(&FailureLabel::MissingDocument));
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

        let comparison = build_rerun_comparison(&original, request, rerun);

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
        }
    }
}
