use std::{fs, path::PathBuf};

use rag_debugger_core::{DebugReport, DebugReportPrivacyMode};
use rag_debugger_rag::reports::{render_debug_report_markdown, ReportExportError};
use serde_json::{json, Value};

#[test]
fn trace_report_matches_professional_markdown_fixture() {
    let report = trace_report(DebugReportPrivacyMode::SnippetsAllowed);
    assert_fixture("trace.md", render(&report));
}

#[test]
fn eval_report_matches_professional_markdown_fixture() {
    assert_fixture("eval.md", render(&eval_report()));
}

#[test]
fn ci_report_matches_professional_markdown_fixture() {
    assert_fixture("ci.md", render(&ci_report()));
}

#[test]
fn metadata_only_fixture_excludes_content_fields() {
    let report = metadata_only_report();
    let markdown = render(&report);

    assert!(!markdown.contains("SECRET"));
    assert!(!markdown.contains("private/document.md"));
    assert_fixture("metadata-only.md", markdown);
}

#[test]
fn snippets_allowed_fixture_escapes_markdown_and_caps_snippets() {
    let report = snippets_allowed_report();
    let markdown = render(&report);

    assert!(!markdown.contains("<script>"));
    assert!(markdown.contains("&lt;script&gt;"));
    assert!(!markdown.contains(&"x".repeat(280)));
    assert_fixture("snippets-allowed.md", markdown);
}

#[test]
fn full_local_only_reports_cannot_be_rendered() {
    let mut report = trace_report(DebugReportPrivacyMode::SnippetsAllowed);
    report.privacy_mode = DebugReportPrivacyMode::FullLocalOnly;

    assert_eq!(
        render_debug_report_markdown(&report),
        Err(ReportExportError::FullLocalOnly)
    );
}

fn render(report: &DebugReport) -> String {
    render_debug_report_markdown(report).expect("report should be exportable")
}

fn assert_fixture(name: &str, actual: String) {
    let path = fixture_path(name);
    if std::env::var_os("UPDATE_REPORT_FIXTURES").is_some() {
        fs::create_dir_all(path.parent().expect("fixture parent"))
            .expect("create fixture directory");
        fs::write(&path, &actual).expect("write report fixture");
    }
    let expected = fs::read_to_string(&path).expect("read report fixture");
    assert_eq!(actual, expected, "report fixture {name} changed");
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/reports")
        .join(name)
}

fn trace_report(privacy_mode: DebugReportPrivacyMode) -> DebugReport {
    report_from_json(json!({
        "id": "00000000-0000-0000-0000-000000000101",
        "workspace_id": "00000000-0000-0000-0000-000000000102",
        "project_id": "00000000-0000-0000-0000-000000000103",
        "title": "RAG trace audit",
        "subject": "When is the GPU index published?",
        "source": { "type": "trace", "trace_id": "00000000-0000-0000-0000-000000000104" },
        "privacy_mode": privacy_label(privacy_mode),
        "executive_summary": "Duplicate evidence weakened the result and the rerun improved latency.",
        "context": {
            "embedding_model": "local-hash-v1",
            "embedding_readiness": "ready",
            "hit_count": "1",
            "latency_ms": "12",
            "latest_rerun_latency_delta_ms": "-4",
            "latest_rerun_mode": "lexical",
            "latest_rerun_overlap_count": "1",
            "latest_rerun_score_delta": "0.125",
            "retrieval_mode": "hybrid",
            "top_k": "5"
        },
        "findings": [
            {
                "code": "duplicate_evidence",
                "severity": "warning",
                "title": "Duplicate evidence weakened the result",
                "summary": "Two equivalent chunks competed in the ranked evidence.",
                "failure_labels": ["duplicate_evidence"],
                "evidence_refs": ["E1"]
            },
            {
                "code": "rerun_comparison",
                "severity": "info",
                "title": "Latest rerun changed retrieval behavior",
                "summary": "Top score changed by +0.125 and latency by -4 ms.",
                "failure_labels": [],
                "evidence_refs": ["E1"]
            }
        ],
        "recommendations": [{
            "code": "deduplicate_chunks",
            "priority": "high",
            "area": "chunking",
            "title": "Remove duplicate chunks before indexing",
            "rationale": "Equivalent chunks can occupy multiple top-k positions.",
            "action": "Deduplicate normalized chunk text and re-index the affected source.",
            "finding_codes": ["duplicate_evidence"]
        }],
        "evidence": [evidence_json("Index publication requires checksum validation.")],
        "diagnosis": diagnosis_json(
            "mixed",
            "duplicate_evidence",
            "warning",
            "Duplicate evidence crowded the ranking",
            &["E1"]
        ),
        "created_at": "2026-06-30T08:15:30Z"
    }))
}

fn eval_report() -> DebugReport {
    report_from_json(json!({
        "id": "00000000-0000-0000-0000-000000000201",
        "workspace_id": "00000000-0000-0000-0000-000000000102",
        "project_id": "00000000-0000-0000-0000-000000000103",
        "title": "RAG evaluation audit",
        "subject": "Release quality dataset",
        "source": { "type": "eval_experiment", "experiment_id": "00000000-0000-0000-0000-000000000204" },
        "privacy_mode": "snippets_allowed",
        "executive_summary": "The evaluation gate failed with one missing-evidence case.",
        "context": {
            "best_retrieval_mode": "hybrid",
            "dataset_case_count": "4",
            "embedding_model": "local-hash-v1",
            "gate_status": "failed",
            "hybrid.latency_p95_ms": "18",
            "hybrid.mrr": "0.500",
            "hybrid.precision_at_k": "0.400",
            "hybrid.recall_at_k": "0.750",
            "top_k": "5"
        },
        "findings": [
            {
                "code": "expected_evidence_missing:case-1:hybrid",
                "severity": "critical",
                "title": "Which policy applies to failed indexing?",
                "summary": "The expected policy chunk was not retrieved.",
                "failure_labels": ["expected_evidence_missing"],
                "evidence_refs": ["M1"]
            },
            {
                "code": "retrieval_mode_comparison",
                "severity": "info",
                "title": "Retrieval modes produced different outcomes",
                "summary": "Hybrid led lexical by 25% recall.",
                "failure_labels": [],
                "evidence_refs": []
            }
        ],
        "recommendations": [{
            "code": "expand_corpus_coverage",
            "priority": "critical",
            "area": "corpus_coverage",
            "title": "Add the missing policy evidence",
            "rationale": "No indexed chunk satisfies the expected evidence reference.",
            "action": "Ingest the current policy source and rerun the dataset gate.",
            "finding_codes": ["expected_evidence_missing:case-1:hybrid"]
        }],
        "evidence": [{
            "label": "M1",
            "role": "missing",
            "source_id": null,
            "document_id": null,
            "chunk_id": "00000000-0000-0000-0000-000000000221",
            "rank": null,
            "document_path": null,
            "section_title": null,
            "checksum_prefix": null,
            "citation_label": null,
            "snippet": null,
            "evidence_strength": null,
            "chunk_quality_flags": [],
            "retrieval_quality_flags": []
        }],
        "diagnosis": diagnosis_json(
            "failing",
            "missing_expected_evidence",
            "critical",
            "Expected evidence was not retrieved",
            &["M1"]
        ),
        "created_at": "2026-06-30T08:15:30Z"
    }))
}

fn ci_report() -> DebugReport {
    report_from_json(json!({
        "id": "00000000-0000-0000-0000-000000000301",
        "workspace_id": "00000000-0000-0000-0000-000000000102",
        "project_id": "00000000-0000-0000-0000-000000000103",
        "title": "RAG CI gate audit",
        "subject": "SECRET branch query",
        "source": { "type": "ci_eval_run", "run_id": "00000000-0000-0000-0000-000000000304" },
        "privacy_mode": "metadata_only",
        "executive_summary": "The CI gate failed after the indexing configuration changed.",
        "context": {
            "ci_branch": "feature/index-v2",
            "ci_commit_sha": "abc123def456",
            "ci_config_label": "gpu-index-v2",
            "ci_gate_status": "failed",
            "ci_latency_delta_ms": "5",
            "ci_mrr_delta": "-0.100",
            "ci_newly_failed_case_count": "1",
            "ci_precision_delta": "-0.100",
            "ci_recall_delta": "-0.200",
            "query_text": "SECRET query content",
            "top_k": "5"
        },
        "findings": [{
            "code": "ci_regression",
            "severity": "critical",
            "title": "CI regression comparison",
            "summary": "One case failed after the indexing change.",
            "failure_labels": ["expected_evidence_missing"],
            "evidence_refs": []
        }],
        "recommendations": [{
            "code": "review_ci_regression",
            "priority": "critical",
            "area": "retrieval_mode",
            "title": "Review newly failing CI cases",
            "rationale": "The current configuration introduced a new retrieval failure.",
            "action": "Compare the baseline and head configuration before release.",
            "finding_codes": ["ci_regression"]
        }],
        "evidence": [],
        "diagnosis": diagnosis_json(
            "failing",
            "missing_expected_evidence",
            "critical",
            "Expected evidence was not retrieved",
            &[]
        ),
        "created_at": "2026-06-30T08:15:30Z"
    }))
}

fn metadata_only_report() -> DebugReport {
    let mut report = trace_report(DebugReportPrivacyMode::MetadataOnly);
    report.subject = "SECRET customer query".to_owned();
    report
        .context
        .insert("query".to_owned(), "SECRET query".to_owned());
    report
        .context
        .insert("document_path".to_owned(), "private/document.md".to_owned());
    report.evidence[0].document_path = Some("private/document.md".to_owned());
    report.evidence[0].section_title = Some("SECRET section".to_owned());
    report.evidence[0].snippet = Some("SECRET evidence snippet".to_owned());
    report
}

fn snippets_allowed_report() -> DebugReport {
    let long_snippet = format!("<script>alert('x')</script> {}", "x".repeat(320));
    report_from_json(json!({
        "id": "00000000-0000-0000-0000-000000000401",
        "workspace_id": "00000000-0000-0000-0000-000000000102",
        "project_id": "00000000-0000-0000-0000-000000000103",
        "title": "Audit *special* [customer]",
        "subject": "Why did #retrieval return [unsafe](link)?",
        "source": { "type": "manual", "label": "Customer [alpha] <script>" },
        "privacy_mode": "snippets_allowed",
        "executive_summary": "User-controlled <content> is escaped before Markdown export.",
        "context": { "document_path": "docs/[private].md", "top_k": "5" },
        "findings": [],
        "recommendations": [],
        "evidence": [evidence_json(&long_snippet)],
        "created_at": "2026-06-30T08:15:30Z"
    }))
}

fn evidence_json(snippet: &str) -> Value {
    json!({
        "label": "E1",
        "role": "retrieved",
        "source_id": "00000000-0000-0000-0000-000000000121",
        "document_id": "00000000-0000-0000-0000-000000000122",
        "chunk_id": "00000000-0000-0000-0000-000000000123",
        "rank": 1,
        "document_path": "technical/gpu.md",
        "section_title": "Index publication",
        "checksum_prefix": "abc123def456",
        "citation_label": "[1]",
        "snippet": snippet,
        "evidence_strength": "medium",
        "chunk_quality_flags": ["duplicate"],
        "retrieval_quality_flags": ["weak_evidence"]
    })
}

fn diagnosis_json(
    outcome: &str,
    code: &str,
    severity: &str,
    title: &str,
    evidence_refs: &[&str],
) -> Value {
    json!({
        "outcome": outcome,
        "summary": format!("This report looks {outcome}. Primary issue: {title}"),
        "primary_issue": {
            "code": code,
            "severity": severity,
            "title": title,
            "summary": "CorpusLab detected this issue from persisted retrieval metadata.",
            "evidence_refs": evidence_refs
        },
        "failures": [{
            "code": code,
            "severity": severity,
            "title": title,
            "summary": "CorpusLab detected this issue from persisted retrieval metadata.",
            "evidence_refs": evidence_refs
        }],
        "score_explanations": [],
        "recommendations": []
    })
}

fn report_from_json(value: Value) -> DebugReport {
    serde_json::from_value(value).expect("valid debug report fixture")
}

fn privacy_label(mode: DebugReportPrivacyMode) -> &'static str {
    match mode {
        DebugReportPrivacyMode::MetadataOnly => "metadata_only",
        DebugReportPrivacyMode::SnippetsAllowed => "snippets_allowed",
        DebugReportPrivacyMode::FullLocalOnly => "full_local_only",
    }
}
