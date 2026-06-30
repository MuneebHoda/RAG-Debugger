use std::collections::BTreeMap;

use rag_debugger_core::{
    ChunkId, ChunkQualityFlag, CiEvalRunId, DebugReport, DebugReportEvidenceRef,
    DebugReportEvidenceRole, DebugReportFinding, DebugReportId, DebugReportPrivacyMode,
    DebugReportRecommendation, DebugReportRecommendationArea, DebugReportRecommendationPriority,
    DebugReportSeverity, DebugReportSource, DocumentId, EvidenceStrength, ProjectId,
    RetrievalEvalExperimentId, RetrievalQualityFlag, SourceId, TraceId, WorkspaceId,
};
use serde_json::json;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

#[test]
fn debug_report_round_trips_with_rfc3339_timestamp() {
    let report = sample_report();

    let json = serde_json::to_value(&report).expect("report serializes");
    let decoded: DebugReport = serde_json::from_value(json.clone()).expect("report deserializes");

    assert_eq!(decoded, report);
    assert_eq!(json["source"]["type"], "trace");
    assert_eq!(json["privacy_mode"], "snippets_allowed");
    assert_eq!(json["created_at"], "2026-06-30T08:15:30Z");
    assert_eq!(
        json["evidence"][0]["snippet"],
        "Published after validation."
    );
}

#[test]
fn older_reports_default_new_diagnosis_fields() {
    let mut value = serde_json::to_value(sample_report()).expect("report serializes");
    value
        .as_object_mut()
        .expect("report object")
        .remove("diagnosis");
    value["recommendations"][0]
        .as_object_mut()
        .expect("recommendation object")
        .remove("evidence_refs");

    let decoded: DebugReport = serde_json::from_value(value).expect("legacy report deserializes");

    assert!(decoded.diagnosis.is_none());
    assert!(decoded.recommendations[0].evidence_refs.is_empty());
}

#[test]
fn report_sources_use_stable_discriminators() {
    let sources = [
        (
            DebugReportSource::Trace {
                trace_id: TraceId(uuid("00000000-0000-0000-0000-000000000101")),
            },
            "trace",
        ),
        (
            DebugReportSource::EvalExperiment {
                experiment_id: RetrievalEvalExperimentId(uuid(
                    "00000000-0000-0000-0000-000000000102",
                )),
            },
            "eval_experiment",
        ),
        (
            DebugReportSource::CiEvalRun {
                run_id: CiEvalRunId(uuid("00000000-0000-0000-0000-000000000103")),
            },
            "ci_eval_run",
        ),
        (
            DebugReportSource::Manual {
                label: "Consultant review".to_owned(),
            },
            "manual",
        ),
    ];

    for (source, expected_type) in sources {
        let value = serde_json::to_value(&source).expect("source serializes");
        let decoded: DebugReportSource =
            serde_json::from_value(value.clone()).expect("source deserializes");

        assert_eq!(value["type"], expected_type);
        assert_eq!(decoded, source);
    }
}

#[test]
fn privacy_modes_use_stable_wire_values() {
    assert_eq!(
        serde_json::to_value(DebugReportPrivacyMode::MetadataOnly)
            .expect("privacy mode serializes"),
        json!("metadata_only")
    );
    assert_eq!(
        serde_json::to_value(DebugReportPrivacyMode::SnippetsAllowed)
            .expect("privacy mode serializes"),
        json!("snippets_allowed")
    );
    assert_eq!(
        serde_json::to_value(DebugReportPrivacyMode::FullLocalOnly)
            .expect("privacy mode serializes"),
        json!("full_local_only")
    );
}

#[test]
fn optional_evidence_fields_round_trip_as_null() {
    let evidence = DebugReportEvidenceRef {
        label: "E-missing".to_owned(),
        role: DebugReportEvidenceRole::Missing,
        source_id: None,
        document_id: Some(DocumentId(uuid("00000000-0000-0000-0000-000000000301"))),
        chunk_id: None,
        rank: None,
        document_path: None,
        section_title: None,
        checksum_prefix: None,
        citation_label: None,
        snippet: None,
        evidence_strength: None,
        chunk_quality_flags: Vec::new(),
        retrieval_quality_flags: Vec::new(),
    };

    let value = serde_json::to_value(&evidence).expect("evidence serializes");
    let decoded: DebugReportEvidenceRef =
        serde_json::from_value(value.clone()).expect("evidence deserializes");

    assert_eq!(decoded, evidence);
    assert!(value["snippet"].is_null());
    assert!(value["chunk_id"].is_null());
}

fn sample_report() -> DebugReport {
    DebugReport {
        id: DebugReportId(uuid("00000000-0000-0000-0000-000000000001")),
        workspace_id: WorkspaceId(uuid("00000000-0000-0000-0000-000000000002")),
        project_id: ProjectId(uuid("00000000-0000-0000-0000-000000000003")),
        title: "GPU indexing retrieval audit".to_owned(),
        subject: "Why was the staging index not published?".to_owned(),
        source: DebugReportSource::Trace {
            trace_id: TraceId(uuid("00000000-0000-0000-0000-000000000004")),
        },
        privacy_mode: DebugReportPrivacyMode::SnippetsAllowed,
        executive_summary: "The expected publication evidence ranked below a duplicate heading."
            .to_owned(),
        context: BTreeMap::from([
            ("retrieval_mode".to_owned(), "hybrid".to_owned()),
            ("top_k".to_owned(), "5".to_owned()),
        ]),
        findings: vec![DebugReportFinding {
            code: "duplicate_evidence".to_owned(),
            severity: DebugReportSeverity::Warning,
            title: "Duplicate evidence displaced the expected chunk".to_owned(),
            summary: "Two equivalent heading chunks occupied ranked evidence positions.".to_owned(),
            failure_labels: vec!["duplicate_evidence".to_owned()],
            evidence_refs: vec!["E1".to_owned()],
        }],
        recommendations: vec![DebugReportRecommendation {
            code: "deduplicate_chunks".to_owned(),
            priority: DebugReportRecommendationPriority::High,
            area: DebugReportRecommendationArea::Chunking,
            title: "Remove duplicate chunks before indexing".to_owned(),
            rationale: "Duplicate chunks reduce result diversity.".to_owned(),
            action: "Deduplicate normalized chunk text before embedding.".to_owned(),
            finding_codes: vec!["duplicate_evidence".to_owned()],
            evidence_refs: vec!["E1".to_owned()],
        }],
        evidence: vec![DebugReportEvidenceRef {
            label: "E1".to_owned(),
            role: DebugReportEvidenceRole::Retrieved,
            source_id: Some(SourceId(uuid("00000000-0000-0000-0000-000000000201"))),
            document_id: Some(DocumentId(uuid("00000000-0000-0000-0000-000000000202"))),
            chunk_id: Some(ChunkId(uuid("00000000-0000-0000-0000-000000000203"))),
            rank: Some(2),
            document_path: Some("technical_docs/gpu-indexing.md".to_owned()),
            section_title: Some("Index publication".to_owned()),
            checksum_prefix: Some("abc123def456".to_owned()),
            citation_label: Some("[1]".to_owned()),
            snippet: Some("Published after validation.".to_owned()),
            evidence_strength: Some(EvidenceStrength::Medium),
            chunk_quality_flags: vec![ChunkQualityFlag::Duplicate],
            retrieval_quality_flags: vec![RetrievalQualityFlag::Duplicate],
        }],
        diagnosis: None,
        created_at: OffsetDateTime::parse("2026-06-30T08:15:30Z", &Rfc3339)
            .expect("valid timestamp"),
    }
}

fn uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).expect("valid UUID")
}
