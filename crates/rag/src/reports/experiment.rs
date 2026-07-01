use std::collections::BTreeMap;

use rag_debugger_core::{
    DebugReport, DebugReportFinding, DebugReportSeverity, DebugReportSource,
    DiagnosisRecommendation, EvidenceDiagnosisSummary, RetrievalEvalExperiment,
    RetrievalEvalFailureLabel, RetrievalEvalFailureSeverity, RetrievalEvalGateStatus,
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
        executive_summary: eval_summary(experiment),
        context: experiment_context(experiment),
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

fn experiment_context(experiment: &RetrievalEvalExperiment) -> BTreeMap<String, String> {
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
    context
}

fn eval_summary(experiment: &RetrievalEvalExperiment) -> String {
    match experiment.gate.status {
        RetrievalEvalGateStatus::Passed => format!(
            "The evaluation gate passed across {} mode(s). {}",
            experiment.modes.len(),
            experiment.comparison.summary
        ),
        RetrievalEvalGateStatus::Failed => format!(
            "The evaluation gate failed with {} diagnosed failure(s). {}",
            experiment.failures.len(),
            experiment.gate.reasons.join(" ")
        ),
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
