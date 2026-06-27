use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    auth::{WorkspaceId, WorkspaceRole},
    eval::{
        RetrievalEvalDatasetId, RetrievalEvalExperiment, RetrievalEvalExperimentId,
        RetrievalEvalFailure, RetrievalEvalGate, RetrievalEvalGateStatus,
    },
    retrieval::RetrievalMode,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct CiEvalRunId(pub Uuid);

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct RunCiEvalRequest {
    pub dataset_id: RetrievalEvalDatasetId,
    pub name: Option<String>,
    #[serde(default)]
    pub modes: Vec<RetrievalMode>,
    pub top_k: Option<u32>,
    pub branch: Option<String>,
    pub commit_sha: Option<String>,
    pub base_ref: Option<String>,
    pub head_ref: Option<String>,
    pub config_label: Option<String>,
    #[serde(default)]
    pub fail_on_gate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CiEvalRun {
    pub id: CiEvalRunId,
    pub workspace_id: WorkspaceId,
    pub dataset_id: RetrievalEvalDatasetId,
    pub dataset_name: String,
    pub experiment_id: RetrievalEvalExperimentId,
    pub status: CiEvalRunStatus,
    pub gate_status: RetrievalEvalGateStatus,
    pub branch: Option<String>,
    pub commit_sha: Option<String>,
    pub base_ref: Option<String>,
    pub head_ref: Option<String>,
    pub config_label: String,
    pub regression: Option<CiEvalRegressionSummary>,
    pub report: CiEvalReport,
    #[serde(with = "crate::wire_time")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CiEvalRunStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CiEvalRegressionSummary {
    pub baseline_run_id: CiEvalRunId,
    pub recall_delta: f32,
    pub precision_delta: f32,
    pub mrr_delta: f32,
    pub latency_delta_ms: i64,
    pub newly_failed_case_count: u32,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CiEvalReport {
    pub title: String,
    pub summary: String,
    pub gate: RetrievalEvalGate,
    pub experiment: RetrievalEvalExperiment,
    pub failed_cases: Vec<RetrievalEvalFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CiEvalRunReportResponse {
    pub run: CiEvalRun,
    pub report: CiEvalReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct CiEvalPrincipal {
    pub workspace_id: WorkspaceId,
    pub role: Option<WorkspaceRole>,
}
