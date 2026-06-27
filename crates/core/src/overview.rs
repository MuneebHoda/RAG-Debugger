use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{
    embedding::EmbeddingStatus,
    eval::{
        RetrievalEvalExperiment, RetrievalEvalExperimentId, RetrievalEvalGateStatus,
        RetrievalEvalRunId,
    },
    retrieval::RetrievalMode,
    source::DocumentProfile,
    trace::FailureLabel,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverviewResponse {
    #[serde(with = "crate::wire_time")]
    pub generated_at: OffsetDateTime,
    pub health: OverviewHealth,
    pub metrics: Vec<OverviewMetric>,
    pub pipeline: Vec<OverviewPipelineStep>,
    pub issues: Vec<OverviewIssue>,
    pub actions: Vec<OverviewAction>,
    pub recent_activity: Vec<OverviewActivity>,
    pub document_mix: Vec<OverviewDocumentProfile>,
    pub embedding_status: EmbeddingStatus,
    pub latest_eval_run: Option<OverviewEvalRunSummary>,
    pub latest_eval_experiment: Option<OverviewEvalExperimentSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverviewHealth {
    pub score: u32,
    pub status: OverviewHealthStatus,
    pub summary: String,
    pub primary_action: Option<OverviewAction>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OverviewHealthStatus {
    Ready,
    NeedsIndexing,
    NeedsEvalCoverage,
    NeedsDocuments,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverviewMetric {
    pub id: String,
    pub label: String,
    pub value: String,
    pub detail: String,
    pub tone: OverviewTone,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverviewPipelineStep {
    pub id: String,
    pub label: String,
    pub status: OverviewStepStatus,
    pub count: u32,
    pub detail: String,
    pub route: String,
    pub action_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverviewIssue {
    pub id: String,
    pub severity: OverviewSeverity,
    pub title: String,
    pub detail: String,
    pub route: String,
    pub action_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverviewAction {
    pub id: String,
    pub label: String,
    pub detail: String,
    pub route: String,
    pub priority: OverviewActionPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverviewActivity {
    pub id: String,
    pub kind: OverviewActivityKind,
    pub label: String,
    pub detail: String,
    pub route: String,
    #[serde(with = "crate::wire_time::option")]
    pub created_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverviewDocumentProfile {
    pub profile: DocumentProfile,
    pub count: u32,
    pub percentage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverviewEvalRunSummary {
    pub id: RetrievalEvalRunId,
    pub retrieval_mode: RetrievalMode,
    pub case_count: u32,
    pub passed_count: u32,
    pub pass_rate: f32,
    pub average_recall_at_k: f32,
    pub average_precision_at_k: f32,
    #[serde(with = "crate::wire_time")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverviewEvalExperimentSummary {
    pub id: RetrievalEvalExperimentId,
    pub dataset_name: String,
    pub gate_status: RetrievalEvalGateStatus,
    pub best_mode: Option<RetrievalMode>,
    pub average_recall_at_k: f32,
    pub average_precision_at_k: f32,
    pub failure_count: u32,
    #[serde(with = "crate::wire_time")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OverviewTone {
    Neutral,
    Good,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OverviewStepStatus {
    Complete,
    Warning,
    Pending,
    Blocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OverviewSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OverviewActionPriority {
    Primary,
    Secondary,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OverviewActivityKind {
    Source,
    Document,
    Trace,
    Eval,
}

impl OverviewEvalRunSummary {
    pub fn from_eval_run(run: &crate::eval::RetrievalEvalRun) -> Self {
        let pass_rate = if run.case_count == 0 {
            0.0
        } else {
            run.passed_count as f32 / run.case_count as f32
        };

        Self {
            id: run.id,
            retrieval_mode: run.retrieval_mode,
            case_count: run.case_count,
            passed_count: run.passed_count,
            pass_rate,
            average_recall_at_k: run.average_recall_at_k,
            average_precision_at_k: run.average_precision_at_k,
            created_at: run.created_at,
        }
    }
}

impl OverviewEvalExperimentSummary {
    pub fn from_experiment(experiment: &RetrievalEvalExperiment) -> Self {
        let best_result = experiment.mode_results.iter().max_by(|left, right| {
            left.average_recall_at_k
                .partial_cmp(&right.average_recall_at_k)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Self {
            id: experiment.id,
            dataset_name: experiment.dataset_name.clone(),
            gate_status: experiment.gate.status,
            best_mode: experiment.comparison.best_mode,
            average_recall_at_k: best_result.map_or(0.0, |result| result.average_recall_at_k),
            average_precision_at_k: best_result.map_or(0.0, |result| result.average_precision_at_k),
            failure_count: experiment.failures.len() as u32,
            created_at: experiment.created_at,
        }
    }
}

pub fn failure_label_summary(labels: &[FailureLabel]) -> String {
    if labels.is_empty() {
        return "no failure labels".to_owned();
    }

    labels
        .iter()
        .take(3)
        .map(|label| format!("{label:?}").to_ascii_lowercase().replace('_', " "))
        .collect::<Vec<_>>()
        .join(", ")
}
