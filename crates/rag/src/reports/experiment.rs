use std::collections::BTreeMap;

use rag_debugger_core::{
    DebugReport, DebugReportFinding, DebugReportSeverity, DebugReportSource,
    DiagnosisRecommendation, EvidenceDiagnosisSummary, RetrievalEvalCompatibilityClassification,
    RetrievalEvalExperiment, RetrievalEvalFailureLabel, RetrievalEvalFailureSeverity,
    RetrievalEvalGateStatus, RetrievalEvalRegressionClassification,
    RetrievalEvalRegressionComparison,
};

use super::{
    evidence::experiment_evidence,
    privacy::permits_content,
    recommendations::{
        debug_report_recommendations, recommendations_for_failure_codes,
        retrieval_mode_recommendation,
    },
    retrieval_mode_label, DebugReportBuildContext,
};

pub fn build_eval_experiment_debug_report(
    context: DebugReportBuildContext,
    experiment: &RetrievalEvalExperiment,
) -> DebugReport {
    build_eval_experiment_debug_report_with_regression(context, experiment, None)
}

pub fn build_eval_experiment_debug_report_with_regression(
    context: DebugReportBuildContext,
    experiment: &RetrievalEvalExperiment,
    regression: Option<&RetrievalEvalRegressionComparison>,
) -> DebugReport {
    let (evidence, case_evidence) = experiment_evidence(experiment);
    let diagnosis = experiment_diagnosis(experiment);
    let mut findings = Vec::new();
    let mut failure_codes = Vec::new();

    for failure in &experiment.failures {
        let code = eval_failure_code(failure.label);
        failure_codes.push(code.to_owned());
        findings.push(DebugReportFinding {
            code: format!(
                "{code}:{}:{}",
                failure.case_id.0,
                retrieval_mode_label(failure.retrieval_mode)
            ),
            severity: match failure.severity {
                RetrievalEvalFailureSeverity::Critical => DebugReportSeverity::Critical,
                RetrievalEvalFailureSeverity::Warning => DebugReportSeverity::Warning,
            },
            title: if permits_content(context.privacy_mode) {
                failure.query.clone()
            } else {
                format!("Eval case {}", failure.case_id.0)
            },
            summary: failure.message.clone(),
            failure_labels: vec![code.to_owned()],
            evidence_refs: case_evidence
                .get(&failure.case_id)
                .cloned()
                .unwrap_or_default(),
        });
    }

    if findings.is_empty() {
        findings.push(DebugReportFinding {
            code: "eval_gate_passed".to_owned(),
            severity: DebugReportSeverity::Info,
            title: "No failed eval cases were recorded".to_owned(),
            summary: experiment.gate.reasons.join(" "),
            failure_labels: Vec::new(),
            evidence_refs: Vec::new(),
        });
    }

    if experiment.comparison.mode_count > 1 {
        findings.push(DebugReportFinding {
            code: "retrieval_mode_comparison".to_owned(),
            severity: DebugReportSeverity::Info,
            title: "Retrieval modes produced different outcomes".to_owned(),
            summary: experiment.comparison.summary.clone(),
            failure_labels: Vec::new(),
            evidence_refs: Vec::new(),
        });
    }

    if let Some(regression) = regression {
        if regression.compatibility.classification
            != RetrievalEvalCompatibilityClassification::Compatible
            && regression.baseline_experiment_id.is_some()
        {
            findings.push(DebugReportFinding {
                code: "baseline_compatibility_warning".to_owned(),
                severity: DebugReportSeverity::Warning,
                title: "Baseline configuration is not fully compatible".to_owned(),
                summary: format!(
                    "Compatibility is {}. Changed identity fields: {}.",
                    compatibility_label(regression.compatibility.classification),
                    if regression.compatibility.changed_fields.is_empty() {
                        "unknown".to_owned()
                    } else {
                        regression.compatibility.changed_fields.join(", ")
                    }
                ),
                failure_labels: vec!["baseline_compatibility".to_owned()],
                evidence_refs: Vec::new(),
            });
        }
        findings.push(DebugReportFinding {
            code: "eval_regression_comparison".to_owned(),
            severity: match regression.classification {
                RetrievalEvalRegressionClassification::Regressed => DebugReportSeverity::Warning,
                RetrievalEvalRegressionClassification::Improved => DebugReportSeverity::Info,
                RetrievalEvalRegressionClassification::Unchanged => DebugReportSeverity::Info,
            },
            title: format!(
                "Experiment {}",
                classification_label(regression.classification)
            ),
            summary: format!(
                "{} Newly failed: {}. Recovered: {}.",
                regression.summary,
                regression.newly_failed_cases.len(),
                regression.recovered_cases.len()
            ),
            failure_labels: if regression.classification
                == RetrievalEvalRegressionClassification::Regressed
            {
                vec!["eval_regression".to_owned()]
            } else {
                Vec::new()
            },
            evidence_refs: Vec::new(),
        });
        for case in regression.newly_failed_cases.iter().take(5) {
            findings.push(DebugReportFinding {
                code: format!("newly_failed_case:{}", case.case_id.0),
                severity: DebugReportSeverity::Warning,
                title: if permits_content(context.privacy_mode) {
                    case.query.clone()
                } else {
                    format!("Eval case {}", case.case_id.0)
                },
                summary: format!(
                    "{} newly failed compared with the baseline experiment.",
                    retrieval_mode_label(case.retrieval_mode)
                ),
                failure_labels: case
                    .current_failure_labels
                    .iter()
                    .map(|label| eval_failure_code(*label).to_owned())
                    .collect(),
                evidence_refs: Vec::new(),
            });
        }
    }

    let mut recommendations = diagnosis.as_ref().map_or_else(
        || recommendations_for_failure_codes(&failure_codes),
        |diagnosis| debug_report_recommendations(&diagnosis.recommendations),
    );
    if experiment.comparison.recall_delta > 0.0 {
        if let Some(best_mode) = experiment.comparison.best_mode {
            recommendations.push(retrieval_mode_recommendation(retrieval_mode_label(
                best_mode,
            )));
        }
    }

    DebugReport {
        id: context.report_id,
        workspace_id: context.workspace_id,
        project_id: context.project_id,
        title: "RAG evaluation audit".to_owned(),
        subject: if permits_content(context.privacy_mode) {
            experiment.dataset_name.clone()
        } else {
            format!("Eval dataset {}", experiment.dataset_id.0)
        },
        source: DebugReportSource::EvalExperiment {
            experiment_id: experiment.id,
        },
        privacy_mode: context.privacy_mode,
        executive_summary: eval_summary_with_regression(experiment, regression),
        context: experiment_context_with_regression(experiment, regression),
        findings,
        recommendations,
        evidence,
        diagnosis,
        created_at: context.created_at,
    }
}

fn experiment_diagnosis(experiment: &RetrievalEvalExperiment) -> Option<EvidenceDiagnosisSummary> {
    let mut failures = Vec::new();
    let mut recommendations = Vec::<DiagnosisRecommendation>::new();
    for diagnosis in experiment
        .mode_results
        .iter()
        .flat_map(|mode| &mode.case_results)
        .filter_map(|result| result.diagnosis.as_ref())
    {
        for failure in &diagnosis.failures {
            if !failures
                .iter()
                .any(|existing: &rag_debugger_core::DiagnosisFailure| existing.code == failure.code)
            {
                let mut failure = failure.clone();
                failure.evidence_refs.clear();
                failures.push(failure);
            }
        }
        for recommendation in &diagnosis.recommendations {
            if !recommendations
                .iter()
                .any(|existing| existing.code == recommendation.code)
            {
                let mut recommendation = recommendation.clone();
                recommendation.evidence_refs.clear();
                recommendations.push(recommendation);
            }
        }
    }
    if failures.is_empty() && recommendations.is_empty() {
        return None;
    }

    let outcome = match experiment.gate.status {
        RetrievalEvalGateStatus::Failed => rag_debugger_core::DiagnosisOutcome::Failing,
        RetrievalEvalGateStatus::Passed => rag_debugger_core::DiagnosisOutcome::Mixed,
    };
    Some(EvidenceDiagnosisSummary {
        outcome,
        summary: eval_summary(experiment),
        primary_issue: failures.first().cloned(),
        failures,
        score_explanations: Vec::new(),
        recommendations,
    })
}

fn experiment_context_with_regression(
    experiment: &RetrievalEvalExperiment,
    regression: Option<&RetrievalEvalRegressionComparison>,
) -> BTreeMap<String, String> {
    let mut context = BTreeMap::new();
    context.insert(
        "dataset_case_count".to_owned(),
        experiment.config_snapshot.dataset_case_count.to_string(),
    );
    context.insert(
        "embedding_model".to_owned(),
        experiment
            .config_snapshot
            .embedding_model
            .model_name
            .clone(),
    );
    context.insert(
        "gate_status".to_owned(),
        gate_status_label(experiment.gate.status).to_owned(),
    );
    context.insert("mode_count".to_owned(), experiment.modes.len().to_string());
    context.insert("top_k".to_owned(), experiment.top_k.to_string());
    if let Some(provenance) = &experiment.provenance {
        context.insert(
            "provenance_schema_version".to_owned(),
            provenance.schema_version.to_string(),
        );
        context.insert(
            "experiment_fingerprint".to_owned(),
            provenance.fingerprint.clone(),
        );
        context.insert(
            "dataset_revision_fingerprint".to_owned(),
            provenance.identity.dataset.revision_fingerprint.clone(),
        );
        context.insert(
            "document_set_fingerprint".to_owned(),
            provenance.identity.corpus.document_set_fingerprint.clone(),
        );
        context.insert(
            "document_count".to_owned(),
            provenance.identity.corpus.document_count.to_string(),
        );
        context.insert(
            "chunking_fingerprint".to_owned(),
            provenance.identity.chunking.fingerprint.clone(),
        );
        context.insert(
            "chunk_set_fingerprint".to_owned(),
            provenance.identity.chunk_set.fingerprint.clone(),
        );
        context.insert(
            "chunk_count".to_owned(),
            provenance.identity.chunk_set.chunk_count.to_string(),
        );
        context.insert(
            "embedding_provider".to_owned(),
            provenance.identity.embedding.provider.clone(),
        );
        context.insert(
            "embedding_dimension".to_owned(),
            provenance.identity.embedding.dimension.to_string(),
        );
        context.insert(
            "embedding_index_fingerprint".to_owned(),
            provenance.identity.embedding.index_fingerprint.clone(),
        );
    } else {
        context.insert("provenance_status".to_owned(), "legacy_unknown".to_owned());
    }
    if let Some(best_mode) = experiment.comparison.best_mode {
        context.insert(
            "best_retrieval_mode".to_owned(),
            retrieval_mode_label(best_mode).to_owned(),
        );
    }
    for result in &experiment.mode_results {
        let prefix = retrieval_mode_label(result.retrieval_mode);
        context.insert(
            format!("{prefix}.recall_at_k"),
            format!("{:.3}", result.average_recall_at_k),
        );
        context.insert(
            format!("{prefix}.precision_at_k"),
            format!("{:.3}", result.average_precision_at_k),
        );
        context.insert(
            format!("{prefix}.mrr"),
            format!("{:.3}", result.mean_reciprocal_rank),
        );
        context.insert(
            format!("{prefix}.latency_p95_ms"),
            result.latency_p95_ms.to_string(),
        );
    }
    if let Some(regression) = regression {
        context.insert(
            "baseline_compatibility".to_owned(),
            compatibility_label(regression.compatibility.classification).to_owned(),
        );
        if !regression.compatibility.changed_fields.is_empty() {
            context.insert(
                "changed_configuration_fields".to_owned(),
                regression.compatibility.changed_fields.join(", "),
            );
        }
        context.insert(
            "regression_classification".to_owned(),
            classification_label(regression.classification).to_owned(),
        );
        if let Some(baseline_id) = regression.baseline_experiment_id {
            context.insert(
                "baseline_experiment_id".to_owned(),
                baseline_id.0.to_string(),
            );
        }
        context.insert(
            "newly_failed_cases".to_owned(),
            regression.newly_failed_cases.len().to_string(),
        );
        context.insert(
            "recovered_cases".to_owned(),
            regression.recovered_cases.len().to_string(),
        );
    }
    context
}

fn eval_summary(experiment: &RetrievalEvalExperiment) -> String {
    eval_summary_with_regression(experiment, None)
}

fn eval_summary_with_regression(
    experiment: &RetrievalEvalExperiment,
    regression: Option<&RetrievalEvalRegressionComparison>,
) -> String {
    let regression_sentence = regression
        .map(|regression| {
            format!(
                " {} Baseline compatibility: {}.",
                regression.summary,
                compatibility_label(regression.compatibility.classification)
            )
        })
        .unwrap_or_default();
    match experiment.gate.status {
        RetrievalEvalGateStatus::Passed => format!(
            "The evaluation gate passed across {} mode(s). {}{}",
            experiment.modes.len(),
            experiment.comparison.summary,
            regression_sentence
        ),
        RetrievalEvalGateStatus::Failed => format!(
            "The evaluation gate failed with {} diagnosed failure(s). {}{}",
            experiment.failures.len(),
            experiment.gate.reasons.join(" "),
            regression_sentence
        ),
    }
}

fn classification_label(classification: RetrievalEvalRegressionClassification) -> &'static str {
    match classification {
        RetrievalEvalRegressionClassification::Improved => "improved",
        RetrievalEvalRegressionClassification::Regressed => "regressed",
        RetrievalEvalRegressionClassification::Unchanged => "unchanged",
    }
}

fn compatibility_label(classification: RetrievalEvalCompatibilityClassification) -> &'static str {
    match classification {
        RetrievalEvalCompatibilityClassification::Compatible => "compatible",
        RetrievalEvalCompatibilityClassification::PartiallyCompatible => "partially_compatible",
        RetrievalEvalCompatibilityClassification::Incompatible => "incompatible",
        RetrievalEvalCompatibilityClassification::LegacyUnknown => "legacy_unknown",
    }
}

fn eval_failure_code(label: RetrievalEvalFailureLabel) -> &'static str {
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

pub(super) fn gate_status_label(status: RetrievalEvalGateStatus) -> &'static str {
    match status {
        RetrievalEvalGateStatus::Passed => "passed",
        RetrievalEvalGateStatus::Failed => "failed",
    }
}
