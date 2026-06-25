mod config;
mod embeddings;
mod eval_lab;
mod evals;
mod health;
mod overview;
mod retrieval;
mod sources;
mod traces;

use axum::{
    extract::DefaultBodyLimit,
    http::{header, HeaderValue, Method},
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    let cors_layer = cors_layer(&state);
    let max_request_bytes = state.config().product.ingestion.max_request_bytes as usize;

    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/readyz", get(health::readyz))
        .nest("/api/v1", api_v1())
        .with_state(state)
        .layer(DefaultBodyLimit::max(max_request_bytes))
        .layer(cors_layer)
}

fn api_v1() -> Router<AppState> {
    Router::new()
        .route("/health", get(health::healthz))
        .route("/config", get(config::get_config))
        .route("/overview", get(overview::get_overview))
        .route("/embeddings/status", get(embeddings::embedding_status))
        .route("/embeddings/index", post(embeddings::index_embeddings))
        .route("/retrieval/query", post(retrieval::query_retrieval))
        .route(
            "/retrieval/evals",
            get(evals::list_retrieval_eval_cases).post(evals::create_retrieval_eval_case),
        )
        .route("/retrieval/evals/run", post(evals::run_retrieval_evals))
        .route(
            "/eval-lab/datasets",
            get(eval_lab::list_datasets).post(eval_lab::create_dataset),
        )
        .route("/eval-lab/datasets/:dataset_id", get(eval_lab::get_dataset))
        .route(
            "/eval-lab/datasets/:dataset_id/cases",
            post(eval_lab::create_case),
        )
        .route(
            "/eval-lab/cases/:case_id",
            axum::routing::patch(eval_lab::update_case).delete(eval_lab::delete_case),
        )
        .route(
            "/eval-lab/experiments",
            get(eval_lab::list_experiments).post(eval_lab::run_experiment),
        )
        .route(
            "/eval-lab/experiments/:experiment_id",
            get(eval_lab::get_experiment),
        )
        .route(
            "/eval-lab/experiments/:experiment_id/compare",
            post(eval_lab::compare_experiment),
        )
        .route("/traces", get(traces::list_traces))
        .route(
            "/traces/from-retrieval-run",
            post(traces::create_trace_from_retrieval_run),
        )
        .route("/traces/:trace_id", get(traces::get_trace))
        .route("/traces/:trace_id/rerun", post(traces::rerun_trace))
        .route("/sources", get(sources::list_sources))
        .route("/sources/files", post(sources::ingest_files))
        .route(
            "/documents/:document_id/chunks",
            get(sources::list_document_chunks),
        )
}

fn cors_layer(state: &AppState) -> CorsLayer {
    let origin = HeaderValue::from_str(&state.config().web_origin)
        .unwrap_or_else(|_| HeaderValue::from_static("http://127.0.0.1:5173"));

    CorsLayer::new()
        .allow_origin(origin)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE])
}
