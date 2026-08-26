use std::collections::HashSet;

use rag_debugger_core::{
    experiment_contains_full_local_data, DebuggerConfig, DocumentId, EvidenceStrength,
    RetrievalEmbeddingReadiness, RetrievalEvalCase, RetrievalEvalCaseEvaluation,
    RetrievalEvalCaseId, RetrievalEvalCaseRegression, RetrievalEvalComparison,
    RetrievalEvalCompatibilityClassification, RetrievalEvalDatasetId, RetrievalEvalExperiment,
    RetrievalEvalExperimentSummary, RetrievalEvalFailure, RetrievalEvalFailureLabel,
    RetrievalEvalFailureSeverity, RetrievalEvalGate, RetrievalEvalGateStatus,
    RetrievalEvalMetricDelta, RetrievalEvalModeResult, RetrievalEvalProvenanceSummary,
    RetrievalEvalRegressionClassification, RetrievalEvalRegressionComparison,
    RetrievalEvalRegressionMetric, RetrievalEvalResult, RetrievalEvalTrendPoint,
    RetrievalEvalTrendSummary, RetrievalMode, RetrievalQualityFlag, RetrievalQueryResponse,
    SearchableChunk,
};

use crate::{
    diagnosis::{diagnose_retrieval, ExpectedEvidence},
    provenance::experiment_compatibility,
};

const DEFAULT_RECALL_THRESHOLD: f32 = 0.80;
const DEFAULT_WEAK_EVIDENCE_LIMIT: f32 = 0.20;
const REGRESSION_RATIO_THRESHOLD: f32 = 0.01;
const LATENCY_REGRESSION_THRESHOLD_MS: f32 = 10.0;
const DEFAULT_TREND_LIMIT: usize = 10;
const MAX_TREND_LIMIT: usize = 50;

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
    evaluate_retrieval_eval_case_with_config(case, response, &DebuggerConfig::default())
}

pub fn evaluate_retrieval_eval_case_with_config(
    case: &RetrievalEvalCase,
    response: &RetrievalQueryResponse,
    debugger_config: &DebuggerConfig,
) -> RetrievalEvalCaseEvaluation {
    evaluate_retrieval_eval_case_with_context(case, response, debugger_config, &[])
}

pub fn evaluate_retrieval_eval_case_with_context(
    case: &RetrievalEvalCase,
    response: &RetrievalQueryResponse,
    debugger_config: &DebuggerConfig,
    expected_chunk_document_ids: &[DocumentId],
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
    let expected_chunk_document_ids = expected_chunk_document_ids
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let matched_expected_chunk_parent_documents = response
        .hits
        .iter()
        .filter(|hit| expected_chunk_document_ids.contains(&hit.document.id))
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
    let wrong_chunk_rank = response
        .hits
        .iter()
        .find(|hit| expected_chunk_document_ids.contains(&hit.document.id))
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
    if !expected_chunk_ids.is_empty()
        && matched_expected_chunks == 0
        && (matched_expected_documents > 0 || matched_expected_chunk_parent_documents > 0)
    {
        failures.push(failure(
            case,
            response.run.retrieval_mode,
            RetrievalEvalFailureLabel::CorrectDocumentWrongChunk,
            RetrievalEvalFailureSeverity::Warning,
            "The expected document matched, but the expected chunk did not.",
            top_hit_rank.or(wrong_chunk_rank),
        ));
    } else if recall_at_k == 0.0 && expected_count > 0 {
        failures.push(failure(
            case,
            response.run.retrieval_mode,
            RetrievalEvalFailureLabel::ExpectedEvidenceMissing,
            RetrievalEvalFailureSeverity::Critical,
            "No expected chunk or document was retrieved.",
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
        provenance: case.provenance.clone(),
        diagnosis: Some(diagnose_retrieval(
            response,
            debugger_config,
            Some(ExpectedEvidence {
                chunk_ids: &case.expected_chunk_ids,
                document_ids: &case.expected_document_ids,
            }),
        )),
    }
}

pub fn expected_chunk_parent_document_ids(
    case: &RetrievalEvalCase,
    candidates: &[SearchableChunk],
) -> Vec<DocumentId> {
    let expected_chunk_ids = case
        .expected_chunk_ids
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let mut document_ids = Vec::new();
    for candidate in candidates {
        if expected_chunk_ids.contains(&candidate.chunk.id)
            && !document_ids.contains(&candidate.document.id)
        {
            document_ids.push(candidate.document.id);
        }
    }
    document_ids
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

pub fn summarize_experiment(
    experiment: &RetrievalEvalExperiment,
) -> RetrievalEvalExperimentSummary {
    let best = headline_mode_result(experiment);
    RetrievalEvalExperimentSummary {
        id: experiment.id,
        dataset_id: experiment.dataset_id,
        dataset_name: experiment.dataset_name.clone(),
        name: experiment.name.clone(),
        modes: experiment.modes.clone(),
        top_k: experiment.top_k,
        best_mode: best.map(|result| result.retrieval_mode),
        gate_status: experiment.gate.status,
        average_recall_at_k: best.map_or(0.0, |result| result.average_recall_at_k),
        average_precision_at_k: best.map_or(0.0, |result| result.average_precision_at_k),
        mean_reciprocal_rank: best.map_or(0.0, |result| result.mean_reciprocal_rank),
        citation_coverage: best.map_or(0.0, |result| result.citation_coverage),
        weak_evidence_case_rate: best.map_or(0.0, weak_evidence_case_rate),
        missing_embedding_failures: best.map_or(0, |result| result.missing_embedding_failures),
        latency_p50_ms: best.map_or(0, |result| result.latency_p50_ms),
        latency_p95_ms: best.map_or(0, |result| result.latency_p95_ms),
        failure_count: experiment.failures.len() as u32,
        contains_full_local_data: experiment_contains_full_local_data(experiment),
        provenance: experiment.provenance.as_ref().map(|provenance| {
            RetrievalEvalProvenanceSummary {
                schema_version: provenance.schema_version,
                fingerprint: provenance.fingerprint.clone(),
            }
        }),
        created_at: experiment.created_at,
    }
}

pub fn build_trend_summary(
    dataset_id: RetrievalEvalDatasetId,
    experiments: &[RetrievalEvalExperiment],
    limit: Option<usize>,
) -> RetrievalEvalTrendSummary {
    let limit = normalize_trend_limit(limit);
    let mut dataset_experiments = experiments
        .iter()
        .filter(|experiment| experiment.dataset_id == dataset_id)
        .collect::<Vec<_>>();
    dataset_experiments.sort_by_key(|experiment| std::cmp::Reverse(experiment.created_at));

    let latest = dataset_experiments.first().copied();
    let latest_regression = latest.map(|current| {
        compare_experiment_regression(
            current,
            previous_comparable_experiment(current, &dataset_experiments),
        )
    });

    let mut points = dataset_experiments
        .iter()
        .take(limit)
        .map(|experiment| trend_point(experiment))
        .collect::<Vec<_>>();
    points.reverse();

    RetrievalEvalTrendSummary {
        dataset_id,
        experiment_count: dataset_experiments.len() as u32,
        window_limit: limit as u32,
        latest_experiment_id: latest.map(|experiment| experiment.id),
        latest_gate_status: latest.map(|experiment| experiment.gate.status),
        points,
        latest_regression,
    }
}

pub fn compare_experiment_regression(
    current: &RetrievalEvalExperiment,
    baseline: Option<&RetrievalEvalExperiment>,
) -> RetrievalEvalRegressionComparison {
    compare_experiment_regression_with_intent(current, baseline, false)
}

pub fn compare_experiment_regression_with_intent(
    current: &RetrievalEvalExperiment,
    baseline: Option<&RetrievalEvalExperiment>,
    intentional_cross_configuration: bool,
) -> RetrievalEvalRegressionComparison {
    let compatibility =
        experiment_compatibility(current, baseline, intentional_cross_configuration);
    let Some(baseline) = baseline else {
        let current_summary = summarize_experiment(current);
        return RetrievalEvalRegressionComparison {
            current_experiment_id: current.id,
            baseline_experiment_id: None,
            compatibility,
            classification: RetrievalEvalRegressionClassification::Unchanged,
            current_gate_status: current.gate.status,
            baseline_gate_status: None,
            metric_deltas: metric_deltas(&current_summary, None),
            newly_failed_cases: Vec::new(),
            recovered_cases: Vec::new(),
            changed_top_evidence_cases: Vec::new(),
            changed_failure_label_cases: Vec::new(),
            summary: "No prior experiment with fully compatible provenance was found.".to_owned(),
        };
    };

    let current_summary = summarize_experiment(current);
    let baseline_summary = summarize_experiment(baseline);
    let metric_deltas = metric_deltas(&current_summary, Some(&baseline_summary));
    let newly_failed_cases = case_regressions(
        current,
        baseline,
        RegressionCaseKind::NewlyFailed,
        RetrievalEvalRegressionClassification::Regressed,
    );
    let recovered_cases = case_regressions(
        current,
        baseline,
        RegressionCaseKind::Recovered,
        RetrievalEvalRegressionClassification::Improved,
    );
    let changed_top_evidence_cases = case_regressions(
        current,
        baseline,
        RegressionCaseKind::ChangedTopEvidence,
        RetrievalEvalRegressionClassification::Unchanged,
    );
    let changed_failure_label_cases = case_regressions(
        current,
        baseline,
        RegressionCaseKind::ChangedFailureLabels,
        RetrievalEvalRegressionClassification::Unchanged,
    );
    let classification = classify_regression(
        current.gate.status,
        baseline.gate.status,
        &metric_deltas,
        !newly_failed_cases.is_empty(),
        !recovered_cases.is_empty(),
    );

    RetrievalEvalRegressionComparison {
        current_experiment_id: current.id,
        baseline_experiment_id: Some(baseline.id),
        compatibility,
        classification,
        current_gate_status: current.gate.status,
        baseline_gate_status: Some(baseline.gate.status),
        metric_deltas,
        newly_failed_cases,
        recovered_cases,
        changed_top_evidence_cases,
        changed_failure_label_cases,
        summary: regression_summary(classification, current, baseline),
    }
}

pub fn previous_comparable_experiment<'a>(
    current: &RetrievalEvalExperiment,
    experiments: &'a [&'a RetrievalEvalExperiment],
) -> Option<&'a RetrievalEvalExperiment> {
    experiments
        .iter()
        .copied()
        .filter(|candidate| candidate.id != current.id)
        .filter(|candidate| candidate.dataset_id == current.dataset_id)
        .filter(|candidate| candidate.created_at < current.created_at)
        .filter(|candidate| {
            experiment_compatibility(current, Some(candidate), false).classification
                == RetrievalEvalCompatibilityClassification::Compatible
        })
        .max_by_key(|candidate| candidate.created_at)
}

fn normalize_trend_limit(limit: Option<usize>) -> usize {
    limit
        .unwrap_or(DEFAULT_TREND_LIMIT)
        .clamp(1, MAX_TREND_LIMIT)
}

fn trend_point(experiment: &RetrievalEvalExperiment) -> RetrievalEvalTrendPoint {
    let summary = summarize_experiment(experiment);
    RetrievalEvalTrendPoint {
        experiment_id: summary.id,
        name: summary.name,
        best_mode: summary.best_mode,
        gate_status: summary.gate_status,
        average_recall_at_k: summary.average_recall_at_k,
        average_precision_at_k: summary.average_precision_at_k,
        mean_reciprocal_rank: summary.mean_reciprocal_rank,
        citation_coverage: summary.citation_coverage,
        weak_evidence_case_rate: summary.weak_evidence_case_rate,
        latency_p95_ms: summary.latency_p95_ms,
        failure_count: summary.failure_count,
        created_at: summary.created_at,
    }
}

fn headline_mode_result(experiment: &RetrievalEvalExperiment) -> Option<&RetrievalEvalModeResult> {
    experiment
        .comparison
        .best_mode
        .and_then(|mode| {
            experiment
                .mode_results
                .iter()
                .find(|result| result.retrieval_mode == mode)
        })
        .or_else(|| {
            experiment.mode_results.iter().max_by(|left, right| {
                left.average_recall_at_k
                    .partial_cmp(&right.average_recall_at_k)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| {
                        left.average_precision_at_k
                            .partial_cmp(&right.average_precision_at_k)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .then_with(|| right.latency_p50_ms.cmp(&left.latency_p50_ms))
            })
        })
}

fn weak_evidence_case_rate(result: &RetrievalEvalModeResult) -> f32 {
    if result.case_count == 0 {
        return 0.0;
    }
    let weak_cases = result
        .case_results
        .iter()
        .filter(|case| {
            case.failures
                .iter()
                .any(|failure| failure.label == RetrievalEvalFailureLabel::WeakEvidence)
        })
        .count() as f32;
    weak_cases / result.case_count as f32
}

fn metric_deltas(
    current: &RetrievalEvalExperimentSummary,
    baseline: Option<&RetrievalEvalExperimentSummary>,
) -> Vec<RetrievalEvalMetricDelta> {
    let metrics = [
        (
            RetrievalEvalRegressionMetric::RecallAtK,
            current.average_recall_at_k,
            baseline.map(|value| value.average_recall_at_k),
            MetricDirection::HigherIsBetter,
            REGRESSION_RATIO_THRESHOLD,
        ),
        (
            RetrievalEvalRegressionMetric::PrecisionAtK,
            current.average_precision_at_k,
            baseline.map(|value| value.average_precision_at_k),
            MetricDirection::HigherIsBetter,
            REGRESSION_RATIO_THRESHOLD,
        ),
        (
            RetrievalEvalRegressionMetric::MeanReciprocalRank,
            current.mean_reciprocal_rank,
            baseline.map(|value| value.mean_reciprocal_rank),
            MetricDirection::HigherIsBetter,
            REGRESSION_RATIO_THRESHOLD,
        ),
        (
            RetrievalEvalRegressionMetric::CitationCoverage,
            current.citation_coverage,
            baseline.map(|value| value.citation_coverage),
            MetricDirection::HigherIsBetter,
            REGRESSION_RATIO_THRESHOLD,
        ),
        (
            RetrievalEvalRegressionMetric::WeakEvidenceCaseRate,
            current.weak_evidence_case_rate,
            baseline.map(|value| value.weak_evidence_case_rate),
            MetricDirection::LowerIsBetter,
            REGRESSION_RATIO_THRESHOLD,
        ),
        (
            RetrievalEvalRegressionMetric::MissingEmbeddingFailures,
            current.missing_embedding_failures as f32,
            baseline.map(|value| value.missing_embedding_failures as f32),
            MetricDirection::LowerIsBetter,
            0.0,
        ),
        (
            RetrievalEvalRegressionMetric::LatencyP95Ms,
            current.latency_p95_ms as f32,
            baseline.map(|value| value.latency_p95_ms as f32),
            MetricDirection::LowerIsBetter,
            LATENCY_REGRESSION_THRESHOLD_MS,
        ),
    ];

    metrics
        .into_iter()
        .map(|(metric, current, baseline, direction, threshold)| {
            let delta = baseline.map_or(0.0, |baseline| current - baseline);
            RetrievalEvalMetricDelta {
                metric,
                current,
                baseline,
                delta,
                classification: baseline
                    .map_or(RetrievalEvalRegressionClassification::Unchanged, |_| {
                        classify_metric_delta(delta, direction, threshold)
                    }),
            }
        })
        .collect()
}

#[derive(Clone, Copy)]
enum MetricDirection {
    HigherIsBetter,
    LowerIsBetter,
}

fn classify_metric_delta(
    delta: f32,
    direction: MetricDirection,
    threshold: f32,
) -> RetrievalEvalRegressionClassification {
    if delta.abs() <= threshold {
        return RetrievalEvalRegressionClassification::Unchanged;
    }
    match direction {
        MetricDirection::HigherIsBetter if delta > 0.0 => {
            RetrievalEvalRegressionClassification::Improved
        }
        MetricDirection::HigherIsBetter => RetrievalEvalRegressionClassification::Regressed,
        MetricDirection::LowerIsBetter if delta < 0.0 => {
            RetrievalEvalRegressionClassification::Improved
        }
        MetricDirection::LowerIsBetter => RetrievalEvalRegressionClassification::Regressed,
    }
}

fn classify_regression(
    current_gate: RetrievalEvalGateStatus,
    baseline_gate: RetrievalEvalGateStatus,
    metric_deltas: &[RetrievalEvalMetricDelta],
    has_newly_failed_cases: bool,
    has_recovered_cases: bool,
) -> RetrievalEvalRegressionClassification {
    if baseline_gate == RetrievalEvalGateStatus::Passed
        && current_gate == RetrievalEvalGateStatus::Failed
    {
        return RetrievalEvalRegressionClassification::Regressed;
    }
    if has_newly_failed_cases
        || metric_deltas
            .iter()
            .any(|delta| delta.classification == RetrievalEvalRegressionClassification::Regressed)
    {
        return RetrievalEvalRegressionClassification::Regressed;
    }
    if baseline_gate == RetrievalEvalGateStatus::Failed
        && current_gate == RetrievalEvalGateStatus::Passed
    {
        return RetrievalEvalRegressionClassification::Improved;
    }
    if has_recovered_cases
        || metric_deltas
            .iter()
            .any(|delta| delta.classification == RetrievalEvalRegressionClassification::Improved)
    {
        return RetrievalEvalRegressionClassification::Improved;
    }
    RetrievalEvalRegressionClassification::Unchanged
}

#[derive(Clone, Copy)]
enum RegressionCaseKind {
    NewlyFailed,
    Recovered,
    ChangedTopEvidence,
    ChangedFailureLabels,
}

fn case_regressions(
    current: &RetrievalEvalExperiment,
    baseline: &RetrievalEvalExperiment,
    kind: RegressionCaseKind,
    classification: RetrievalEvalRegressionClassification,
) -> Vec<RetrievalEvalCaseRegression> {
    let mut regressions = Vec::new();
    for current_case in all_case_results(current) {
        let baseline_case = find_case_result(
            baseline,
            current_case.retrieval_mode,
            current_case.case.case_id,
        );
        let include = match kind {
            RegressionCaseKind::NewlyFailed => {
                case_failed(current_case) && baseline_case.is_some_and(|case| !case_failed(case))
            }
            RegressionCaseKind::Recovered => {
                !case_failed(current_case) && baseline_case.is_some_and(case_failed)
            }
            RegressionCaseKind::ChangedTopEvidence => baseline_case.is_some_and(|case| {
                current_case.case.top_hit_rank != case.case.top_hit_rank
                    || first_chunk_id(current_case) != first_chunk_id(case)
            }),
            RegressionCaseKind::ChangedFailureLabels => baseline_case
                .is_some_and(|case| failure_labels(current_case) != failure_labels(case)),
        };
        if include {
            regressions.push(case_regression(current_case, baseline_case, classification));
        }
    }
    sort_case_regressions(&mut regressions);
    regressions
}

fn all_case_results(
    experiment: &RetrievalEvalExperiment,
) -> impl Iterator<Item = CaseResultWithMode<'_>> {
    experiment.mode_results.iter().flat_map(|mode| {
        mode.case_results.iter().map(|case| CaseResultWithMode {
            retrieval_mode: mode.retrieval_mode,
            case,
        })
    })
}

#[derive(Clone, Copy)]
struct CaseResultWithMode<'a> {
    retrieval_mode: RetrievalMode,
    case: &'a RetrievalEvalCaseEvaluation,
}

fn find_case_result(
    experiment: &RetrievalEvalExperiment,
    retrieval_mode: RetrievalMode,
    case_id: RetrievalEvalCaseId,
) -> Option<CaseResultWithMode<'_>> {
    experiment
        .mode_results
        .iter()
        .find(|mode| mode.retrieval_mode == retrieval_mode)
        .and_then(|mode| {
            mode.case_results
                .iter()
                .find(|case| case.case_id == case_id)
                .map(|case| CaseResultWithMode {
                    retrieval_mode,
                    case,
                })
        })
}

fn case_failed(case: CaseResultWithMode<'_>) -> bool {
    !case.case.passed || !case.case.failures.is_empty()
}

fn case_regression(
    current: CaseResultWithMode<'_>,
    baseline: Option<CaseResultWithMode<'_>>,
    classification: RetrievalEvalRegressionClassification,
) -> RetrievalEvalCaseRegression {
    RetrievalEvalCaseRegression {
        case_id: current.case.case_id,
        retrieval_mode: current.retrieval_mode,
        query: current.case.query.clone(),
        classification,
        current_passed: Some(current.case.passed),
        baseline_passed: baseline.map(|case| case.case.passed),
        current_top_hit_rank: current.case.top_hit_rank,
        baseline_top_hit_rank: baseline.and_then(|case| case.case.top_hit_rank),
        current_retrieved_chunk_ids: current.case.retrieved_chunk_ids.clone(),
        baseline_retrieved_chunk_ids: baseline
            .map(|case| case.case.retrieved_chunk_ids.clone())
            .unwrap_or_default(),
        current_failure_labels: failure_labels(current),
        baseline_failure_labels: baseline.map(failure_labels).unwrap_or_default(),
    }
}

fn failure_labels(case: CaseResultWithMode<'_>) -> Vec<RetrievalEvalFailureLabel> {
    let mut labels = case
        .case
        .failures
        .iter()
        .map(|failure| failure.label)
        .collect::<Vec<_>>();
    labels.sort_by_key(|label| failure_label_code(*label));
    labels.dedup();
    labels
}

fn first_chunk_id(case: CaseResultWithMode<'_>) -> Option<String> {
    case.case
        .retrieved_chunk_ids
        .first()
        .map(|chunk_id| chunk_id.0.to_string())
}

fn sort_case_regressions(regressions: &mut [RetrievalEvalCaseRegression]) {
    regressions.sort_by(|left, right| {
        left.case_id
            .0
            .to_string()
            .cmp(&right.case_id.0.to_string())
            .then_with(|| mode_code(left.retrieval_mode).cmp(mode_code(right.retrieval_mode)))
    });
}

fn mode_code(mode: RetrievalMode) -> &'static str {
    match mode {
        RetrievalMode::Lexical => "lexical",
        RetrievalMode::Vector => "vector",
        RetrievalMode::Hybrid => "hybrid",
    }
}

fn failure_label_code(label: RetrievalEvalFailureLabel) -> &'static str {
    match label {
        RetrievalEvalFailureLabel::ExpectedEvidenceMissing => "expected_evidence_missing",
        RetrievalEvalFailureLabel::CorrectDocumentWrongChunk => "correct_document_wrong_chunk",
        RetrievalEvalFailureLabel::LowPrecision => "low_precision",
        RetrievalEvalFailureLabel::WeakEvidence => "weak_evidence",
        RetrievalEvalFailureLabel::MissingEmbeddings => "missing_embeddings",
        RetrievalEvalFailureLabel::HeadingOnlyEvidence => "heading_only_evidence",
        RetrievalEvalFailureLabel::DuplicateEvidence => "duplicate_evidence",
    }
}

fn regression_summary(
    classification: RetrievalEvalRegressionClassification,
    current: &RetrievalEvalExperiment,
    baseline: &RetrievalEvalExperiment,
) -> String {
    match classification {
        RetrievalEvalRegressionClassification::Improved => {
            format!("{} improved compared with {}.", current.name, baseline.name)
        }
        RetrievalEvalRegressionClassification::Regressed => format!(
            "{} regressed compared with {}.",
            current.name, baseline.name
        ),
        RetrievalEvalRegressionClassification::Unchanged => format!(
            "{} stayed within regression thresholds compared with {}.",
            current.name, baseline.name
        ),
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
        RetrievalEmbeddingStatus, RetrievalEvalCase, RetrievalEvalCaseEvaluation,
        RetrievalEvalCaseId, RetrievalEvalConfigSnapshot, RetrievalEvalDatasetId,
        RetrievalEvalExperiment, RetrievalEvalExperimentId, RetrievalEvalExperimentProvenance,
        RetrievalEvalFailure, RetrievalEvalFailureSeverity, RetrievalEvalGateStatus,
        RetrievalEvalRegressionClassification, RetrievalMatchedTerm, RetrievalMode,
        RetrievalQueryHit, RetrievalQueryResponse, RetrievalQueryRun, RetrievalQueryRunId,
        RetrievalScoreBreakdown, Source, SourceId, SourceKind, SourceSyncPolicy,
    };
    use time::{Duration, OffsetDateTime};
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
            provenance: None,
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
            diagnosis: None,
        };

        let result = score_retrieval_eval_case(&case, &response);

        assert!(result.passed);
        assert_eq!(result.recall_at_k, 1.0);
        assert_eq!(result.precision_at_k, 1.0);
        assert_eq!(result.top_hit_rank, Some(1));
    }

    #[test]
    fn failure_diagnosis_is_deterministic_for_degraded_evidence() {
        let document_id = DocumentId(Uuid::now_v7());
        let expected_chunk_id = ChunkId(Uuid::now_v7());
        let case = eval_case(expected_chunk_id, Some(document_id));
        let mut degraded_hit = hit(ChunkId(Uuid::now_v7()), document_id);
        degraded_hit.evidence_strength = EvidenceStrength::Weak;
        degraded_hit.duplicate_count = 2;
        degraded_hit.quality_flags = vec![
            RetrievalQualityFlag::HeadingOnly,
            RetrievalQualityFlag::Duplicate,
        ];
        let response = eval_response(vec![degraded_hit], RetrievalEmbeddingReadiness::Missing);

        let first = evaluate_retrieval_eval_case(&case, &response);
        let second = evaluate_retrieval_eval_case(&case, &response);
        let labels = first
            .failures
            .iter()
            .map(|failure| failure.label)
            .collect::<Vec<_>>();

        assert_eq!(first.failures, second.failures);
        assert_eq!(
            labels,
            vec![
                RetrievalEvalFailureLabel::MissingEmbeddings,
                RetrievalEvalFailureLabel::CorrectDocumentWrongChunk,
                RetrievalEvalFailureLabel::WeakEvidence,
                RetrievalEvalFailureLabel::HeadingOnlyEvidence,
                RetrievalEvalFailureLabel::DuplicateEvidence,
            ]
        );
    }

    #[test]
    fn exact_chunk_only_expectation_can_report_same_document_wrong_chunk() {
        let document_id = DocumentId(Uuid::now_v7());
        let expected_chunk_id = ChunkId(Uuid::now_v7());
        let retrieved_chunk_id = ChunkId(Uuid::now_v7());
        let case = eval_case(expected_chunk_id, None);
        let response = eval_response(
            vec![hit(retrieved_chunk_id, document_id)],
            RetrievalEmbeddingReadiness::Ready,
        );

        let result = evaluate_retrieval_eval_case_with_context(
            &case,
            &response,
            &DebuggerConfig::default(),
            &[document_id],
        );
        let labels = result
            .failures
            .iter()
            .map(|failure| failure.label)
            .collect::<Vec<_>>();

        assert!(!result.passed);
        assert_eq!(result.recall_at_k, 0.0);
        assert!(result.expected_document_ids.is_empty());
        assert!(labels.contains(&RetrievalEvalFailureLabel::CorrectDocumentWrongChunk));
        assert!(!labels.contains(&RetrievalEvalFailureLabel::ExpectedEvidenceMissing));
    }

    #[test]
    fn missing_expected_evidence_also_reports_low_precision() {
        let expected_chunk_id = ChunkId(Uuid::now_v7());
        let case = eval_case(expected_chunk_id, None);
        let response = eval_response(
            vec![
                hit(ChunkId(Uuid::now_v7()), DocumentId(Uuid::now_v7())),
                hit(ChunkId(Uuid::now_v7()), DocumentId(Uuid::now_v7())),
            ],
            RetrievalEmbeddingReadiness::Ready,
        );

        let result = evaluate_retrieval_eval_case(&case, &response);
        let labels = result
            .failures
            .iter()
            .map(|failure| failure.label)
            .collect::<Vec<_>>();

        assert_eq!(result.recall_at_k, 0.0);
        assert_eq!(result.precision_at_k, 0.0);
        assert_eq!(
            labels,
            vec![
                RetrievalEvalFailureLabel::ExpectedEvidenceMissing,
                RetrievalEvalFailureLabel::LowPrecision,
            ]
        );
    }

    #[test]
    fn experiment_regression_detects_newly_failed_gate() {
        let case_id = RetrievalEvalCaseId(Uuid::now_v7());
        let expected_chunk = ChunkId(Uuid::now_v7());
        let wrong_chunk = ChunkId(Uuid::now_v7());
        let baseline = experiment(
            "Baseline",
            OffsetDateTime::now_utc() - Duration::minutes(5),
            vec![case_evaluation(
                case_id,
                expected_chunk,
                expected_chunk,
                true,
            )],
        );
        let current = experiment(
            "Current",
            OffsetDateTime::now_utc(),
            vec![case_evaluation(case_id, expected_chunk, wrong_chunk, false)],
        );

        let comparison = compare_experiment_regression(&current, Some(&baseline));

        assert_eq!(
            comparison.classification,
            RetrievalEvalRegressionClassification::Regressed
        );
        assert_eq!(comparison.baseline_experiment_id, Some(baseline.id));
        assert_eq!(
            comparison.current_gate_status,
            RetrievalEvalGateStatus::Failed
        );
        assert_eq!(
            comparison.baseline_gate_status,
            Some(RetrievalEvalGateStatus::Passed)
        );
        assert_eq!(comparison.newly_failed_cases.len(), 1);
        assert_eq!(comparison.recovered_cases.len(), 0);
        assert!(comparison
            .metric_deltas
            .iter()
            .any(|delta| delta.classification == RetrievalEvalRegressionClassification::Regressed));
    }

    #[test]
    fn trend_summary_finds_previous_comparable_experiment() {
        let case_id = RetrievalEvalCaseId(Uuid::now_v7());
        let expected_chunk = ChunkId(Uuid::now_v7());
        let wrong_chunk = ChunkId(Uuid::now_v7());
        let dataset_id = RetrievalEvalDatasetId(Uuid::now_v7());
        let older = experiment_with_dataset(
            dataset_id,
            "Older",
            OffsetDateTime::now_utc() - Duration::minutes(10),
            vec![case_evaluation(
                case_id,
                expected_chunk,
                expected_chunk,
                true,
            )],
        );
        let latest = experiment_with_dataset(
            dataset_id,
            "Latest",
            OffsetDateTime::now_utc(),
            vec![case_evaluation(case_id, expected_chunk, wrong_chunk, false)],
        );

        let trend = build_trend_summary(dataset_id, &[latest.clone(), older.clone()], Some(99));

        assert_eq!(trend.window_limit, 50);
        assert_eq!(trend.points.len(), 2);
        assert_eq!(trend.points[0].experiment_id, older.id);
        assert_eq!(trend.latest_experiment_id, Some(latest.id));
        assert_eq!(
            trend
                .latest_regression
                .as_ref()
                .map(|value| value.classification),
            Some(RetrievalEvalRegressionClassification::Regressed)
        );
    }

    #[test]
    fn automatic_baseline_excludes_incompatible_and_legacy_experiments() {
        let dataset_id = RetrievalEvalDatasetId(Uuid::now_v7());
        let now = OffsetDateTime::now_utc();
        let current = experiment_with_dataset(dataset_id, "Current", now, Vec::new());
        let compatible = experiment_with_dataset(
            dataset_id,
            "Compatible",
            now - Duration::minutes(3),
            Vec::new(),
        );
        let mut incompatible = experiment_with_dataset(
            dataset_id,
            "Incompatible",
            now - Duration::minutes(1),
            Vec::new(),
        );
        incompatible
            .provenance
            .as_mut()
            .unwrap()
            .identity
            .retrieval
            .top_k = 10;
        let mut legacy =
            experiment_with_dataset(dataset_id, "Legacy", now - Duration::minutes(2), Vec::new());
        legacy.provenance = None;

        assert_eq!(
            previous_comparable_experiment(&current, &[&incompatible, &legacy, &compatible])
                .map(|experiment| experiment.id),
            Some(compatible.id)
        );
        assert!(previous_comparable_experiment(&current, &[&incompatible, &legacy]).is_none());
    }

    #[test]
    fn regression_tracks_changed_top_evidence_and_failure_labels() {
        let case_id = RetrievalEvalCaseId(Uuid::now_v7());
        let expected_chunk = ChunkId(Uuid::now_v7());
        let first_wrong_chunk = ChunkId(Uuid::now_v7());
        let second_wrong_chunk = ChunkId(Uuid::now_v7());
        let baseline = experiment(
            "Baseline",
            OffsetDateTime::now_utc() - Duration::minutes(5),
            vec![case_evaluation_with_failure(
                case_id,
                expected_chunk,
                first_wrong_chunk,
                RetrievalEvalFailureLabel::LowPrecision,
            )],
        );
        let current = experiment(
            "Current",
            OffsetDateTime::now_utc(),
            vec![case_evaluation_with_failure(
                case_id,
                expected_chunk,
                second_wrong_chunk,
                RetrievalEvalFailureLabel::ExpectedEvidenceMissing,
            )],
        );

        let comparison = compare_experiment_regression(&current, Some(&baseline));

        assert_eq!(comparison.changed_top_evidence_cases.len(), 1);
        assert_eq!(comparison.changed_failure_label_cases.len(), 1);
        assert_eq!(
            comparison.changed_failure_label_cases[0].baseline_failure_labels,
            vec![RetrievalEvalFailureLabel::LowPrecision]
        );
        assert_eq!(
            comparison.changed_failure_label_cases[0].current_failure_labels,
            vec![RetrievalEvalFailureLabel::ExpectedEvidenceMissing]
        );
    }

    fn eval_case(
        expected_chunk_id: ChunkId,
        expected_document_id: Option<DocumentId>,
    ) -> RetrievalEvalCase {
        RetrievalEvalCase {
            id: RetrievalEvalCaseId(Uuid::now_v7()),
            name: "Fixture expectation".to_owned(),
            query: "How is GPU indexing configured?".to_owned(),
            top_k: 5,
            expected_chunk_ids: vec![expected_chunk_id],
            expected_document_ids: expected_document_id.into_iter().collect(),
            notes: Some("Deterministic regression fixture".to_owned()),
            provenance: None,
            created_at: OffsetDateTime::now_utc(),
        }
    }

    fn eval_response(
        hits: Vec<RetrievalQueryHit>,
        readiness: RetrievalEmbeddingReadiness,
    ) -> RetrievalQueryResponse {
        RetrievalQueryResponse {
            run: RetrievalQueryRun {
                id: RetrievalQueryRunId(Uuid::now_v7()),
                query: "How is GPU indexing configured?".to_owned(),
                top_k: 5,
                retrieval_mode: RetrievalMode::Hybrid,
                latency_ms: 7,
                created_at: OffsetDateTime::now_utc(),
            },
            answer: ExtractiveAnswer {
                status: ExtractiveAnswerStatus::InsufficientEvidence,
                text: "Not enough local evidence.".to_owned(),
                citations: Vec::new(),
            },
            embedding_status: RetrievalEmbeddingStatus {
                readiness,
                required: true,
                model: EmbeddingModelInfo::default(),
                total_chunks: hits.len() as u32,
                indexed_chunks: if readiness == RetrievalEmbeddingReadiness::Ready {
                    hits.len() as u32
                } else {
                    0
                },
                missing_chunks: if readiness == RetrievalEmbeddingReadiness::Missing {
                    hits.len() as u32
                } else {
                    0
                },
                stale_chunks: 0,
            },
            hits,
            diagnosis: None,
        }
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
            answer_support: Default::default(),
        }
    }

    fn experiment(
        name: &str,
        created_at: OffsetDateTime,
        case_results: Vec<RetrievalEvalCaseEvaluation>,
    ) -> RetrievalEvalExperiment {
        experiment_with_dataset(
            RetrievalEvalDatasetId(Uuid::now_v7()),
            name,
            created_at,
            case_results,
        )
    }

    fn experiment_with_dataset(
        dataset_id: RetrievalEvalDatasetId,
        name: &str,
        created_at: OffsetDateTime,
        case_results: Vec<RetrievalEvalCaseEvaluation>,
    ) -> RetrievalEvalExperiment {
        let mode_results = vec![summarize_mode_result(RetrievalMode::Hybrid, case_results)];
        let comparison = compare_mode_results(&mode_results);
        let gate = evaluate_gate(&mode_results);
        let failures = mode_results
            .iter()
            .flat_map(|mode| &mode.case_results)
            .flat_map(|result| result.failures.iter().cloned())
            .collect::<Vec<_>>();
        RetrievalEvalExperiment {
            id: RetrievalEvalExperimentId(Uuid::now_v7()),
            dataset_id,
            dataset_name: "Regression dataset".to_owned(),
            name: name.to_owned(),
            modes: vec![RetrievalMode::Hybrid],
            top_k: 5,
            config_snapshot: RetrievalEvalConfigSnapshot {
                top_k: 5,
                scoring_weights: Default::default(),
                embedding_model: EmbeddingModelInfo::default(),
                dataset_case_count: 1,
            },
            provenance: Some(provenance_fixture(dataset_id)),
            mode_results,
            comparison,
            gate,
            failures,
            created_at,
        }
    }

    fn provenance_fixture(dataset_id: RetrievalEvalDatasetId) -> RetrievalEvalExperimentProvenance {
        serde_json::from_value(serde_json::json!({
            "schema_version": 1,
            "fingerprint": "stable-identity",
            "identity": {
                "workspace_id": "00000000-0000-0000-0000-000000000001",
                "project_ids": [],
                "dataset": {"dataset_id": dataset_id, "revision_fingerprint": "dataset", "case_count": 1},
                "corpus": {"source_ids": [], "document_count": 0, "document_set_fingerprint": "documents", "documents": []},
                "chunking": {"fingerprint": "chunking", "sources": []},
                "chunk_set": {"fingerprint": "chunks", "chunk_count": 0},
                "embedding": {"provider": "local", "model_name": "local-hash-v1", "dimension": 384, "index_fingerprint": "index", "indexed_chunk_count": 0, "missing_chunk_count": 0, "stale_chunk_count": 0},
                "retrieval": {
                    "modes": ["hybrid"],
                    "top_k": 5,
                    "scoring": {
                        "weights": rag_debugger_core::RetrievalWeights::default(),
                        "min_evidence_score": 0.35,
                        "min_semantic_similarity": 0.25,
                        "answer_citation_limit": 3,
                        "answerability": rag_debugger_core::AnswerabilityConfig::default()
                    },
                    "filters": {"source_ids": [], "document_ids": []},
                    "runtime_flags": {}
                }
            },
            "informational": {
                "application_version": "test",
                "deployment_mode": "local",
                "runtime_environment": "test",
                "storage_backend": "memory",
                "labels": {}
            }
        }))
        .unwrap()
    }

    fn case_evaluation(
        case_id: RetrievalEvalCaseId,
        expected_chunk: ChunkId,
        retrieved_chunk: ChunkId,
        passed: bool,
    ) -> RetrievalEvalCaseEvaluation {
        let failures = if passed {
            Vec::new()
        } else {
            vec![eval_failure(
                case_id,
                RetrievalEvalFailureLabel::ExpectedEvidenceMissing,
            )]
        };
        RetrievalEvalCaseEvaluation {
            case_id,
            query: "Which evidence should be retrieved?".to_owned(),
            top_k: 5,
            recall_at_k: if passed { 1.0 } else { 0.0 },
            precision_at_k: if passed { 1.0 } else { 0.0 },
            mrr: if passed { 1.0 } else { 0.0 },
            top_hit_rank: if passed { Some(1) } else { Some(4) },
            citation_coverage: if passed { 1.0 } else { 0.0 },
            weak_evidence_count: 0,
            missing_embedding_failures: 0,
            passed,
            expected_chunk_ids: vec![expected_chunk],
            expected_document_ids: Vec::new(),
            retrieved_chunk_ids: vec![retrieved_chunk],
            latency_ms: if passed { 20 } else { 35 },
            failures,
            provenance: None,
            diagnosis: None,
        }
    }

    fn case_evaluation_with_failure(
        case_id: RetrievalEvalCaseId,
        expected_chunk: ChunkId,
        retrieved_chunk: ChunkId,
        label: RetrievalEvalFailureLabel,
    ) -> RetrievalEvalCaseEvaluation {
        let mut evaluation = case_evaluation(case_id, expected_chunk, retrieved_chunk, false);
        evaluation.failures = vec![eval_failure(case_id, label)];
        evaluation
    }

    fn eval_failure(
        case_id: RetrievalEvalCaseId,
        label: RetrievalEvalFailureLabel,
    ) -> RetrievalEvalFailure {
        RetrievalEvalFailure {
            case_id,
            query: "Which evidence should be retrieved?".to_owned(),
            retrieval_mode: RetrievalMode::Hybrid,
            label,
            severity: RetrievalEvalFailureSeverity::Critical,
            message: "Expected evidence was not retrieved.".to_owned(),
            top_hit_rank: Some(4),
        }
    }
}
