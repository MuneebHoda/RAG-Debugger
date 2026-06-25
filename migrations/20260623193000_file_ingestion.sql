CREATE TABLE projects (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    privacy_mode TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE sources (
    id UUID PRIMARY KEY,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    source_kind TEXT NOT NULL,
    root_hint TEXT,
    github_owner TEXT,
    github_repo TEXT,
    sync_policy TEXT NOT NULL,
    sync_cron TEXT,
    target_tokens INTEGER NOT NULL,
    overlap_tokens INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_sources_project_id ON sources(project_id);
CREATE INDEX idx_sources_created_at ON sources(created_at DESC);

CREATE TABLE ingestion_runs (
    id UUID PRIMARY KEY,
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    status TEXT NOT NULL,
    files_received INTEGER NOT NULL DEFAULT 0,
    documents_created INTEGER NOT NULL DEFAULT 0,
    chunks_created INTEGER NOT NULL DEFAULT 0,
    failed_files INTEGER NOT NULL DEFAULT 0,
    started_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ
);

CREATE INDEX idx_ingestion_runs_source_id ON ingestion_runs(source_id);

CREATE TABLE documents (
    id UUID PRIMARY KEY,
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    mime_type TEXT,
    checksum TEXT NOT NULL,
    byte_size BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_documents_source_id ON documents(source_id);
CREATE INDEX idx_documents_checksum ON documents(checksum);

CREATE TABLE chunks (
    id UUID PRIMARY KEY,
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL,
    text TEXT NOT NULL,
    token_count INTEGER NOT NULL,
    byte_start BIGINT NOT NULL,
    byte_end BIGINT NOT NULL,
    checksum TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    UNIQUE(document_id, ordinal)
);

CREATE INDEX idx_chunks_document_id ON chunks(document_id, ordinal);
CREATE INDEX idx_chunks_checksum ON chunks(checksum);

