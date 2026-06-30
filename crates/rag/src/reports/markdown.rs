use std::fmt::Write;

use rag_debugger_core::{
    DebugReport, DebugReportEvidenceRole, DebugReportPrivacyMode, DebugReportRecommendationPriority,
};
use thiserror::Error;

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
    writeln!(output, "# {}", single_line(&report.title)).expect("writing to String cannot fail");
    writeln!(output, "\n**Subject:** {}", single_line(&report.subject))
        .expect("writing to String cannot fail");
    writeln!(output, "\n**Privacy:** {:?}", report.privacy_mode)
        .expect("writing to String cannot fail");
    writeln!(output, "\n## Executive Summary\n").expect("writing to String cannot fail");
    writeln!(output, "{}", single_line(&report.executive_summary))
        .expect("writing to String cannot fail");

    writeln!(output, "\n## Configuration\n").expect("writing to String cannot fail");
    for (key, value) in &report.context {
        writeln!(output, "- **{}:** {}", single_line(key), single_line(value))
            .expect("writing to String cannot fail");
    }

    writeln!(output, "\n## Findings\n").expect("writing to String cannot fail");
    for finding in &report.findings {
        writeln!(
            output,
            "### {} [{}]\n\n{}\n",
            single_line(&finding.title),
            single_line(&finding.code),
            single_line(&finding.summary)
        )
        .expect("writing to String cannot fail");
    }

    writeln!(output, "## Evidence\n").expect("writing to String cannot fail");
    for evidence in &report.evidence {
        write!(
            output,
            "- **{}** ({})",
            single_line(&evidence.label),
            evidence_role_label(evidence.role)
        )
        .expect("writing to String cannot fail");
        if let Some(chunk_id) = evidence.chunk_id {
            write!(output, " chunk `{}`", chunk_id.0).expect("writing to String cannot fail");
        } else if let Some(document_id) = evidence.document_id {
            write!(output, " document `{}`", document_id.0).expect("writing to String cannot fail");
        }
        if report.privacy_mode == DebugReportPrivacyMode::SnippetsAllowed {
            if let Some(snippet) = &evidence.snippet {
                write!(output, ": {}", single_line(snippet))
                    .expect("writing to String cannot fail");
            }
        }
        writeln!(output).expect("writing to String cannot fail");
    }

    writeln!(output, "\n## Recommendations\n").expect("writing to String cannot fail");
    for recommendation in &report.recommendations {
        writeln!(
            output,
            "### {} ({})\n\n{}\n\n**Action:** {}\n",
            single_line(&recommendation.title),
            priority_label(recommendation.priority),
            single_line(&recommendation.rationale),
            single_line(&recommendation.action)
        )
        .expect("writing to String cannot fail");
    }

    writeln!(
        output,
        "## Privacy Note\n\nThis report was exported in `{:?}` mode. Original uploaded files and embedding vectors are not included.",
        report.privacy_mode
    )
    .expect("writing to String cannot fail");
    Ok(output)
}

fn single_line(value: &str) -> String {
    value
        .replace(['\r', '\n'], " ")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn evidence_role_label(role: DebugReportEvidenceRole) -> &'static str {
    match role {
        DebugReportEvidenceRole::Retrieved => "retrieved",
        DebugReportEvidenceRole::Expected => "expected",
        DebugReportEvidenceRole::Missing => "missing",
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
