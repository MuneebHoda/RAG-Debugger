use std::collections::BTreeMap;

use rag_debugger_core::{
    DebugReport, DebugReportEvidenceRef, DebugReportEvidenceRole, DebugReportFinding,
    DebugReportSeverity, DebugReportSource, DebuggerConfig, DiagnosisFailure, DiagnosisSeverity,
    RetrievalConfig, Trace,
};

use crate::diagnosis::diagnose_retrieval;

use super::{
    embedding_readiness_label,
    privacy::{evidence_text, permits_content},
    recommendations::debug_report_recommendations,
    retrieval_mode_label, DebugReportBuildContext, ReportBuildError,
};

pub fn build_trace_debug_report(
    context: DebugReportBuildContext,
    trace: &Trace,
) -> Result<DebugReport, ReportBuildError> {
    let trace = crate::tracing::ensure_trace_diagnosis(
        trace.clone(),
        &RetrievalConfig::default(),
        &DebuggerConfig::default(),
    );
    let trace = &trace;
    let retrieval = trace
        .retrieval
        .as_ref()
        .ok_or(ReportBuildError::InvalidSource(
            "trace does not contain a retrieval response",
        ))?;
    let evidence = retrieval
        .hits
        .iter()
        .enumerate()
        .map(|(index, hit)| DebugReportEvidenceRef {
            label: format!("E{}", index + 1),
            role: DebugReportEvidenceRole::Retrieved,
            source_id: Some(hit.source.id),
            document_id: Some(hit.document.id),
            chunk_id: Some(hit.chunk.id),
            rank: Some(hit.rank),
            document_path: permits_content(context.privacy_mode).then(|| hit.document.path.clone()),
            section_title: permits_content(context.privacy_mode)
                .then(|| hit.chunk.section_title.clone())
                .flatten(),
            checksum_prefix: Some(hit.citation.checksum_prefix.clone()),
            citation_label: Some(hit.citation.label.clone()),
            snippet: evidence_text(context.privacy_mode, &hit.snippet, &hit.chunk.text),
            evidence_strength: Some(hit.evidence_strength),
            chunk_quality_flags: hit.chunk.quality_flags.clone(),
            retrieval_quality_flags: hit.quality_flags.clone(),
        })
        .collect::<Vec<_>>();
    let diagnosis = trace
        .diagnosis
        .clone()
        .or_else(|| retrieval.diagnosis.clone())
        .unwrap_or_else(|| diagnose_retrieval(retrieval, &DebuggerConfig::default(), None));
    let evidence_labels = evidence
        .iter()
        .map(|evidence| evidence.label.clone())
        .collect::<Vec<_>>();
    let mut findings = diagnosis
        .failures
        .iter()
        .map(diagnosis_finding)
        .collect::<Vec<_>>();

    if let Some(rerun) = trace.reruns.last() {
        findings.push(DebugReportFinding {
            code: "rerun_comparison".to_owned(),
            severity: DebugReportSeverity::Info,
            title: "Latest rerun changed retrieval behavior".to_owned(),
            summary: rerun.diagnosis.as_ref().map_or_else(
                || format!(
                    "Top score changed by {:+.3}, latency by {:+} ms, with {} overlapping chunks and {} rank changes.",
                    rerun.score_delta,
                    rerun.latency_delta_ms,
                    rerun.overlap_count,
                    rerun.changed_rank_count
                ),
                |comparison| comparison.summary.clone(),
            ),
            failure_labels: Vec::new(),
            evidence_refs: evidence_labels.clone(),
        });
    }

    if findings.is_empty() {
        findings.push(DebugReportFinding {
            code: "no_detected_failures".to_owned(),
            severity: DebugReportSeverity::Info,
            title: "No deterministic failure labels were detected".to_owned(),
            summary: "The saved trace contains no current CorpusLab failure labels.".to_owned(),
            failure_labels: Vec::new(),
            evidence_refs: evidence_labels,
        });
    }

    Ok(DebugReport {
        id: context.report_id,
        workspace_id: context.workspace_id,
        project_id: context.project_id,
        title: "RAG trace audit".to_owned(),
        subject: if permits_content(context.privacy_mode) {
            trace.input.clone()
        } else {
            format!("Trace {}", trace.id.0)
        },
        source: DebugReportSource::Trace { trace_id: trace.id },
        privacy_mode: context.privacy_mode,
        executive_summary: diagnosis.summary.clone(),
        context: trace_context(trace),
        findings,
        recommendations: debug_report_recommendations(&diagnosis.recommendations),
        evidence,
        diagnosis: Some(diagnosis),
        created_at: context.created_at,
    })
}

fn trace_context(trace: &Trace) -> BTreeMap<String, String> {
    let mut context = BTreeMap::new();
    if let Some(retrieval) = &trace.retrieval {
        context.insert(
            "embedding_model".to_owned(),
            retrieval.embedding_status.model.model_name.clone(),
        );
        context.insert(
            "embedding_readiness".to_owned(),
            embedding_readiness_label(retrieval.embedding_status.readiness).to_owned(),
        );
        context.insert("hit_count".to_owned(), retrieval.hits.len().to_string());
        context.insert(
            "latency_ms".to_owned(),
            retrieval.run.latency_ms.to_string(),
        );
        context.insert(
            "retrieval_mode".to_owned(),
            retrieval_mode_label(retrieval.run.retrieval_mode).to_owned(),
        );
        context.insert("top_k".to_owned(), retrieval.run.top_k.to_string());
    }
    context.insert("rerun_count".to_owned(), trace.reruns.len().to_string());
    if let Some(rerun) = trace.reruns.last() {
        context.insert(
            "latest_rerun_changed_rank_count".to_owned(),
            rerun.changed_rank_count.to_string(),
        );
        context.insert(
            "latest_rerun_latency_delta_ms".to_owned(),
            rerun.latency_delta_ms.to_string(),
        );
        context.insert(
            "latest_rerun_mode".to_owned(),
            retrieval_mode_label(rerun.response.run.retrieval_mode).to_owned(),
        );
        context.insert(
            "latest_rerun_overlap_count".to_owned(),
            rerun.overlap_count.to_string(),
        );
        context.insert(
            "latest_rerun_score_delta".to_owned(),
            format!("{:.3}", rerun.score_delta),
        );
    }
    context
}

fn diagnosis_finding(failure: &DiagnosisFailure) -> DebugReportFinding {
    let code = failure.code.as_str();
    DebugReportFinding {
        code: code.to_owned(),
        severity: match failure.severity {
            DiagnosisSeverity::Info => DebugReportSeverity::Info,
            DiagnosisSeverity::Warning => DebugReportSeverity::Warning,
            DiagnosisSeverity::Critical => DebugReportSeverity::Critical,
        },
        title: failure.title.clone(),
        summary: failure.summary.clone(),
        failure_labels: vec![code.to_owned()],
        evidence_refs: failure.evidence_refs.clone(),
    }
}
