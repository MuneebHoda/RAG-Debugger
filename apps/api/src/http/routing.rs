use axum::{
    extract::DefaultBodyLimit,
    http::{header, HeaderMap, HeaderValue, Method, Request},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;

use super::{
    api_keys, auth_routes, ci_eval, config, embeddings, eval_lab, evals, health, overview, reports,
    retrieval, sources, traces,
};
use crate::{auth, config::RuntimeEnvironment, error::ApiError, state::AppState};

pub fn router(state: AppState) -> Router {
    let cors_layer = cors_layer(&state);
    let max_request_bytes = state.config().product.ingestion.max_request_bytes as usize;

    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/readyz", get(health::readyz))
        .nest("/api/v1", api_v1(state.clone()))
        .with_state(state)
        .layer(DefaultBodyLimit::max(max_request_bytes))
        .layer(cors_layer)
}

fn api_v1(state: AppState) -> Router<AppState> {
    public_routes().merge(protected_routes(state))
}

fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health::healthz))
        .route("/config", get(config::get_config))
        .route("/auth/signup", post(auth_routes::signup))
        .route("/auth/login", post(auth_routes::login))
        .route("/auth/logout", post(auth_routes::logout))
        .route("/auth/me", get(auth_routes::me))
        .route(
            "/eval-lab/ci/runs",
            get(ci_eval::list_ci_eval_runs).post(ci_eval::run_ci_eval),
        )
        .route("/eval-lab/ci/runs/:run_id", get(ci_eval::get_ci_eval_run))
        .route(
            "/eval-lab/ci/runs/:run_id/report",
            get(ci_eval::get_ci_eval_report),
        )
}

fn protected_routes(state: AppState) -> Router<AppState> {
    Router::new()
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
        .route("/reports", get(reports::list_reports))
        .route(
            "/reports/from-trace",
            post(reports::create_report_from_trace),
        )
        .route(
            "/reports/from-experiment",
            post(reports::create_report_from_experiment),
        )
        .route(
            "/reports/from-ci-run",
            post(reports::create_report_from_ci_run),
        )
        .route("/reports/:report_id", get(reports::get_report))
        .route(
            "/reports/:report_id/export.md",
            get(reports::export_report_markdown),
        )
        .route("/sources", get(sources::list_sources))
        .route("/sources/files", post(sources::ingest_files))
        .route(
            "/documents/:document_id/chunks",
            get(sources::list_document_chunks),
        )
        .route("/workspaces/current", get(auth_routes::current_workspace))
        .route(
            "/api-keys",
            get(api_keys::list_api_keys).post(api_keys::create_api_key),
        )
        .route(
            "/api-keys/:api_key_id",
            axum::routing::delete(api_keys::revoke_api_key),
        )
        .route_layer(middleware::from_fn_with_state(state, require_session))
}

async fn require_session(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: HeaderMap,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, ApiError> {
    if matches!(state.config().environment, RuntimeEnvironment::Test) {
        return Ok(next.run(request).await);
    }
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    auth::authenticate_session(repository.as_ref(), &headers, &state.config().auth).await?;
    Ok(next.run(request).await)
}

fn cors_layer(state: &AppState) -> CorsLayer {
    let origin = HeaderValue::from_str(&state.config().web_origin)
        .unwrap_or_else(|_| HeaderValue::from_static("http://127.0.0.1:5173"));

    CorsLayer::new()
        .allow_origin(origin)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_credentials(true)
}
