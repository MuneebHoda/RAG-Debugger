use std::collections::HashSet;

use rag_debugger_core::{
    EvidenceStrength, RetrievalEmbeddingReadiness, RetrievalEvalCase, RetrievalEvalCaseEvaluation,
    RetrievalEvalComparison, RetrievalEvalFailure, RetrievalEvalFailureLabel,
    RetrievalEvalFailureSeverity, RetrievalEvalGate, RetrievalEvalGateStatus,
    RetrievalEvalModeResult, RetrievalEvalResult, RetrievalMode, RetrievalQualityFlag,
    RetrievalQueryResponse,
};

const DEFAULT_RECALL_THRESHOLD: f32 = 0.80;
const DEFAULT_WEAK_EVIDENCE_LIMIT: f32 = 0.20;

pub fn score_retrieval_eval_case(
    case: &RetrievalEvalCase,
    response: &RetrievalQueryResponse,
) -> RetrievalEvalResult {
    let evaluation = evaluate_retrieval_eval_case(case, response);

    RetrievalEvalResult {
        case_id: evaluation.case_id,
        query: evaluation.query,
        top_k: evaluation.top_k,
        recall_at_k: evaluation.recall_at_k,
        precision_at_k: evaluation.precision_at_k,
        top_hit_rank: evaluation.top_hit_rank,
        passed: evaluation.passed,
        expected_chunk_ids: evaluation.expected_chunk_ids,
        expected_document_ids: evaluation.expected_document_ids,
        retrieved_chunk_ids: evaluation.retrieved_chunk_ids,
        latency_ms: evaluation.latency_ms,
    }
}

pub fn evaluate_retrieval_eval_case(
    case: &RetrievalEvalCase,
    response: &RetrievalQueryResponse,
) -> RetrievalEvalCaseEvaluation {
    let retrieved_chunk_ids = response
        .hits
        .iter()
        .map(|hit| hit.chunk.id)
        .collect::<Vec<_>>();

    let expected_chunk_ids = case
        .expected_chunk_ids
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let expected_document_ids = case
        .expected_document_ids
        .iter()
        .copied()
        .collect::<HashSet<_>>();

    let expected_count = expected_chunk_ids.len() + expected_document_ids.len();
    let matched_expected_chunks = response
        .hits
        .iter()
        .filter(|hit| expected_chunk_ids.contains(&hit.chunk.id))
        .map(|hit| hit.chunk.id)
        .collect::<HashSet<_>>()
        .len();
    let matched_expected_documents = response
        .hits
        .iter()
        .filter(|hit| expected_document_ids.contains(&hit.document.id))
        .map(|hit| hit.document.id)
        .collect::<HashSet<_>>()
        .len();
    let matched_expected_count = matched_expected_chunks + matched_expected_documents;

    let matching_hit_count = response
        .hits
        .iter()
        .filter(|hit| {
            expected_chunk_ids.contains(&hit.chunk.id)
                || expected_document_ids.contains(&hit.document.id)
        })
        .count();

    let recall_at_k = if expected_count == 0 {
        0.0
    } else {
        matched_expected_count as f32 / expected_count as f32
    };
    let precision_at_k = if response.hits.is_empty() {
        0.0
    } else {
        matching_hit_count as f32 / response.hits.len() as f32
    };
    let top_hit_rank = response
        .hits
        .iter()
        .find(|hit| {
            expected_chunk_ids.contains(&hit.chunk.id)
                || expected_document_ids.contains(&hit.document.id)
        })
        .map(|hit| hit.rank);
    let mrr = top_hit_rank.map_or(0.0, |rank| 1.0 / rank as f32);
    let citation_coverage = if response.hits.is_empty() {
        0.0
    } else {
        response.answer.citations.len().min(response.hits.len()) as f32 / response.hits.len() as f32
    };
    let weak_evidence_count = response
        .hits
        .iter()
        .filter(|hit| hit.evidence_strength == EvidenceStrength::Weak)
        .count() as u32;
    let missing_embedding_failures =
        u32::from(response.embedding_status.readiness == RetrievalEmbeddingReadiness::Missing);
    let mut failures = Vec::new();

    if missing_embedding_failures > 0 {
        failures.push(failure(
            case,
            response.run.retrieval_mode,
            RetrievalEvalFailureLabel::MissingEmbeddings,
            RetrievalEvalFailureSeverity::Critical,
            "Embeddings are missing for this retrieval mode.",
            top_hit_rank,
        ));
    }
    if recall_at_k == 0.0 && expected_count > 0 {
        failures.push(failure(
            case,
            response.run.retrieval_mode,
            RetrievalEvalFailureLabel::ExpectedEvidenceMissing,
            RetrievalEvalFailureSeverity::Critical,
            "No expected chunk or document was retrieved.",
            top_hit_rank,
        ));
    } else if matched_expected_documents > 0
        && !expected_chunk_ids.is_empty()
        && matched_expected_chunks == 0
    {
        failures.push(failure(
            case,
            response.run.retrieval_mode,
            RetrievalEvalFailureLabel::CorrectDocumentWrongChunk,
            RetrievalEvalFailureSeverity::Warning,
            "The expected document matched, but the expected chunk did not.",
            top_hit_rank,
        ));
    }
    if !response.hits.is_empty() && precision_at_k < 0.5 {
        failures.push(failure(
            case,
            response.run.retrieval_mode,
            RetrievalEvalFailureLabel::LowPrecision,
            RetrievalEvalFailureSeverity::Warning,
            "Less than half of retrieved evidence matched expectations.",
            top_hit_rank,
        ));
    }
    if weak_evidence_count > 0 {
        failures.push(failure(
            case,
            response.run.retrieval_mode,
            RetrievalEvalFailureLabel::WeakEvidence,
            RetrievalEvalFailureSeverity::Warning,
            "One or more retrieved hits were marked as weak evidence.",
            top_hit_rank,
        ));
    }
    if response.hits.iter().any(|hit| {
        hit.quality_flags
            .contains(&RetrievalQualityFlag::HeadingOnly)
    }) {
        failures.push(failure(
            case,
            response.run.retrieval_mode,
            RetrievalEvalFailureLabel::HeadingOnlyEvidence,
            RetrievalEvalFailureSeverity::Warning,
            "A heading-only chunk appeared in the ranked evidence.",
            top_hit_rank,
        ));
    }
    if response.hits.iter().any(|hit| {
        hit.duplicate_count > 1 || hit.quality_flags.contains(&RetrievalQualityFlag::Duplicate)
    }) {
        failures.push(failure(
            case,
            response.run.retrieval_mode,
            RetrievalEvalFailureLabel::DuplicateEvidence,
            RetrievalEvalFailureSeverity::Warning,
            "Duplicate evidence was present in the ranked results.",
            top_hit_rank,
        ));
    }

    RetrievalEvalCaseEvaluation {
        case_id: case.id,
        query: case.query.clone(),
        top_k: case.top_k,
        recall_at_k,
        precision_at_k,
        mrr,
        top_hit_rank,
        citation_coverage,
        weak_evidence_count,
        missing_embedding_failures,
        passed: recall_at_k > 0.0,
        expected_chunk_ids: case.expected_chunk_ids.clone(),
        expected_document_ids: case.expected_document_ids.clone(),
        retrieved_chunk_ids,
        latency_ms: response.run.latency_ms,
        failures,
    }
}

pub fn summarize_mode_result(
    retrieval_mode: RetrievalMode,
    case_results: Vec<RetrievalEvalCaseEvaluation>,
) -> RetrievalEvalModeResult {
    let case_count = case_results.len() as u32;
    let passed_count = case_results.iter().filter(|result| result.passed).count() as u32;
    let mut latencies = case_results
        .iter()
        .map(|result| result.latency_ms)
        .collect::<Vec<_>>();
    latencies.sort_unstable();

    RetrievalEvalModeResult {
        retrieval_mode,
        case_count,
        passed_count,
        average_recall_at_k: average(case_results.iter().map(|result| result.recall_at_k)),
        average_precision_at_k: average(case_results.iter().map(|result| result.precision_at_k)),
        mean_reciprocal_rank: average(case_results.iter().map(|result| result.mrr)),
        citation_coverage: average(case_results.iter().map(|result| result.citation_coverage)),
        weak_evidence_count: case_results
            .iter()
            .map(|result| result.weak_evidence_count)
            .sum(),
        missing_embedding_failures: case_results
            .iter()
            .map(|result| result.missing_embedding_failures)
            .sum(),
        latency_p50_ms: percentile(&latencies, 0.50),
        latency_p95_ms: percentile(&latencies, 0.95),
        case_results,
    }
}

pub fn compare_mode_results(mode_results: &[RetrievalEvalModeResult]) -> RetrievalEvalComparison {
    let best = mode_results.iter().max_by(|left, right| {
        left.average_recall_at_k
            .partial_cmp(&right.average_recall_at_k)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                left.average_precision_at_k
                    .partial_cmp(&right.average_precision_at_k)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| right.latency_p50_ms.cmp(&left.latency_p50_ms))
    });
    let worst_recall = mode_results
        .iter()
        .map(|result| result.average_recall_at_k)
        .fold(f32::INFINITY, f32::min);
    let worst_precision = mode_results
        .iter()
        .map(|result| result.average_precision_at_k)
        .fold(f32::INFINITY, f32::min);
    let min_latency = mode_results
        .iter()
        .map(|result| result.latency_p50_ms)
        .min()
        .unwrap_or(0);
    let max_latency = mode_results
        .iter()
        .map(|result| result.latency_p50_ms)
        .max()
        .unwrap_or(0);

    RetrievalEvalComparison {
        best_mode: best.map(|result| result.retrieval_mode),
        mode_count: mode_results.len() as u32,
        recall_delta: best.map_or(0.0, |result| {
            result.average_recall_at_k - finite_or_zero(worst_recall)
        }),
        precision_delta: best.map_or(0.0, |result| {
            result.average_precision_at_k - finite_or_zero(worst_precision)
        }),
        latency_delta_ms: max_latency as i64 - min_latency as i64,
        summary: best
            .map(|result| {
                format!(
                    "{:?} led with {:.0}% recall and {:.0}% precision.",
                    result.retrieval_mode,
                    result.average_recall_at_k * 100.0,
                    result.average_precision_at_k * 100.0
                )
            })
            .unwrap_or_else(|| "No modes were evaluated.".to_owned()),
    }
}

pub fn evaluate_gate(mode_results: &[RetrievalEvalModeResult]) -> RetrievalEvalGate {
    let best = mode_results.iter().max_by(|left, right| {
        left.average_recall_at_k
            .partial_cmp(&right.average_recall_at_k)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let total_cases = mode_results
        .iter()
        .map(|result| result.case_count)
        .sum::<u32>()
        .max(1);
    let weak_evidence_count = mode_results
        .iter()
        .map(|result| result.weak_evidence_count)
        .sum::<u32>();
    let critical_failure_count = mode_results
        .iter()
        .flat_map(|result| result.case_results.iter())
        .flat_map(|result| result.failures.iter())
        .filter(|failure| failure.severity == RetrievalEvalFailureSeverity::Critical)
        .count() as u32;
    let average_recall_at_k = best.map_or(0.0, |result| result.average_recall_at_k);
    let weak_evidence_rate = weak_evidence_count as f32 / total_cases as f32;
    let mut reasons = Vec::new();

    if average_recall_at_k < DEFAULT_RECALL_THRESHOLD {
        reasons.push(format!(
            "Best recall {:.0}% is below the {:.0}% gate.",
            average_recall_at_k * 100.0,
            DEFAULT_RECALL_THRESHOLD * 100.0
        ));
    }
    if critical_failure_count > 0 {
        reasons.push(format!(
            "{critical_failure_count} critical eval failures require review."
        ));
    }
    if weak_evidence_rate > DEFAULT_WEAK_EVIDENCE_LIMIT {
        reasons.push(format!(
            "Weak evidence rate {:.0}% is above the {:.0}% limit.",
            weak_evidence_rate * 100.0,
            DEFAULT_WEAK_EVIDENCE_LIMIT * 100.0
        ));
    }
    if reasons.is_empty() {
        reasons.push("Eval gate passed for the best retrieval mode.".to_owned());
    }

    RetrievalEvalGate {
        status: if average_recall_at_k >= DEFAULT_RECALL_THRESHOLD
            && critical_failure_count == 0
            && weak_evidence_rate <= DEFAULT_WEAK_EVIDENCE_LIMIT
        {
            RetrievalEvalGateStatus::Passed
        } else {
            RetrievalEvalGateStatus::Failed
        },
        average_recall_at_k,
        weak_evidence_rate,
        critical_failure_count,
        recall_threshold: DEFAULT_RECALL_THRESHOLD,
        weak_evidence_limit: DEFAULT_WEAK_EVIDENCE_LIMIT,
        reasons,
    }
}

fn failure(
    case: &RetrievalEvalCase,
    retrieval_mode: RetrievalMode,
    label: RetrievalEvalFailureLabel,
    severity: RetrievalEvalFailureSeverity,
    message: &str,
    top_hit_rank: Option<u32>,
) -> RetrievalEvalFailure {
    RetrievalEvalFailure {
        case_id: case.id,
        query: case.query.clone(),
        retrieval_mode,
        label,
        severity,
        message: message.to_owned(),
        top_hit_rank,
    }
}

fn average(values: impl Iterator<Item = f32>) -> f32 {
    let mut total = 0.0;
    let mut count = 0u32;
    for value in values {
        total += value;
        count += 1;
    }
    if count == 0 {
        0.0
    } else {
        total / count as f32
    }
}

fn percentile(sorted: &[u64], percentile: f32) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let index = ((sorted.len() - 1) as f32 * percentile).ceil() as usize;
    sorted[index.min(sorted.len() - 1)]
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use rag_debugger_core::{
        ByteRange, ChunkId, ChunkPreview, ChunkSplitReason, ChunkingStrategy, Document, DocumentId,
        DocumentProfile, EmbeddingModelInfo, EvidenceStrength, ExtractionQuality, ExtractiveAnswer,
        ExtractiveAnswerStatus, ProjectId, RetrievalCitation, RetrievalEmbeddingReadiness,
        RetrievalEmbeddingStatus, RetrievalEvalCase, RetrievalEvalCaseId, RetrievalMatchedTerm,
        RetrievalMode, RetrievalQueryHit, RetrievalQueryResponse, RetrievalQueryRun,
        RetrievalQueryRunId, RetrievalScoreBreakdown, Source, SourceId, SourceKind,
        SourceSyncPolicy,
    };
    use time::OffsetDateTime;
    use uuid::Uuid;

    use super::*;

    #[test]
    fn scores_recall_precision_and_top_rank() {
        let document_id = DocumentId(Uuid::now_v7());
        let chunk_id = ChunkId(Uuid::now_v7());
        let case = RetrievalEvalCase {
            id: RetrievalEvalCaseId(Uuid::now_v7()),
            name: "GPU evidence".to_owned(),
            query: "gpu work".to_owned(),
            top_k: 5,
            expected_chunk_ids: vec![chunk_id],
            expected_document_ids: Vec::new(),
            notes: None,
            created_at: OffsetDateTime::now_utc(),
        };
        let response = RetrievalQueryResponse {
            run: RetrievalQueryRun {
                id: RetrievalQueryRunId(Uuid::now_v7()),
                query: case.query.clone(),
                top_k: 5,
                retrieval_mode: RetrievalMode::Hybrid,
                latency_ms: 4,
                created_at: OffsetDateTime::now_utc(),
            },
            answer: ExtractiveAnswer {
                status: ExtractiveAnswerStatus::Answered,
                text: "Built GPU tools [1]".to_owned(),
                citations: Vec::new(),
            },
            hits: vec![hit(chunk_id, document_id)],
            embedding_status: RetrievalEmbeddingStatus {
                readiness: RetrievalEmbeddingReadiness::Ready,
                required: true,
                model: EmbeddingModelInfo::default(),
                total_chunks: 1,
                indexed_chunks: 1,
                missing_chunks: 0,
                stale_chunks: 0,
            },
        };

        let result = score_retrieval_eval_case(&case, &response);

        assert!(result.passed);
        assert_eq!(result.recall_at_k, 1.0);
        assert_eq!(result.precision_at_k, 1.0);
        assert_eq!(result.top_hit_rank, Some(1));
    }

    fn hit(chunk_id: ChunkId, document_id: DocumentId) -> RetrievalQueryHit {
        let source_id = SourceId(Uuid::now_v7());
        let source = Source {
            id: source_id,
            project_id: ProjectId(Uuid::now_v7()),
            name: "Corpus upload".to_owned(),
            kind: SourceKind::FileSet {
                root_hint: "browser-upload".to_owned(),
            },
            sync_policy: SourceSyncPolicy::Manual,
            chunking: Default::default(),
        };
        let document = Document {
            id: document_id,
            source_id,
            path: "resume.md".to_owned(),
            mime_type: Some("text/markdown".to_owned()),
            checksum: "abc".to_owned(),
            byte_size: 32,
            profile: DocumentProfile::TechnicalDocs,
            extraction_quality: ExtractionQuality::High,
            warnings: Vec::new(),
        };
        let chunk = ChunkPreview {
            id: chunk_id,
            document_id,
            ordinal: 0,
            text: "Built GPU tools".to_owned(),
            token_count: 3,
            byte_range: ByteRange { start: 0, end: 15 },
            checksum: "1234567890ab".to_owned(),
            strategy: ChunkingStrategy::SmartSections,
            section_title: Some("Projects".to_owned()),
            split_reason: ChunkSplitReason::DocumentEnd,
            quality_flags: Vec::new(),
            is_duplicate: false,
            text_density: 1.0,
            evidence_score_hint: 0.8,
        };
        let citation = RetrievalCitation {
            label: "[1]".to_owned(),
            chunk_id,
            document_id,
            document_path: document.path.clone(),
            chunk_ordinal: 0,
            section_title: Some("Projects".to_owned()),
            checksum_prefix: "1234567890ab".to_owned(),
            snippet: "Built GPU tools".to_owned(),
        };

        RetrievalQueryHit {
            rank: 1,
            score: 3.0,
            chunk,
            document,
            source,
            matched_terms: vec![RetrievalMatchedTerm {
                term: "gpu".to_owned(),
                count: 1,
            }],
            score_breakdown: RetrievalScoreBreakdown {
                semantic: 1.0,
                lexical: 2.0,
                phrase: 0.0,
                section: 0.0,
                path: 0.0,
                metadata: 0.0,
            },
            normalized_score_breakdown: RetrievalScoreBreakdown {
                semantic: 0.5,
                lexical: 1.0,
                phrase: 0.0,
                section: 0.0,
                path: 0.0,
                metadata: 0.0,
            },
            snippet: "Built GPU tools".to_owned(),
            citation,
            quality_flags: Vec::new(),
            evidence_strength: EvidenceStrength::Strong,
            duplicate_count: 1,
        }
    }
}
