use rag_debugger_core::{NativeTraceIngestionRequest, Trace};
use serde_json::json;

#[test]
fn legacy_trace_json_remains_readable() {
    let trace: Trace = serde_json::from_value(json!({
        "id": "00000000-0000-0000-0000-000000000001",
        "project_id": "00000000-0000-0000-0000-000000000002",
        "input": "legacy query",
        "output": null,
        "started_at": "2026-08-12T08:00:00Z",
        "completed_at": null,
        "generation": null
    }))
    .expect("legacy trace");

    assert!(trace.ingestion.is_none());
}

#[test]
fn native_contract_rejects_unknown_fields() {
    let value = json!({
        "schema_version": "1",
        "project_id": "00000000-0000-0000-0000-000000000002",
        "external_trace_id": "external-1",
        "privacy_mode": "metadata_only",
        "unexpected": true
    });

    assert!(serde_json::from_value::<NativeTraceIngestionRequest>(value).is_err());
}
