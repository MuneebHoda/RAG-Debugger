use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{model::ModelConfigId, project::ProjectId, trace::TraceId};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct EvalDataset {
    pub id: EvalDatasetId,
    pub project_id: ProjectId,
    pub name: String,
    pub cases: Vec<EvalCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct EvalCase {
    pub id: EvalCaseId,
    pub question: String,
    pub expected_answer: Option<String>,
    pub required_source_refs: Vec<String>,
    pub rubric: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvalRun {
    pub id: EvalRunId,
    pub dataset_id: EvalDatasetId,
    pub model_config_id: ModelConfigId,
    pub status: EvalRunStatus,
    pub results: Vec<EvalCaseResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvalCaseResult {
    pub case_id: EvalCaseId,
    pub trace_id: Option<TraceId>,
    pub retrieval_recall_at_k: Option<f32>,
    pub mean_reciprocal_rank: Option<f32>,
    pub faithfulness_score: Option<f32>,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum EvalRunStatus {
    Pending,
    Running,
    Passed,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct EvalDatasetId(pub Uuid);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct EvalCaseId(pub Uuid);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct EvalRunId(pub Uuid);
