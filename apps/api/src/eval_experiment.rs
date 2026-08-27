use std::collections::BTreeMap;

use rag_debugger_core::{
    DeploymentMode, RetrievalEvalConfigSnapshot, RetrievalEvalDataset, RetrievalEvalExperiment,
    RetrievalEvalExperimentId, RetrievalEvalProvenanceInformation, RetrievalMode,
    RetrievalQueryRequest, WorkspaceId,
};
use rag_debugger_rag::{
    embedding::LocalHashEmbeddingProvider,
    evals::{
        compare_mode_results, evaluate_gate, evaluate_retrieval_eval_case_with_context,
        expected_chunk_parent_document_ids, summarize_mode_result,
    },
    provenance::{build_experiment_provenance, ExperimentCorpusSnapshot},
    retrieval::LocalHybridRetriever,
    RagError,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    config::{RuntimeEnvironment, StorageBackend},
    error::ApiError,
    state::AppState,
};

#[derive(Default)]
pub(crate) struct CiProvenanceMetadata {
    pub branch: Option<String>,
    pub commit_sha: Option<String>,
    pub base_ref: Option<String>,
    pub head_ref: Option<String>,
    pub labels: BTreeMap<String, String>,
}

pub(crate) async fn run_experiment_for_dataset(
    state: &AppState,
    workspace_id: WorkspaceId,
    dataset: RetrievalEvalDataset,
    modes: Vec<RetrievalMode>,
    top_k: u32,
    name: String,
    ci: CiProvenanceMetadata,
) -> Result<RetrievalEvalExperiment, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let corpus_snapshot = repository
        .retrieval_eval_corpus_snapshot(workspace_id)
        .await?;
    let sources = corpus_snapshot.sources;
    let candidates = corpus_snapshot.candidates;
    let provenance = build_experiment_provenance(
        workspace_id,
        &dataset,
        &modes,
        top_k,
        &state.config().product,
        ExperimentCorpusSnapshot {
            sources: &sources,
            candidates: &candidates,
        },
        provenance_information(state, ci),
    )
    .map_err(|error| {
        let correlation_id = Uuid::now_v7();
        tracing::error!(
            %correlation_id,
            workspace_id = %workspace_id.0,
            dataset_id = %dataset.id.0,
            %error,
            "failed to build eval experiment provenance"
        );
        ApiError::Internal
    })?;

    let provider = LocalHashEmbeddingProvider::new(state.config().product.embedding.model.clone());
    let retriever = LocalHybridRetriever::new(provider, state.config().product.retrieval.clone())
        .with_debugger_config(state.config().product.debugger.clone());
    let mut mode_results = Vec::with_capacity(modes.len());
    for mode in &modes {
        let mut case_results = Vec::with_capacity(dataset.cases.len());
        for eval_case in &dataset.cases {
            let case = rag_debugger_core::RetrievalEvalCase {
                top_k,
                ..eval_case.clone()
            };
            let query_request = RetrievalQueryRequest {
                query: case.query.clone(),
                top_k,
                retrieval_mode: *mode,
                source_ids: Vec::new(),
                document_ids: Vec::new(),
            };
            let expected_chunk_document_ids =
                expected_chunk_parent_document_ids(&case, &candidates);
            let response = retriever
                .retrieve(query_request, candidates.clone())
                .map_err(rag_error_to_api_error)?;
            case_results.push(evaluate_retrieval_eval_case_with_context(
                &case,
                &response,
                &state.config().product.debugger,
                &expected_chunk_document_ids,
            ));
        }
        mode_results.push(summarize_mode_result(*mode, case_results));
    }

    let comparison = compare_mode_results(&mode_results);
    let gate = evaluate_gate(&mode_results);
    let failures = mode_results
        .iter()
        .flat_map(|result| result.case_results.iter())
        .flat_map(|result| result.failures.iter().cloned())
        .collect();
    Ok(RetrievalEvalExperiment {
        id: RetrievalEvalExperimentId(Uuid::now_v7()),
        dataset_id: dataset.id,
        dataset_name: dataset.name,
        name,
        modes,
        top_k,
        config_snapshot: RetrievalEvalConfigSnapshot {
            top_k,
            scoring_weights: state.config().product.retrieval.weights.clone(),
            embedding_model: state.config().product.embedding.model.clone(),
            dataset_case_count: dataset.cases.len() as u32,
        },
        provenance: Some(provenance),
        mode_results,
        comparison,
        gate,
        failures,
        created_at: OffsetDateTime::now_utc(),
    })
}

fn provenance_information(
    state: &AppState,
    ci: CiProvenanceMetadata,
) -> RetrievalEvalProvenanceInformation {
    RetrievalEvalProvenanceInformation {
        application_version: env!("CARGO_PKG_VERSION").to_owned(),
        deployment_mode: match state.config().product.product.deployment_mode {
            DeploymentMode::Local => "local",
            DeploymentMode::Hosted => "hosted",
            DeploymentMode::Hybrid => "hybrid",
        }
        .to_owned(),
        runtime_environment: Some(
            match state.config().environment {
                RuntimeEnvironment::Local => "local",
                RuntimeEnvironment::Test => "test",
                RuntimeEnvironment::Production => "production",
            }
            .to_owned(),
        ),
        storage_backend: Some(
            match state.config().storage_backend {
                StorageBackend::Memory => "memory",
                StorageBackend::Postgres => "postgres",
            }
            .to_owned(),
        ),
        branch: ci.branch,
        commit_sha: ci.commit_sha,
        base_ref: ci.base_ref,
        head_ref: ci.head_ref,
        labels: ci.labels,
    }
}

fn rag_error_to_api_error(error: RagError) -> ApiError {
    match error {
        RagError::InvalidConfig(message) => ApiError::BadRequest(message.to_owned()),
        RagError::NotImplemented(_) => ApiError::Internal,
    }
}
