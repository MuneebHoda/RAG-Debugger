use std::collections::BTreeMap;

use rag_debugger_core::{
    DebugReport, DebugReportEvidenceRef, DebugReportEvidenceRole, DebugReportFinding,
    DebugReportSeverity, DebugReportSource, FailureLabel, Trace,
};

use super::{
    embedding_readiness_label,
    privacy::{evidence_text, permits_content},
    recommendations::recommendations_for_failure_codes,
    retrieval_mode_label, DebugReportBuildContext, ReportBuildError,
};

pub fn build_trace_debug_report(
    context: DebugReportBuildContext,
    trace: &Trace,
) -> Result<DebugReport, ReportBuildError> {
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
    let evidence_labels = evidence
        .iter()
        .map(|evidence| evidence.label.clone())
        .collect::<Vec<_>>();
    let mut findings = trace
        .failure_labels
        .iter()
        .map(|label| trace_finding(label, &evidence_labels))
        .collect::<Vec<_>>();
    let mut failure_codes = trace
        .failure_labels
        .iter()
        .map(trace_failure_code)
        .map(str::to_owned)
        .collect::<Vec<_>>();

    if !retrieval.hits.is_empty() && retrieval.answer.citations.is_empty() {
        failure_codes.push("missing_citations".to_owned());
        findings.push(DebugReportFinding {
            code: "missing_citations".to_owned(),
            severity: DebugReportSeverity::Critical,
            title: "Retrieved evidence was not cited".to_owned(),
            summary: "The retrieval run returned evidence but the evidence summary contained no citations."
                .to_owned(),
            failure_labels: vec!["missing_citations".to_owned()],
            evidence_refs: evidence_labels.clone(),
        });
    }

    if let Some(rerun) = trace.reruns.last() {
        findings.push(DebugReportFinding {
            code: "rerun_comparison".to_owned(),
            severity: DebugReportSeverity::Info,
            title: "Latest rerun changed retrieval behavior".to_owned(),
            summary: format!(
                "Top score changed by {:+.3}, latency by {:+} ms, with {} overlapping chunks and {} rank changes.",
                rerun.score_delta,
                rerun.latency_delta_ms,
                rerun.overlap_count,
                rerun.changed_rank_count
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
        executive_summary: trace.summary.clone(),
        context: trace_context(trace),
        findings,
        recommendations: recommendations_for_failure_codes(&failure_codes),
        evidence,
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

fn trace_finding(label: &FailureLabel, evidence_refs: &[String]) -> DebugReportFinding {
    let code = trace_failure_code(label);
    let (severity, title, summary) = match label {
        FailureLabel::MissingDocument => (
            DebugReportSeverity::Critical,
            "No evidence was retrieved",
            "The query produced no ranked document evidence.",
        ),
        FailureLabel::MissingEmbeddingIndex | FailureLabel::BadEmbedding => (
            DebugReportSeverity::Critical,
            "Embedding coverage is incomplete",
            "Vector retrieval could not rely on a complete embedding index.",
        ),
        FailureLabel::DuplicateEvidence
        | FailureLabel::HeadingOnlyEvidence
        | FailureLabel::BadChunking => (
            DebugReportSeverity::Warning,
            "Chunk quality weakened the result",
            "Duplicate, heading-only, or poorly bounded chunks affected ranked evidence.",
        ),
        FailureLabel::BadRanking => (
            DebugReportSeverity::Warning,
            "Weak evidence ranked too highly",
            "The ranking stage promoted evidence marked as weak.",
        ),
        FailureLabel::WeakEvidence => (
            DebugReportSeverity::Warning,
            "Evidence was insufficient",
            "The evidence summary could not support a defensible answer.",
        ),
        FailureLabel::HallucinatedAnswer => (
            DebugReportSeverity::Critical,
            "Answer grounding failed",
            "The answer was not supported by retrieved evidence.",
        ),
        FailureLabel::BadPrompt | FailureLabel::UnsupportedQuestion => (
            DebugReportSeverity::Warning,
            "The query contract needs review",
            "The query or prompt did not align with the available evidence.",
        ),
    };
    DebugReportFinding {
        code: code.to_owned(),
        severity,
        title: title.to_owned(),
        summary: summary.to_owned(),
        failure_labels: vec![code.to_owned()],
        evidence_refs: evidence_refs.to_vec(),
    }
}

fn trace_failure_code(label: &FailureLabel) -> &'static str {
    match label {
        FailureLabel::MissingDocument => "missing_document",
        FailureLabel::BadChunking => "bad_chunking",
        FailureLabel::BadEmbedding => "bad_embedding",
        FailureLabel::BadRanking => "bad_ranking",
        FailureLabel::BadPrompt => "bad_prompt",
        FailureLabel::UnsupportedQuestion => "unsupported_question",
        FailureLabel::HallucinatedAnswer => "hallucinated_answer",
        FailureLabel::WeakEvidence => "weak_evidence",
        FailureLabel::MissingEmbeddingIndex => "missing_embedding_index",
        FailureLabel::DuplicateEvidence => "duplicate_evidence",
        FailureLabel::HeadingOnlyEvidence => "heading_only_evidence",
    }
}
