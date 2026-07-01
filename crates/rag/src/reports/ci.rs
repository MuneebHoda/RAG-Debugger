use rag_debugger_core::{
    CiEvalRun, DebugReport, DebugReportFinding, DebugReportRecommendation,
    DebugReportRecommendationArea, DebugReportRecommendationPriority, DebugReportSeverity,
    DebugReportSource,
};

use super::{
    experiment::{build_eval_experiment_debug_report, gate_status_label},
    privacy::permits_content,
    DebugReportBuildContext,
};

pub fn build_ci_eval_debug_report(
    context: DebugReportBuildContext,
    run: &CiEvalRun,
) -> DebugReport {
    let mut report = build_eval_experiment_debug_report(context, &run.report.experiment);
    report.title = "RAG CI gate audit".to_owned();
    report.subject = if permits_content(context.privacy_mode) {
        run.branch
            .clone()
            .unwrap_or_else(|| run.config_label.clone())
    } else {
        format!("CI eval run {}", run.id.0)
    };
    report.source = DebugReportSource::CiEvalRun { run_id: run.id };
    report
        .context
        .insert("ci_config_label".to_owned(), run.config_label.clone());
    insert_optional(&mut report.context, "ci_branch", run.branch.as_deref());
    insert_optional(
        &mut report.context,
        "ci_commit_sha",
        run.commit_sha.as_deref(),
    );
    report.context.insert(
        "ci_gate_status".to_owned(),
        gate_status_label(run.gate_status).to_owned(),
    );

    if let Some(regression) = &run.regression {
        report.context.insert(
            "ci_recall_delta".to_owned(),
            format!("{:.3}", regression.recall_delta),
        );
        report.context.insert(
            "ci_precision_delta".to_owned(),
            format!("{:.3}", regression.precision_delta),
        );
        report.context.insert(
            "ci_mrr_delta".to_owned(),
            format!("{:.3}", regression.mrr_delta),
        );
        report.context.insert(
            "ci_latency_delta_ms".to_owned(),
            regression.latency_delta_ms.to_string(),
        );
        report.context.insert(
            "ci_newly_failed_case_count".to_owned(),
            regression.newly_failed_case_count.to_string(),
        );
        report.findings.push(DebugReportFinding {
            code: "ci_regression".to_owned(),
            severity: if regression.newly_failed_case_count > 0 {
                DebugReportSeverity::Critical
            } else {
                DebugReportSeverity::Info
            },
            title: "CI regression comparison".to_owned(),
            summary: regression.summary.clone(),
            failure_labels: Vec::new(),
            evidence_refs: Vec::new(),
        });
        if regression.newly_failed_case_count > 0
            && !report
                .recommendations
                .iter()
                .any(|recommendation| recommendation.code == "review_ci_regression")
        {
            report.recommendations.push(DebugReportRecommendation {
                code: "review_ci_regression".to_owned(),
                priority: DebugReportRecommendationPriority::Critical,
                area: DebugReportRecommendationArea::RetrievalMode,
                title: "Review newly failing CI cases".to_owned(),
                rationale: "The current configuration introduced new retrieval failures."
                    .to_owned(),
                action: "Compare the baseline and head configuration, then block release until the new failures are resolved."
                    .to_owned(),
                finding_codes: vec!["ci_regression".to_owned()],
                evidence_refs: Vec::new(),
            });
        }
    }
    report
}

fn insert_optional(
    context: &mut std::collections::BTreeMap<String, String>,
    key: &str,
    value: Option<&str>,
) {
    if let Some(value) = value {
        context.insert(key.to_owned(), value.to_owned());
    }
}
