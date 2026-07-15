use serde::{Deserialize, Serialize};
use uuid::Uuid;

use time::OffsetDateTime;

use crate::{
    chunk::{ChunkId, ChunkQualityFlag},
    config::RetrievalWeights,
    diagnosis::EvidenceDiagnosisSummary,
    embedding::EmbeddingModelInfo,
    model::ModelConfigId,
    project::ProjectId,
    retrieval::{RetrievalMode, DEFAULT_RETRIEVAL_TOP_K},
    source::{DocumentId, DocumentProfile, DocumentWarning, ExtractionQuality, SourceId},
    trace::TraceId,
};

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

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct RetrievalEvalCase {
    pub id: RetrievalEvalCaseId,
    pub name: String,
    pub query: String,
    pub top_k: u32,
    pub expected_chunk_ids: Vec<ChunkId>,
    pub expected_document_ids: Vec<DocumentId>,
    pub notes: Option<String>,
    #[serde(with = "crate::wire_time")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalDataset {
    pub id: RetrievalEvalDatasetId,
    pub name: String,
    pub description: Option<String>,
    pub cases: Vec<RetrievalEvalCase>,
    #[serde(with = "crate::wire_time")]
    pub created_at: OffsetDateTime,
    #[serde(with = "crate::wire_time")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalDatasetSummary {
    pub id: RetrievalEvalDatasetId,
    pub name: String,
    pub description: Option<String>,
    pub case_count: u32,
    pub latest_experiment_id: Option<RetrievalEvalExperimentId>,
    pub latest_gate: Option<RetrievalEvalGate>,
    pub latest_average_recall_at_k: Option<f32>,
    pub latest_average_precision_at_k: Option<f32>,
    #[serde(with = "crate::wire_time")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct CreateRetrievalEvalDatasetRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct CreateRetrievalEvalLabCaseRequest {
    pub name: Option<String>,
    pub query: String,
    #[serde(default = "default_eval_top_k")]
    pub top_k: u32,
    #[serde(default)]
    pub expected_chunk_ids: Vec<ChunkId>,
    #[serde(default)]
    pub expected_document_ids: Vec<DocumentId>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Default)]
pub struct UpdateRetrievalEvalCaseRequest {
    pub name: Option<String>,
    pub query: Option<String>,
    pub top_k: Option<u32>,
    pub expected_chunk_ids: Option<Vec<ChunkId>>,
    pub expected_document_ids: Option<Vec<DocumentId>>,
    #[serde(
        default,
        deserialize_with = "deserialize_present_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub notes: Option<Option<String>>,
}

fn deserialize_present_option<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct QueryEvalLabEvidenceRequest {
    pub query: Option<String>,
    #[serde(default)]
    pub document_ids: Vec<DocumentId>,
    #[serde(default)]
    pub chunk_ids: Vec<ChunkId>,
    pub limit: Option<u32>,
    #[serde(default)]
    pub include_chunks: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueryEvalLabEvidenceResponse {
    pub documents: Vec<EvalLabEvidenceDocument>,
    pub chunks: Vec<EvalLabEvidenceChunk>,
    pub unresolved_document_ids: Vec<DocumentId>,
    pub unresolved_chunk_ids: Vec<ChunkId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct EvalLabEvidenceDocument {
    pub id: DocumentId,
    pub source_id: SourceId,
    pub source_name: String,
    pub path: String,
    pub profile: DocumentProfile,
    pub extraction_quality: ExtractionQuality,
    pub warnings: Vec<DocumentWarning>,
    pub chunk_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvalLabEvidenceChunk {
    pub id: ChunkId,
    pub document_id: DocumentId,
    pub source_id: SourceId,
    pub source_name: String,
    pub document_path: String,
    pub ordinal: u32,
    pub text: String,
    pub token_count: u32,
    pub checksum: String,
    pub section_title: Option<String>,
    pub quality_flags: Vec<ChunkQualityFlag>,
    pub is_duplicate: bool,
    pub text_density: f32,
    pub evidence_score_hint: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct RunRetrievalEvalExperimentRequest {
    pub dataset_id: RetrievalEvalDatasetId,
    pub name: Option<String>,
    #[serde(default)]
    pub modes: Vec<RetrievalMode>,
    pub top_k: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Default)]
pub struct CompareRetrievalEvalExperimentRequest {
    #[serde(default)]
    pub modes: Vec<RetrievalMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalExperiment {
    pub id: RetrievalEvalExperimentId,
    pub dataset_id: RetrievalEvalDatasetId,
    pub dataset_name: String,
    pub name: String,
    pub modes: Vec<RetrievalMode>,
    pub top_k: u32,
    pub config_snapshot: RetrievalEvalConfigSnapshot,
    pub mode_results: Vec<RetrievalEvalModeResult>,
    pub comparison: RetrievalEvalComparison,
    pub gate: RetrievalEvalGate,
    pub failures: Vec<RetrievalEvalFailure>,
    #[serde(with = "crate::wire_time")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalConfigSnapshot {
    pub top_k: u32,
    pub scoring_weights: RetrievalWeights,
    pub embedding_model: EmbeddingModelInfo,
    pub dataset_case_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalModeResult {
    pub retrieval_mode: RetrievalMode,
    pub case_count: u32,
    pub passed_count: u32,
    pub average_recall_at_k: f32,
    pub average_precision_at_k: f32,
    pub mean_reciprocal_rank: f32,
    pub citation_coverage: f32,
    pub weak_evidence_count: u32,
    pub missing_embedding_failures: u32,
    pub latency_p50_ms: u64,
    pub latency_p95_ms: u64,
    pub case_results: Vec<RetrievalEvalCaseEvaluation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalCaseEvaluation {
    pub case_id: RetrievalEvalCaseId,
    pub query: String,
    pub top_k: u32,
    pub recall_at_k: f32,
    pub precision_at_k: f32,
    pub mrr: f32,
    pub top_hit_rank: Option<u32>,
    pub citation_coverage: f32,
    pub weak_evidence_count: u32,
    pub missing_embedding_failures: u32,
    pub passed: bool,
    pub expected_chunk_ids: Vec<ChunkId>,
    pub expected_document_ids: Vec<DocumentId>,
    pub retrieved_chunk_ids: Vec<ChunkId>,
    pub latency_ms: u64,
    pub failures: Vec<RetrievalEvalFailure>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnosis: Option<EvidenceDiagnosisSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalComparison {
    pub best_mode: Option<RetrievalMode>,
    pub mode_count: u32,
    pub recall_delta: f32,
    pub precision_delta: f32,
    pub latency_delta_ms: i64,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalExperimentSummary {
    pub id: RetrievalEvalExperimentId,
    pub dataset_id: RetrievalEvalDatasetId,
    pub dataset_name: String,
    pub name: String,
    pub modes: Vec<RetrievalMode>,
    pub top_k: u32,
    pub best_mode: Option<RetrievalMode>,
    pub gate_status: RetrievalEvalGateStatus,
    pub average_recall_at_k: f32,
    pub average_precision_at_k: f32,
    pub mean_reciprocal_rank: f32,
    pub citation_coverage: f32,
    pub weak_evidence_case_rate: f32,
    pub missing_embedding_failures: u32,
    pub latency_p50_ms: u64,
    pub latency_p95_ms: u64,
    pub failure_count: u32,
    #[serde(with = "crate::wire_time")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalTrendSummary {
    pub dataset_id: RetrievalEvalDatasetId,
    pub experiment_count: u32,
    pub window_limit: u32,
    pub latest_experiment_id: Option<RetrievalEvalExperimentId>,
    pub latest_gate_status: Option<RetrievalEvalGateStatus>,
    pub points: Vec<RetrievalEvalTrendPoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_regression: Option<RetrievalEvalRegressionComparison>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalTrendPoint {
    pub experiment_id: RetrievalEvalExperimentId,
    pub name: String,
    pub best_mode: Option<RetrievalMode>,
    pub gate_status: RetrievalEvalGateStatus,
    pub average_recall_at_k: f32,
    pub average_precision_at_k: f32,
    pub mean_reciprocal_rank: f32,
    pub citation_coverage: f32,
    pub weak_evidence_case_rate: f32,
    pub latency_p95_ms: u64,
    pub failure_count: u32,
    #[serde(with = "crate::wire_time")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalRegressionComparison {
    pub current_experiment_id: RetrievalEvalExperimentId,
    pub baseline_experiment_id: Option<RetrievalEvalExperimentId>,
    pub classification: RetrievalEvalRegressionClassification,
    pub current_gate_status: RetrievalEvalGateStatus,
    pub baseline_gate_status: Option<RetrievalEvalGateStatus>,
    pub metric_deltas: Vec<RetrievalEvalMetricDelta>,
    pub newly_failed_cases: Vec<RetrievalEvalCaseRegression>,
    pub recovered_cases: Vec<RetrievalEvalCaseRegression>,
    pub changed_top_evidence_cases: Vec<RetrievalEvalCaseRegression>,
    pub changed_failure_label_cases: Vec<RetrievalEvalCaseRegression>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalMetricDelta {
    pub metric: RetrievalEvalRegressionMetric,
    pub current: f32,
    pub baseline: Option<f32>,
    pub delta: f32,
    pub classification: RetrievalEvalRegressionClassification,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalCaseRegression {
    pub case_id: RetrievalEvalCaseId,
    pub retrieval_mode: RetrievalMode,
    pub query: String,
    pub classification: RetrievalEvalRegressionClassification,
    pub current_passed: Option<bool>,
    pub baseline_passed: Option<bool>,
    pub current_top_hit_rank: Option<u32>,
    pub baseline_top_hit_rank: Option<u32>,
    pub current_retrieved_chunk_ids: Vec<ChunkId>,
    pub baseline_retrieved_chunk_ids: Vec<ChunkId>,
    pub current_failure_labels: Vec<RetrievalEvalFailureLabel>,
    pub baseline_failure_labels: Vec<RetrievalEvalFailureLabel>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalEvalRegressionClassification {
    Improved,
    Regressed,
    Unchanged,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalEvalRegressionMetric {
    RecallAtK,
    PrecisionAtK,
    MeanReciprocalRank,
    CitationCoverage,
    WeakEvidenceCaseRate,
    MissingEmbeddingFailures,
    LatencyP95Ms,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalGate {
    pub status: RetrievalEvalGateStatus,
    pub average_recall_at_k: f32,
    pub weak_evidence_rate: f32,
    pub critical_failure_count: u32,
    pub recall_threshold: f32,
    pub weak_evidence_limit: f32,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalEvalGateStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalFailure {
    pub case_id: RetrievalEvalCaseId,
    pub query: String,
    pub retrieval_mode: RetrievalMode,
    pub label: RetrievalEvalFailureLabel,
    pub severity: RetrievalEvalFailureSeverity,
    pub message: String,
    pub top_hit_rank: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalEvalFailureLabel {
    ExpectedEvidenceMissing,
    CorrectDocumentWrongChunk,
    LowPrecision,
    WeakEvidence,
    MissingEmbeddings,
    HeadingOnlyEvidence,
    DuplicateEvidence,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalEvalFailureSeverity {
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct CreateRetrievalEvalCaseRequest {
    pub name: Option<String>,
    pub query: String,
    #[serde(default = "default_eval_top_k")]
    pub top_k: u32,
    #[serde(default)]
    pub expected_chunk_ids: Vec<ChunkId>,
    #[serde(default)]
    pub expected_document_ids: Vec<DocumentId>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct RunRetrievalEvalRequest {
    #[serde(default)]
    pub case_ids: Vec<RetrievalEvalCaseId>,
    #[serde(default)]
    pub retrieval_mode: RetrievalMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalRun {
    pub id: RetrievalEvalRunId,
    pub retrieval_mode: RetrievalMode,
    pub case_count: u32,
    pub passed_count: u32,
    pub average_recall_at_k: f32,
    pub average_precision_at_k: f32,
    #[serde(with = "crate::wire_time")]
    pub created_at: OffsetDateTime,
    pub results: Vec<RetrievalEvalResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalEvalResult {
    pub case_id: RetrievalEvalCaseId,
    pub query: String,
    pub top_k: u32,
    pub recall_at_k: f32,
    pub precision_at_k: f32,
    pub top_hit_rank: Option<u32>,
    pub passed: bool,
    pub expected_chunk_ids: Vec<ChunkId>,
    pub expected_document_ids: Vec<DocumentId>,
    pub retrieved_chunk_ids: Vec<ChunkId>,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct RetrievalEvalCaseId(pub Uuid);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct RetrievalEvalRunId(pub Uuid);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct RetrievalEvalDatasetId(pub Uuid);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct RetrievalEvalExperimentId(pub Uuid);

fn default_eval_top_k() -> u32 {
    DEFAULT_RETRIEVAL_TOP_K
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use time::format_description::well_known::Rfc3339;

    use super::*;

    #[test]
    fn update_case_notes_distinguish_missing_null_and_string_values() {
        let missing: UpdateRetrievalEvalCaseRequest =
            serde_json::from_value(json!({})).expect("missing notes deserialize");
        let cleared: UpdateRetrievalEvalCaseRequest = serde_json::from_value(json!({
            "notes": null
        }))
        .expect("null notes deserialize");
        let replaced: UpdateRetrievalEvalCaseRequest = serde_json::from_value(json!({
            "notes": "Updated"
        }))
        .expect("string notes deserialize");

        assert_eq!(missing.notes, None);
        assert_eq!(cleared.notes, Some(None));
        assert_eq!(replaced.notes, Some(Some("Updated".to_owned())));

        let missing_json = serde_json::to_value(missing).expect("missing notes serialize");
        let cleared_json = serde_json::to_value(cleared).expect("null notes serialize");
        let replaced_json = serde_json::to_value(replaced).expect("string notes serialize");
        assert!(missing_json.get("notes").is_none());
        assert_eq!(cleared_json["notes"], serde_json::Value::Null);
        assert_eq!(replaced_json["notes"], "Updated");
    }

    #[test]
    fn regression_contract_uses_stable_wire_values() {
        let case_id = RetrievalEvalCaseId(Uuid::now_v7());
        let chunk_id = ChunkId(Uuid::now_v7());
        let comparison = RetrievalEvalRegressionComparison {
            current_experiment_id: RetrievalEvalExperimentId(Uuid::now_v7()),
            baseline_experiment_id: Some(RetrievalEvalExperimentId(Uuid::now_v7())),
            classification: RetrievalEvalRegressionClassification::Regressed,
            current_gate_status: RetrievalEvalGateStatus::Failed,
            baseline_gate_status: Some(RetrievalEvalGateStatus::Passed),
            metric_deltas: vec![RetrievalEvalMetricDelta {
                metric: RetrievalEvalRegressionMetric::RecallAtK,
                current: 0.6,
                baseline: Some(0.9),
                delta: -0.3,
                classification: RetrievalEvalRegressionClassification::Regressed,
            }],
            newly_failed_cases: vec![RetrievalEvalCaseRegression {
                case_id,
                retrieval_mode: RetrievalMode::Hybrid,
                query: "Which chunks support account recovery?".to_owned(),
                classification: RetrievalEvalRegressionClassification::Regressed,
                current_passed: Some(false),
                baseline_passed: Some(true),
                current_top_hit_rank: Some(4),
                baseline_top_hit_rank: Some(1),
                current_retrieved_chunk_ids: vec![chunk_id],
                baseline_retrieved_chunk_ids: Vec::new(),
                current_failure_labels: vec![RetrievalEvalFailureLabel::ExpectedEvidenceMissing],
                baseline_failure_labels: Vec::new(),
            }],
            recovered_cases: Vec::new(),
            changed_top_evidence_cases: Vec::new(),
            changed_failure_label_cases: Vec::new(),
            summary: "Current regressed compared with baseline.".to_owned(),
        };

        let json = serde_json::to_value(&comparison).expect("comparison serializes");
        assert_eq!(json["classification"], "regressed");
        assert_eq!(json["current_gate_status"], "failed");
        assert_eq!(json["baseline_gate_status"], "passed");
        assert_eq!(json["metric_deltas"][0]["metric"], "recall_at_k");
        assert_eq!(
            json["newly_failed_cases"][0]["current_failure_labels"][0],
            "expected_evidence_missing"
        );

        let round_trip: RetrievalEvalRegressionComparison =
            serde_json::from_value(json).expect("comparison deserializes");
        assert_eq!(round_trip, comparison);
    }

    #[test]
    fn trend_summary_serializes_rfc3339_points() {
        let summary = RetrievalEvalTrendSummary {
            dataset_id: RetrievalEvalDatasetId(Uuid::now_v7()),
            experiment_count: 1,
            window_limit: 10,
            latest_experiment_id: Some(RetrievalEvalExperimentId(Uuid::now_v7())),
            latest_gate_status: Some(RetrievalEvalGateStatus::Passed),
            points: vec![RetrievalEvalTrendPoint {
                experiment_id: RetrievalEvalExperimentId(Uuid::now_v7()),
                name: "Release gate".to_owned(),
                best_mode: Some(RetrievalMode::Hybrid),
                gate_status: RetrievalEvalGateStatus::Passed,
                average_recall_at_k: 0.9,
                average_precision_at_k: 0.8,
                mean_reciprocal_rank: 1.0,
                citation_coverage: 0.7,
                weak_evidence_case_rate: 0.0,
                latency_p95_ms: 42,
                failure_count: 0,
                created_at: OffsetDateTime::parse("2026-06-25T12:00:00Z", &Rfc3339)
                    .expect("fixture timestamp parses"),
            }],
            latest_regression: None,
        };

        let json = serde_json::to_value(&summary).expect("trend serializes");
        assert_eq!(json["latest_gate_status"], "passed");
        assert_eq!(json["points"][0]["created_at"], "2026-06-25T12:00:00Z");
        assert_eq!(json["points"][0]["best_mode"], "hybrid");

        let round_trip: RetrievalEvalTrendSummary =
            serde_json::from_value(json).expect("trend deserializes");
        assert_eq!(round_trip, summary);
    }

    #[test]
    fn evidence_lookup_contract_round_trips() {
        let document_id = DocumentId(Uuid::now_v7());
        let chunk_id = ChunkId(Uuid::now_v7());
        let source_id = SourceId(Uuid::now_v7());
        let response = QueryEvalLabEvidenceResponse {
            documents: vec![EvalLabEvidenceDocument {
                id: document_id,
                source_id,
                source_name: "Support KB".to_owned(),
                path: "support/account-recovery.md".to_owned(),
                profile: DocumentProfile::SupportKb,
                extraction_quality: ExtractionQuality::High,
                warnings: Vec::new(),
                chunk_count: 2,
            }],
            chunks: vec![EvalLabEvidenceChunk {
                id: chunk_id,
                document_id,
                source_id,
                source_name: "Support KB".to_owned(),
                document_path: "support/account-recovery.md".to_owned(),
                ordinal: 0,
                text: "Password reset links expire after fifteen minutes.".to_owned(),
                token_count: 7,
                checksum: "abc123".to_owned(),
                section_title: Some("Account recovery".to_owned()),
                quality_flags: vec![ChunkQualityFlag::GoodEvidenceCandidate],
                is_duplicate: false,
                text_density: 0.9,
                evidence_score_hint: 0.8,
            }],
            unresolved_document_ids: vec![DocumentId(Uuid::now_v7())],
            unresolved_chunk_ids: Vec::new(),
        };

        let json = serde_json::to_value(&response).expect("evidence serializes");
        assert_eq!(json["documents"][0]["profile"], "support_kb");
        assert_eq!(
            json["chunks"][0]["quality_flags"][0],
            "good_evidence_candidate"
        );
        assert!(json["unresolved_document_ids"][0].is_string());

        let round_trip: QueryEvalLabEvidenceResponse =
            serde_json::from_value(json).expect("evidence deserializes");
        assert_eq!(round_trip, response);
    }

    #[test]
    fn older_experiment_json_remains_valid_without_regression_fields() {
        let experiment_id = Uuid::now_v7();
        let dataset_id = Uuid::now_v7();
        let value = json!({
            "id": experiment_id,
            "dataset_id": dataset_id,
            "dataset_name": "Default retrieval dataset",
            "name": "Baseline comparison",
            "modes": ["hybrid"],
            "top_k": 5,
            "config_snapshot": {
                "top_k": 5,
                "scoring_weights": RetrievalWeights::default(),
                "embedding_model": EmbeddingModelInfo::default(),
                "dataset_case_count": 0
            },
            "mode_results": [],
            "comparison": {
                "best_mode": null,
                "mode_count": 0,
                "recall_delta": 0.0,
                "precision_delta": 0.0,
                "latency_delta_ms": 0,
                "summary": "No modes were evaluated."
            },
            "gate": {
                "status": "failed",
                "average_recall_at_k": 0.0,
                "weak_evidence_rate": 0.0,
                "critical_failure_count": 0,
                "recall_threshold": 0.8,
                "weak_evidence_limit": 0.2,
                "reasons": ["No cases were evaluated."]
            },
            "failures": [],
            "created_at": "2026-06-25T12:00:00Z"
        });

        let experiment: RetrievalEvalExperiment =
            serde_json::from_value(value).expect("legacy experiment deserializes");
        assert_eq!(experiment.id.0, experiment_id);
        assert_eq!(experiment.dataset_id.0, dataset_id);
    }
}
