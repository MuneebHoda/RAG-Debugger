use rag_debugger_core::{
    DemoLoadResponse, DemoProgress, DemoQueryId, DemoStatus, DemoSuggestedQuery, ProjectId,
    SourceId,
};
use serde_json::json;
use uuid::Uuid;

#[test]
fn demo_contract_round_trips_with_stable_query_ids() {
    let status = DemoStatus {
        version: "corpuslab-guided-demo-v1".to_owned(),
        project_id: Some(ProjectId(uuid("00000000-0000-0000-0000-000000000001"))),
        source_id: Some(SourceId(uuid("00000000-0000-0000-0000-000000000002"))),
        progress: DemoProgress {
            sample_corpus_loaded: true,
            chunks_created: true,
            document_count: 3,
            chunk_count: 12,
            ..DemoProgress::default()
        },
        suggested_queries: vec![DemoSuggestedQuery {
            id: DemoQueryId::AccountRecovery,
            question: "How does account recovery work?".to_owned(),
            description: "Diagnose duplicate support evidence.".to_owned(),
            recommended: true,
        }],
    };
    let response = DemoLoadResponse {
        created_documents: 3,
        status: status.clone(),
    };

    let value = serde_json::to_value(&response).expect("demo response serializes");
    let decoded: DemoLoadResponse =
        serde_json::from_value(value.clone()).expect("demo response deserializes");

    assert_eq!(decoded, response);
    assert_eq!(
        value["status"]["suggested_queries"][0]["id"],
        "account_recovery"
    );
    assert_eq!(value["status"]["progress"]["document_count"], 3);
    assert_eq!(
        serde_json::to_value(DemoQueryId::DataRetention).expect("query id serializes"),
        json!("data_retention")
    );
    assert_eq!(
        serde_json::to_value(DemoQueryId::GpuIndexing).expect("query id serializes"),
        json!("gpu_indexing")
    );
    assert_eq!(decoded.status, status);
}

#[test]
fn demo_progress_is_complete_only_after_report_generation() {
    let progress = DemoProgress {
        sample_corpus_loaded: true,
        chunks_created: true,
        embeddings_indexed: true,
        document_count: 3,
        chunk_count: 12,
        indexed_chunk_count: 12,
        retrieval_run_id: None,
        trace_id: None,
        report_id: None,
    };

    assert!(!progress.is_complete());
}

fn uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).expect("valid UUID")
}
