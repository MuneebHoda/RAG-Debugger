use std::{collections::HashSet, fmt::Write};

use rag_debugger_core::{
    ChunkQualityFlag, DebugReport, DebugReportEvidenceRef, DebugReportEvidenceRole,
    DebugReportPrivacyMode, DebugReportRecommendationArea, DebugReportRecommendationPriority,
    DebugReportSeverity, DebugReportSource, DiagnosisSeverity, EvidenceStrength,
    RetrievalQualityFlag,
};
use thiserror::Error;
use time::format_description::well_known::Rfc3339;

use super::privacy::bounded_snippet;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ReportExportError {
    #[error("full-local-only reports must be redacted before export")]
    FullLocalOnly,
}

pub fn render_debug_report_markdown(report: &DebugReport) -> Result<String, ReportExportError> {
    if report.privacy_mode == DebugReportPrivacyMode::FullLocalOnly {
        return Err(ReportExportError::FullLocalOnly);
    }

    let mut output = String::new();
    write_title(&mut output, report);
    write_executive_summary(&mut output, report);
    if report.diagnosis.is_some() {
        write_deterministic_diagnosis(&mut output, report);
    }
    write_source_and_privacy(&mut output, report);
    write_configuration(&mut output, report);
    write_failing_cases(&mut output, report);
    write_evidence_diagnosis(&mut output, report);
    write_failure_labels(&mut output, report);
    write_changes(&mut output, report);
    write_recommendations(&mut output, report);
    write_privacy_note(&mut output, report.privacy_mode);
    Ok(output)
}

fn write_deterministic_diagnosis(output: &mut String, report: &DebugReport) {
    section(output, "Deterministic Diagnosis");
    let Some(diagnosis) = &report.diagnosis else {
        writeln!(output, "No structured diagnosis snapshot was recorded.\n")
            .expect("String writes cannot fail");
        return;
    };

    table_header(output, "Diagnosis field", "Value");
    table_row(output, "Outcome", diagnosis.outcome.as_str());
    if let Some(primary) = &diagnosis.primary_issue {
        table_row(output, "Primary issue", primary.code.as_str());
        table_row(
            output,
            "Severity",
            diagnosis_severity_label(primary.severity),
        );
    }
    writeln!(output).expect("String writes cannot fail");
    writeln!(output, "{}\n", escape_markdown(&diagnosis.summary))
        .expect("String writes cannot fail");

    if !diagnosis.score_explanations.is_empty() {
        writeln!(output, "### Ranking explanations\n").expect("String writes cannot fail");
        table_header(output, "Evidence", "Explanation");
        for explanation in &diagnosis.score_explanations {
            table_row(
                output,
                &explanation.evidence_ref,
                &format!(
                    "rank {} · score {:.3} · {}",
                    explanation.rank,
                    explanation.final_score,
                    explanation.dominant_signal.label()
                ),
            );
        }
        writeln!(output).expect("String writes cannot fail");
    }
}

fn write_title(output: &mut String, report: &DebugReport) {
    writeln!(output, "# {}\n", escape_markdown(&report.title)).expect("String writes cannot fail");
    writeln!(output, "CorpusLab RAG Audit Report\n").expect("String writes cannot fail");
}

fn write_executive_summary(output: &mut String, report: &DebugReport) {
    section(output, "Executive Summary");
    writeln!(output, "{}\n", escape_markdown(&report.executive_summary))
        .expect("String writes cannot fail");
}

fn write_source_and_privacy(output: &mut String, report: &DebugReport) {
    section(output, "Report Source and Privacy Classification");
    let (source_type, source_id) = source_identity(&report.source, report.privacy_mode);
    table_header(output, "Field", "Value");
    table_row(output, "Report ID", &report.id.0.to_string());
    table_row(output, "Source type", source_type);
    table_row(output, "Source reference", &source_id);
    table_row(output, "Privacy mode", privacy_label(report.privacy_mode));
    table_row(
        output,
        "Created at",
        &report
            .created_at
            .format(&Rfc3339)
            .expect("OffsetDateTime must format as RFC3339"),
    );
    writeln!(output).expect("String writes cannot fail");
}

fn write_configuration(output: &mut String, report: &DebugReport) {
    section(output, "System and Configuration Snapshot");
    let context = report
        .context
        .iter()
        .filter(|(key, _)| context_key_is_exportable(key, report.privacy_mode))
        .collect::<Vec<_>>();
    if context.is_empty() {
        writeln!(output, "No configuration metadata was recorded.\n")
            .expect("String writes cannot fail");
        return;
    }

    table_header(output, "Configuration", "Value");
    for (key, value) in context {
        table_row(output, &humanize(key), value);
    }
    writeln!(output).expect("String writes cannot fail");
}

fn write_failing_cases(output: &mut String, report: &DebugReport) {
    section(output, "Failing Queries or Cases");
    if report.privacy_mode == DebugReportPrivacyMode::SnippetsAllowed {
        writeln!(
            output,
            "**Report subject:** {}\n",
            escape_markdown(&report.subject)
        )
        .expect("String writes cannot fail");
    } else {
        writeln!(
            output,
            "Query and case content is omitted by the `metadata_only` privacy policy.\n"
        )
        .expect("String writes cannot fail");
    }

    if report.findings.is_empty() {
        writeln!(output, "No deterministic failure findings were recorded.\n")
            .expect("String writes cannot fail");
        return;
    }

    for (index, finding) in report.findings.iter().enumerate() {
        writeln!(
            output,
            "### {}. {}\n",
            index + 1,
            escape_markdown(&finding.title)
        )
        .expect("String writes cannot fail");
        writeln!(
            output,
            "- **Finding code:** `{}`",
            escape_code(&finding.code)
        )
        .expect("String writes cannot fail");
        writeln!(
            output,
            "- **Severity:** {}",
            severity_label(finding.severity)
        )
        .expect("String writes cannot fail");
        if !finding.evidence_refs.is_empty() {
            writeln!(
                output,
                "- **Evidence references:** {}",
                escaped_join(&finding.evidence_refs)
            )
            .expect("String writes cannot fail");
        }
        writeln!(output, "\n{}\n", escape_markdown(&finding.summary))
            .expect("String writes cannot fail");
    }
}

fn write_evidence_diagnosis(output: &mut String, report: &DebugReport) {
    section(output, "Evidence Diagnosis");
    if report.evidence.is_empty() {
        writeln!(
            output,
            "No evidence references were available for this report.\n"
        )
        .expect("String writes cannot fail");
        return;
    }

    for evidence in &report.evidence {
        write_evidence(output, report.privacy_mode, evidence);
    }
}

fn write_evidence(
    output: &mut String,
    privacy_mode: DebugReportPrivacyMode,
    evidence: &DebugReportEvidenceRef,
) {
    writeln!(
        output,
        "### {} - {}\n",
        escape_markdown(&evidence.label),
        evidence_role_label(evidence.role)
    )
    .expect("String writes cannot fail");
    table_header(output, "Evidence field", "Value");
    table_row(output, "Role", evidence_role_label(evidence.role));
    optional_table_row(output, "Rank", evidence.rank.map(|rank| rank.to_string()));
    optional_table_row(
        output,
        "Source ID",
        evidence.source_id.map(|id| id.0.to_string()),
    );
    optional_table_row(
        output,
        "Document ID",
        evidence.document_id.map(|id| id.0.to_string()),
    );
    optional_table_row(
        output,
        "Chunk ID",
        evidence.chunk_id.map(|id| id.0.to_string()),
    );
    optional_table_row(output, "Checksum", evidence.checksum_prefix.clone());
    optional_table_row(output, "Citation", evidence.citation_label.clone());
    optional_table_row(
        output,
        "Evidence strength",
        evidence
            .evidence_strength
            .map(evidence_strength_label)
            .map(str::to_owned),
    );
    if privacy_mode == DebugReportPrivacyMode::SnippetsAllowed {
        optional_table_row(output, "Document path", evidence.document_path.clone());
        optional_table_row(output, "Section", evidence.section_title.clone());
    }
    writeln!(output).expect("String writes cannot fail");

    let quality_flags = evidence
        .chunk_quality_flags
        .iter()
        .map(|flag| chunk_quality_label(*flag))
        .chain(
            evidence
                .retrieval_quality_flags
                .iter()
                .map(|flag| retrieval_quality_label(*flag)),
        )
        .collect::<Vec<_>>();
    if !quality_flags.is_empty() {
        writeln!(
            output,
            "**Quality signals:** {}\n",
            quality_flags.join(", ")
        )
        .expect("String writes cannot fail");
    }

    if privacy_mode == DebugReportPrivacyMode::SnippetsAllowed {
        if let Some(snippet) = &evidence.snippet {
            writeln!(
                output,
                "**Approved snippet:** {}\n",
                escape_markdown(&bounded_snippet(snippet))
            )
            .expect("String writes cannot fail");
        }
    }
}

fn write_failure_labels(output: &mut String, report: &DebugReport) {
    section(output, "Failure Labels");
    let mut seen = HashSet::new();
    let labels = report
        .findings
        .iter()
        .flat_map(|finding| finding.failure_labels.iter())
        .filter(|label| seen.insert(label.as_str()))
        .collect::<Vec<_>>();
    if labels.is_empty() {
        writeln!(output, "No failure labels were recorded.\n").expect("String writes cannot fail");
        return;
    }
    for label in labels {
        writeln!(output, "- `{}`", escape_code(label)).expect("String writes cannot fail");
    }
    writeln!(output).expect("String writes cannot fail");
}

fn write_changes(output: &mut String, report: &DebugReport) {
    section(output, "Rerun, Experiment, and Regression Changes");
    let change_findings = report
        .findings
        .iter()
        .filter(|finding| {
            let code = finding.code.to_ascii_lowercase();
            code.contains("rerun") || code.contains("comparison") || code.contains("regression")
        })
        .collect::<Vec<_>>();
    let change_context = report
        .context
        .iter()
        .filter(|(key, _)| is_change_key(key))
        .collect::<Vec<_>>();

    if change_findings.is_empty() && change_context.is_empty() {
        writeln!(
            output,
            "No rerun, mode-comparison, or regression change was recorded.\n"
        )
        .expect("String writes cannot fail");
        return;
    }

    for finding in change_findings {
        writeln!(
            output,
            "- **{}:** {}",
            escape_markdown(&finding.title),
            escape_markdown(&finding.summary)
        )
        .expect("String writes cannot fail");
    }
    if !change_context.is_empty() {
        writeln!(output).expect("String writes cannot fail");
        table_header(output, "Change signal", "Value");
        for (key, value) in change_context {
            table_row(output, &humanize(key), value);
        }
    }
    writeln!(output).expect("String writes cannot fail");
}

fn write_recommendations(output: &mut String, report: &DebugReport) {
    section(output, "Prioritized Recommendations");
    if report.recommendations.is_empty() {
        writeln!(output, "No remediation recommendation was generated.\n")
            .expect("String writes cannot fail");
        return;
    }

    for (index, recommendation) in report.recommendations.iter().enumerate() {
        writeln!(
            output,
            "### {}. {}\n",
            index + 1,
            escape_markdown(&recommendation.title)
        )
        .expect("String writes cannot fail");
        writeln!(
            output,
            "- **Priority:** {}",
            priority_label(recommendation.priority)
        )
        .expect("String writes cannot fail");
        writeln!(
            output,
            "- **Area:** {}",
            recommendation_area_label(recommendation.area)
        )
        .expect("String writes cannot fail");
        writeln!(
            output,
            "- **Recommendation code:** `{}`",
            escape_code(&recommendation.code)
        )
        .expect("String writes cannot fail");
        if !recommendation.finding_codes.is_empty() {
            writeln!(
                output,
                "- **Related findings:** {}",
                escaped_join(&recommendation.finding_codes)
            )
            .expect("String writes cannot fail");
        }
        if !recommendation.evidence_refs.is_empty() {
            writeln!(
                output,
                "- **Affected evidence:** {}",
                escaped_join(&recommendation.evidence_refs)
            )
            .expect("String writes cannot fail");
        }
        writeln!(
            output,
            "\n**Rationale:** {}\n\n**Recommended action:** {}\n",
            escape_markdown(&recommendation.rationale),
            escape_markdown(&recommendation.action)
        )
        .expect("String writes cannot fail");
    }
}

fn write_privacy_note(output: &mut String, mode: DebugReportPrivacyMode) {
    section(output, "Privacy and Sharing Note");
    let note = match mode {
        DebugReportPrivacyMode::MetadataOnly => {
            "This export is classified `metadata_only`. Query text, document paths, section titles, and evidence snippets are omitted. Review identifiers and operational metadata before sharing outside the workspace."
        }
        DebugReportPrivacyMode::SnippetsAllowed => {
            "This export is classified `snippets_allowed`. It may contain explicitly approved query or case text, document labels, section titles, and evidence snippets capped at 280 characters. Review every included snippet before sharing."
        }
        DebugReportPrivacyMode::FullLocalOnly => unreachable!("full-local reports are rejected"),
    };
    writeln!(output, "{note}\n").expect("String writes cannot fail");
    writeln!(
        output,
        "Original uploaded binaries, complete documents, embedding vectors, credentials, and session data are not included."
    )
    .expect("String writes cannot fail");
}

fn section(output: &mut String, title: &str) {
    writeln!(output, "## {title}\n").expect("String writes cannot fail");
}

fn table_header(output: &mut String, first: &str, second: &str) {
    writeln!(output, "| {first} | {second} |").expect("String writes cannot fail");
    writeln!(output, "| --- | --- |").expect("String writes cannot fail");
}

fn table_row(output: &mut String, key: &str, value: &str) {
    writeln!(
        output,
        "| {} | {} |",
        escape_markdown(key),
        escape_markdown(value)
    )
    .expect("String writes cannot fail");
}

fn optional_table_row(output: &mut String, key: &str, value: Option<String>) {
    if let Some(value) = value {
        table_row(output, key, &value);
    }
}

fn source_identity(
    source: &DebugReportSource,
    privacy_mode: DebugReportPrivacyMode,
) -> (&'static str, String) {
    match source {
        DebugReportSource::Trace { trace_id } => ("trace", trace_id.0.to_string()),
        DebugReportSource::EvalExperiment { experiment_id } => {
            ("eval experiment", experiment_id.0.to_string())
        }
        DebugReportSource::CiEvalRun { run_id } => ("CI eval run", run_id.0.to_string()),
        DebugReportSource::Manual { label } => (
            "manual investigation",
            if privacy_mode == DebugReportPrivacyMode::SnippetsAllowed {
                label.clone()
            } else {
                "redacted manual source".to_owned()
            },
        ),
    }
}

fn context_key_is_exportable(key: &str, privacy_mode: DebugReportPrivacyMode) -> bool {
    if privacy_mode == DebugReportPrivacyMode::SnippetsAllowed {
        return true;
    }
    let key = key.to_ascii_lowercase();
    ![
        "query", "prompt", "answer", "snippet", "text", "path", "section",
    ]
    .iter()
    .any(|sensitive| key.contains(sensitive))
}

fn is_change_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.starts_with("latest_rerun_")
        || key.contains("_delta")
        || key.contains("newly_failed")
        || key == "best_retrieval_mode"
        || key == "gate_status"
        || key == "ci_gate_status"
}

fn escape_markdown(value: &str) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut escaped = String::with_capacity(normalized.len());
    for character in normalized.chars() {
        match character {
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '(' | ')' | '#' | '+' | '!' | '|'
            | '~' => {
                escaped.push('\\');
                escaped.push(character);
            }
            _ => escaped.push(character),
        }
    }
    escaped
}

fn escape_code(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('`', "\\`")
}

fn escaped_join(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("`{}`", escape_code(value)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn humanize(value: &str) -> String {
    value.replace(['_', '.'], " ")
}

fn privacy_label(mode: DebugReportPrivacyMode) -> &'static str {
    match mode {
        DebugReportPrivacyMode::MetadataOnly => "metadata_only",
        DebugReportPrivacyMode::SnippetsAllowed => "snippets_allowed",
        DebugReportPrivacyMode::FullLocalOnly => "full_local_only",
    }
}

fn severity_label(severity: DebugReportSeverity) -> &'static str {
    match severity {
        DebugReportSeverity::Info => "info",
        DebugReportSeverity::Warning => "warning",
        DebugReportSeverity::Critical => "critical",
    }
}

fn diagnosis_severity_label(severity: DiagnosisSeverity) -> &'static str {
    match severity {
        DiagnosisSeverity::Info => "info",
        DiagnosisSeverity::Warning => "warning",
        DiagnosisSeverity::Critical => "critical",
    }
}

fn evidence_role_label(role: DebugReportEvidenceRole) -> &'static str {
    match role {
        DebugReportEvidenceRole::Retrieved => "retrieved",
        DebugReportEvidenceRole::Expected => "expected",
        DebugReportEvidenceRole::Missing => "missing",
    }
}

fn evidence_strength_label(strength: EvidenceStrength) -> &'static str {
    match strength {
        EvidenceStrength::Strong => "strong",
        EvidenceStrength::Medium => "medium",
        EvidenceStrength::Weak => "weak",
    }
}

fn priority_label(priority: DebugReportRecommendationPriority) -> &'static str {
    match priority {
        DebugReportRecommendationPriority::Critical => "critical",
        DebugReportRecommendationPriority::High => "high",
        DebugReportRecommendationPriority::Medium => "medium",
        DebugReportRecommendationPriority::Low => "low",
    }
}

fn recommendation_area_label(area: DebugReportRecommendationArea) -> &'static str {
    match area {
        DebugReportRecommendationArea::Chunking => "chunking",
        DebugReportRecommendationArea::Embeddings => "embeddings",
        DebugReportRecommendationArea::TopK => "top_k",
        DebugReportRecommendationArea::RetrievalMode => "retrieval_mode",
        DebugReportRecommendationArea::Reranking => "reranking",
        DebugReportRecommendationArea::MetadataFilters => "metadata_filters",
        DebugReportRecommendationArea::Citations => "citations",
        DebugReportRecommendationArea::CorpusCoverage => "corpus_coverage",
        DebugReportRecommendationArea::Other => "other",
    }
}

fn chunk_quality_label(flag: ChunkQualityFlag) -> &'static str {
    match flag {
        ChunkQualityFlag::HeadingOnly => "heading_only",
        ChunkQualityFlag::TooShort => "too_short",
        ChunkQualityFlag::TooLong => "too_long",
        ChunkQualityFlag::Duplicate => "duplicate",
        ChunkQualityFlag::LowTextDensity => "low_text_density",
        ChunkQualityFlag::ExtractionWarning => "extraction_warning",
        ChunkQualityFlag::GoodEvidenceCandidate => "good_evidence_candidate",
    }
}

fn retrieval_quality_label(flag: RetrievalQualityFlag) -> &'static str {
    match flag {
        RetrievalQualityFlag::Duplicate => "duplicate",
        RetrievalQualityFlag::HeadingOnly => "heading_only",
        RetrievalQualityFlag::TooShort => "too_short",
        RetrievalQualityFlag::WeakEvidence => "weak_evidence",
        RetrievalQualityFlag::SemanticMatch => "semantic_match",
        RetrievalQualityFlag::ExactTermMatch => "exact_term_match",
        RetrievalQualityFlag::SectionOnlyMatch => "section_only_match",
    }
}
