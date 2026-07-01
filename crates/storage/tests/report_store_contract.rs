use std::collections::BTreeMap;

use rag_debugger_core::{
    DebugReport, DebugReportId, DebugReportPrivacyMode, DebugReportSource, ProjectId, WorkspaceId,
};
use rag_debugger_storage::{memory::MemoryStore, repository::ReportRepository, StorageError};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

#[tokio::test]
async fn memory_store_orders_and_scopes_debug_reports() {
    let store = MemoryStore::default();
    let workspace = WorkspaceId(Uuid::now_v7());
    let other_workspace = WorkspaceId(Uuid::now_v7());
    let now = OffsetDateTime::now_utc();
    let older = report(workspace, now - Duration::minutes(1), "Older audit");
    let newer = report(workspace, now, "Newer audit");
    let private = report(other_workspace, now, "Other workspace audit");

    store
        .save_debug_report(older.clone())
        .await
        .expect("save older report");
    store
        .save_debug_report(newer.clone())
        .await
        .expect("save newer report");
    store
        .save_debug_report(private.clone())
        .await
        .expect("save other workspace report");

    let reports = store
        .list_debug_reports(workspace)
        .await
        .expect("list workspace reports");
    assert_eq!(
        reports.iter().map(|report| report.id).collect::<Vec<_>>(),
        vec![newer.id, older.id]
    );
    assert_eq!(
        store
            .get_debug_report(workspace, older.id)
            .await
            .expect("get owned report"),
        older
    );
    assert!(matches!(
        store.get_debug_report(workspace, private.id).await,
        Err(StorageError::NotFound)
    ));
}

#[tokio::test]
async fn memory_store_rejects_duplicate_report_ids() {
    let store = MemoryStore::default();
    let report = report(
        WorkspaceId(Uuid::now_v7()),
        OffsetDateTime::now_utc(),
        "Duplicate audit",
    );

    store
        .save_debug_report(report.clone())
        .await
        .expect("save report");
    let error = store
        .save_debug_report(report)
        .await
        .expect_err("duplicate report must fail");

    assert!(matches!(error, StorageError::Conflict(_)));
}

fn report(workspace_id: WorkspaceId, created_at: OffsetDateTime, title: &str) -> DebugReport {
    DebugReport {
        id: DebugReportId(Uuid::now_v7()),
        workspace_id,
        project_id: ProjectId(Uuid::now_v7()),
        title: title.to_owned(),
        subject: "Metadata-only report".to_owned(),
        source: DebugReportSource::Manual {
            label: "Storage contract".to_owned(),
        },
        privacy_mode: DebugReportPrivacyMode::MetadataOnly,
        executive_summary: "Storage contract fixture".to_owned(),
        context: BTreeMap::new(),
        findings: Vec::new(),
        recommendations: Vec::new(),
        evidence: Vec::new(),
        diagnosis: None,
        created_at,
    }
}
