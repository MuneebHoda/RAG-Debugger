#![recursion_limit = "256"]

use rag_debugger_core::{
    CiEvalRun, DebugReportId, DebugReportPrivacyMode, ProjectId, RetrievalEvalExperiment,
    RetrievalQueryRequest, Trace, TraceRerunComparison, TraceRerunId, WorkspaceId,
};
use rag_debugger_rag::reports::{
    build_ci_eval_debug_report, build_eval_experiment_debug_report, build_trace_debug_report,
    DebugReportBuildContext, ReportBuildError,
};
use serde_json::{json, Value};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

#[test]
fn trace_report_is_deterministic_and_metadata_only_redacts_content() {
    let trace = trace_fixture();
    let context = context(DebugReportPrivacyMode::MetadataOnly);

    let first = build_trace_debug_report(context, &trace).expect("build trace report");
    let second = build_trace_debug_report(context, &trace).expect("rebuild trace report");

    assert_eq!(first, second);
    assert_eq!(first.subject, format!("Trace {}", trace.id.0));
    assert_eq!(first.evidence.len(), 1);
    assert_eq!(first.evidence[0].document_path, None);
    assert_eq!(first.evidence[0].section_title, None);
    assert_eq!(first.evidence[0].snippet, None);
    let diagnosis_json = serde_json::to_string(&first.diagnosis).expect("serialize diagnosis");
    assert!(!diagnosis_json.contains(&trace.input));
    assert!(!diagnosis_json.contains("technical/gpu.md"));
    assert!(!diagnosis_json.contains("Index publication requires checksum validation"));
    assert!(first.diagnosis.is_some());
    assert!(first
        .recommendations
        .iter()
        .any(|recommendation| recommendation.code == "improve_chunk_boundaries"));
}

#[test]
fn trace_report_records_latest_rerun_comparison() {
    let mut trace = trace_fixture();
    let response = trace.retrieval.clone().expect("retrieval response");
    trace.reruns.push(TraceRerunComparison {
        id: TraceRerunId(uuid("00000000-0000-0000-0000-000000000090")),
        request: RetrievalQueryRequest {
            query: response.run.query.clone(),
            top_k: 3,
            retrieval_mode: rag_debugger_core::RetrievalMode::Lexical,
            source_ids: Vec::new(),
            document_ids: Vec::new(),
        },
        response,
        score_delta: 0.125,
        latency_delta_ms: -4,
        overlap_count: 1,
        changed_rank_count: 1,
        diagnosis: None,
        created_at: timestamp(),
    });

    let report = build_trace_debug_report(context(DebugReportPrivacyMode::SnippetsAllowed), &trace)
        .expect("build rerun report");

    assert_eq!(report.context["latest_rerun_score_delta"], "0.125");
    assert!(report
        .findings
        .iter()
        .any(|finding| finding.code == "rerun_comparison"));
    assert_eq!(
        report.evidence[0].snippet.as_deref(),
        Some("Index publication requires checksum validation.")
    );
}

#[test]
fn trace_without_retrieval_payload_is_rejected() {
    let mut trace = trace_fixture();
    trace.retrieval = None;

    let error = build_trace_debug_report(context(DebugReportPrivacyMode::MetadataOnly), &trace)
        .expect_err("missing retrieval must fail");

    assert!(matches!(error, ReportBuildError::InvalidSource(_)));
}

#[test]
fn eval_report_maps_failures_missing_evidence_and_recommendations() {
    let experiment = experiment_fixture();
    let context = context(DebugReportPrivacyMode::SnippetsAllowed);

    let first = build_eval_experiment_debug_report(context, &experiment);
    let second = build_eval_experiment_debug_report(context, &experiment);

    assert_eq!(first, second);
    assert_eq!(first.subject, "Release quality");
    assert!(first
        .findings
        .iter()
        .any(|finding| finding.failure_labels == ["expected_evidence_missing"]));
    assert!(first
        .evidence
        .iter()
        .any(|evidence| evidence.role == rag_debugger_core::DebugReportEvidenceRole::Missing));
    assert!(first
        .recommendations
        .iter()
        .any(|recommendation| recommendation.code == "expand_corpus_coverage"));
}

#[test]
fn ci_report_preserves_regression_context() {
    let run = ci_run_fixture();

    let report = build_ci_eval_debug_report(context(DebugReportPrivacyMode::MetadataOnly), &run);

    assert_eq!(report.subject, format!("CI eval run {}", run.id.0));
    assert_eq!(report.context["ci_branch"], "feature/indexing");
    assert_eq!(report.context["ci_newly_failed_case_count"], "1");
    assert!(report
        .recommendations
        .iter()
        .any(|recommendation| recommendation.code == "review_ci_regression"));
}

fn context(privacy_mode: DebugReportPrivacyMode) -> DebugReportBuildContext {
    DebugReportBuildContext {
        report_id: DebugReportId(uuid("00000000-0000-0000-0000-000000000001")),
        workspace_id: WorkspaceId(uuid("00000000-0000-0000-0000-000000000002")),
        project_id: ProjectId(uuid("00000000-0000-0000-0000-000000000003")),
        privacy_mode,
        created_at: timestamp(),
    }
}

fn trace_fixture() -> Trace {
    serde_json::from_value(json!({
        "id": "00000000-0000-0000-0000-000000000010",
        "project_id": "00000000-0000-0000-0000-000000000003",
        "input": "When is the GPU index published?",
        "output": "After checksum validation.",
        "started_at": "2026-06-30T08:15:30Z",
        "completed_at": "2026-06-30T08:15:31Z",
        "retrieval_runs": [],
        "generation": null,
        "failure_labels": ["duplicate_evidence", "bad_chunking"],
        "source_run_id": "00000000-0000-0000-0000-000000000011",
        "summary": "Duplicate evidence weakened the result.",
        "status": "warning",
        "evidence_strength": "medium",
        "spans": [],
        "retrieval": retrieval_response_json(),
        "reruns": []
    }))
    .expect("valid trace fixture")
}

fn retrieval_response_json() -> Value {
    json!({
        "run": {
            "id": "00000000-0000-0000-0000-000000000011",
            "query": "When is the GPU index published?",
            "top_k": 5,
            "retrieval_mode": "hybrid",
            "latency_ms": 12,
            "created_at": "2026-06-30T08:15:30Z"
        },
        "answer": {
            "status": "answered",
            "text": "After checksum validation [1].",
            "citations": [{
                "label": "[1]",
                "chunk_id": "00000000-0000-0000-0000-000000000021",
                "document_id": "00000000-0000-0000-0000-000000000022",
                "document_path": "technical/gpu.md",
                "chunk_ordinal": 2,
                "section_title": "Index publication",
                "checksum_prefix": "abc123def456",
                "snippet": "Index publication requires checksum validation."
            }]
        },
        "hits": [{
            "rank": 1,
            "score": 2.5,
            "chunk": {
                "id": "00000000-0000-0000-0000-000000000021",
                "document_id": "00000000-0000-0000-0000-000000000022",
                "ordinal": 2,
                "text": "Index publication requires checksum validation before the control plane publishes the index.",
                "token_count": 12,
                "byte_range": { "start": 10, "end": 100 },
                "checksum": "abc123def4567890",
                "strategy": "structured",
                "section_title": "Index publication",
                "split_reason": "section_boundary",
                "quality_flags": ["duplicate"],
                "is_duplicate": true,
                "text_density": 0.9,
                "evidence_score_hint": 0.8
            },
            "document": {
                "id": "00000000-0000-0000-0000-000000000022",
                "source_id": "00000000-0000-0000-0000-000000000023",
                "path": "technical/gpu.md",
                "mime_type": "text/markdown",
                "checksum": "document-checksum",
                "byte_size": 120,
                "profile": "technical_docs",
                "extraction_quality": "high",
                "warnings": []
            },
            "source": {
                "id": "00000000-0000-0000-0000-000000000023",
                "project_id": "00000000-0000-0000-0000-000000000003",
                "name": "Technical docs",
                "kind": { "FileSet": { "root_hint": "fixtures" } },
                "sync_policy": "Manual",
                "chunking": { "target_tokens": 512, "overlap_tokens": 64, "strategy": "structured" }
            },
            "matched_terms": [{ "term": "published", "count": 1 }],
            "score_breakdown": { "semantic": 1.0, "lexical": 1.0, "phrase": 0.0, "section": 0.2, "path": 0.1, "metadata": 0.2 },
            "normalized_score_breakdown": { "semantic": 0.8, "lexical": 0.8, "phrase": 0.0, "section": 0.2, "path": 0.1, "metadata": 0.2 },
            "snippet": "Index publication requires checksum validation.",
            "citation": {
                "label": "[1]",
                "chunk_id": "00000000-0000-0000-0000-000000000021",
                "document_id": "00000000-0000-0000-0000-000000000022",
                "document_path": "technical/gpu.md",
                "chunk_ordinal": 2,
                "section_title": "Index publication",
                "checksum_prefix": "abc123def456",
                "snippet": "Index publication requires checksum validation."
            },
            "quality_flags": ["duplicate"],
            "evidence_strength": "medium",
            "duplicate_count": 2
        }],
        "embedding_status": {
            "readiness": "ready",
            "required": true,
            "model": { "provider": "local", "model_name": "local-hash-v1", "dimension": 384 },
            "total_chunks": 1,
            "indexed_chunks": 1,
            "missing_chunks": 0,
            "stale_chunks": 0
        }
    })
}

fn experiment_fixture() -> RetrievalEvalExperiment {
    serde_json::from_value(json!({
        "id": "00000000-0000-0000-0000-000000000040",
        "dataset_id": "00000000-0000-0000-0000-000000000041",
        "dataset_name": "Release quality",
        "name": "Hybrid comparison",
        "modes": ["lexical", "hybrid"],
        "top_k": 5,
        "config_snapshot": {
            "top_k": 5,
            "scoring_weights": {
                "semantic_hybrid": 1.0,
                "semantic_vector": 1.0,
                "lexical": 1.0,
                "frequency": 0.1,
                "phrase": 0.2,
                "section": 0.2,
                "path": 0.1,
                "metadata": 0.1
            },
            "embedding_model": { "provider": "local", "model_name": "local-hash-v1", "dimension": 384 },
            "dataset_case_count": 1
        },
        "mode_results": [{
            "retrieval_mode": "hybrid",
            "case_count": 1,
            "passed_count": 0,
            "average_recall_at_k": 0.0,
            "average_precision_at_k": 0.0,
            "mean_reciprocal_rank": 0.0,
            "citation_coverage": 0.0,
            "weak_evidence_count": 0,
            "missing_embedding_failures": 0,
            "latency_p50_ms": 12,
            "latency_p95_ms": 12,
            "case_results": [{
                "case_id": "00000000-0000-0000-0000-000000000042",
                "query": "When is the index published?",
                "top_k": 5,
                "recall_at_k": 0.0,
                "precision_at_k": 0.0,
                "mrr": 0.0,
                "top_hit_rank": null,
                "citation_coverage": 0.0,
                "weak_evidence_count": 0,
                "missing_embedding_failures": 0,
                "passed": false,
                "expected_chunk_ids": ["00000000-0000-0000-0000-000000000043"],
                "expected_document_ids": [],
                "retrieved_chunk_ids": [],
                "latency_ms": 12,
                "failures": [{
                    "case_id": "00000000-0000-0000-0000-000000000042",
                    "query": "When is the index published?",
                    "retrieval_mode": "hybrid",
                    "label": "expected_evidence_missing",
                    "severity": "critical",
                    "message": "No expected chunk or document was retrieved.",
                    "top_hit_rank": null
                }]
            }]
        }],
        "comparison": {
            "best_mode": "hybrid",
            "mode_count": 2,
            "recall_delta": 0.2,
            "precision_delta": 0.1,
            "latency_delta_ms": 3,
            "summary": "Hybrid led the compared modes."
        },
        "gate": {
            "status": "failed",
            "average_recall_at_k": 0.0,
            "weak_evidence_rate": 0.0,
            "critical_failure_count": 1,
            "recall_threshold": 0.8,
            "weak_evidence_limit": 0.2,
            "reasons": ["Recall is below the gate."]
        },
        "failures": [{
            "case_id": "00000000-0000-0000-0000-000000000042",
            "query": "When is the index published?",
            "retrieval_mode": "hybrid",
            "label": "expected_evidence_missing",
            "severity": "critical",
            "message": "No expected chunk or document was retrieved.",
            "top_hit_rank": null
        }],
        "created_at": "2026-06-30T08:15:30Z"
    }))
    .expect("valid experiment fixture")
}

fn ci_run_fixture() -> CiEvalRun {
    let experiment = experiment_fixture();
    serde_json::from_value(json!({
        "id": "00000000-0000-0000-0000-000000000050",
        "workspace_id": "00000000-0000-0000-0000-000000000002",
        "dataset_id": experiment.dataset_id,
        "dataset_name": experiment.dataset_name,
        "experiment_id": experiment.id,
        "status": "failed",
        "gate_status": "failed",
        "branch": "feature/indexing",
        "commit_sha": "abc123def456",
        "base_ref": "main",
        "head_ref": "feature/indexing",
        "config_label": "gpu-index-v2",
        "regression": {
            "baseline_run_id": "00000000-0000-0000-0000-000000000051",
            "recall_delta": -0.2,
            "precision_delta": -0.1,
            "mrr_delta": -0.1,
            "latency_delta_ms": 5,
            "newly_failed_case_count": 1,
            "summary": "One case failed after the indexing change."
        },
        "report": {
            "title": "CI gate failed",
            "summary": "The release gate failed.",
            "gate": experiment.gate,
            "experiment": experiment,
            "failed_cases": experiment.failures
        },
        "created_at": "2026-06-30T08:15:30Z"
    }))
    .expect("valid CI run fixture")
}

fn timestamp() -> OffsetDateTime {
    OffsetDateTime::parse("2026-06-30T08:15:30Z", &Rfc3339).expect("valid timestamp")
}

fn uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).expect("valid UUID")
}
