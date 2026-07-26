#![allow(dead_code)]

use std::sync::Arc;

use axum::{
    extract::State,
    http::{header, HeaderValue, Request},
    middleware::{self, Next},
    response::Response,
    Router,
};
use rag_debugger_api::{
    app, auth,
    config::{ApiConfig, RuntimeEnvironment, StorageBackend},
    state::AppState,
};
use rag_debugger_core::{
    Organization, OrganizationId, ProductConfig, User, UserId, Workspace, WorkspaceId,
    WorkspaceRole,
};
use rag_debugger_storage::{
    memory::MemoryStore,
    repository::{AuthRepository, ProjectRepository},
};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct AuthenticatedTestApp {
    pub router: Router,
    #[allow(dead_code)]
    pub store: Arc<MemoryStore>,
    #[allow(dead_code)]
    pub workspace_id: WorkspaceId,
}

pub async fn authenticated_test_app() -> AuthenticatedTestApp {
    let store = Arc::new(MemoryStore::default());
    let config = test_config();
    let now = OffsetDateTime::now_utc();
    let organization = Organization {
        id: OrganizationId(Uuid::now_v7()),
        name: "CorpusLab API tests".to_owned(),
        created_at: now,
    };
    let workspace = Workspace {
        id: WorkspaceId(Uuid::now_v7()),
        organization_id: organization.id,
        name: "CorpusLab API test workspace".to_owned(),
        created_at: now,
    };
    let user = User {
        id: UserId(Uuid::now_v7()),
        email: format!("api-test-{}@example.test", Uuid::now_v7()),
        name: "API Test User".to_owned(),
        created_at: now,
    };
    let authenticated = store
        .create_user_workspace(
            organization,
            workspace.clone(),
            user,
            WorkspaceRole::Owner,
            "unused-test-password-hash".to_owned(),
        )
        .await
        .expect("create authenticated test workspace");
    store
        .ensure_default_project(workspace.id)
        .await
        .expect("create authenticated test project");
    let cookie = auth::create_session(store.as_ref(), &authenticated, &config.auth)
        .await
        .expect("create authenticated test session");
    let cookie = HeaderValue::from_str(&format!("{}={}", cookie.name, cookie.value))
        .expect("valid test cookie");
    let router = app(AppState::new(config, store.clone())).layer(middleware::from_fn_with_state(
        cookie,
        inject_session_cookie,
    ));

    AuthenticatedTestApp {
        router,
        store,
        workspace_id: workspace.id,
    }
}

pub fn test_config() -> ApiConfig {
    ApiConfig {
        environment: RuntimeEnvironment::Test,
        bind_addr: "127.0.0.1:0".parse().expect("valid test socket"),
        storage_backend: StorageBackend::Memory,
        database_url: "postgres://postgres:postgres@localhost:5432/rag_debugger_test".to_owned(),
        web_origin: "http://127.0.0.1:5173".to_owned(),
        auth: Default::default(),
        product: ProductConfig::default(),
    }
}

async fn inject_session_cookie(
    State(cookie): State<HeaderValue>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    request.headers_mut().insert(header::COOKIE, cookie);
    next.run(request).await
}
