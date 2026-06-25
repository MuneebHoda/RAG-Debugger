CREATE TABLE organizations (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE workspaces (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE users (
    id UUID PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE workspace_memberships (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (user_id, workspace_id)
);

CREATE TABLE auth_sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ
);

CREATE INDEX idx_auth_sessions_token_hash
ON auth_sessions(token_hash);

CREATE TABLE api_keys (
    id UUID PRIMARY KEY,
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    prefix TEXT NOT NULL,
    secret_hash TEXT NOT NULL UNIQUE,
    scopes TEXT[] NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    last_used_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ
);

CREATE INDEX idx_api_keys_workspace_id
ON api_keys(workspace_id, created_at DESC);

CREATE TABLE ci_eval_runs (
    id UUID PRIMARY KEY,
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    dataset_id UUID NOT NULL REFERENCES retrieval_eval_datasets(id) ON DELETE CASCADE,
    dataset_name TEXT NOT NULL,
    experiment_id UUID NOT NULL REFERENCES retrieval_eval_experiments(id) ON DELETE CASCADE,
    status TEXT NOT NULL,
    gate_status TEXT NOT NULL,
    branch TEXT,
    commit_sha TEXT,
    base_ref TEXT,
    head_ref TEXT,
    config_label TEXT NOT NULL,
    run_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_ci_eval_runs_dataset_config
ON ci_eval_runs(dataset_id, config_label, created_at DESC);

CREATE INDEX idx_ci_eval_runs_workspace
ON ci_eval_runs(workspace_id, created_at DESC);

ALTER TABLE projects
ADD COLUMN workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL;
