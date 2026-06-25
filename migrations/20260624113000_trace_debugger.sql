ALTER TABLE retrieval_playground_runs
ADD COLUMN response_json JSONB;

CREATE TABLE debug_traces (
    id UUID PRIMARY KEY,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    source_run_id UUID REFERENCES retrieval_playground_runs(id) ON DELETE SET NULL,
    query TEXT NOT NULL,
    retrieval_mode TEXT NOT NULL,
    summary TEXT NOT NULL,
    status TEXT NOT NULL,
    evidence_strength TEXT NOT NULL,
    failure_labels TEXT[] NOT NULL DEFAULT '{}',
    span_count INTEGER NOT NULL,
    rerun_count INTEGER NOT NULL,
    latency_ms BIGINT NOT NULL,
    trace_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_debug_traces_created_at
ON debug_traces(created_at DESC);

CREATE INDEX idx_debug_traces_project_id
ON debug_traces(project_id, created_at DESC);

CREATE TABLE trace_rerun_experiments (
    id UUID PRIMARY KEY,
    trace_id UUID NOT NULL REFERENCES debug_traces(id) ON DELETE CASCADE,
    retrieval_mode TEXT NOT NULL,
    top_k INTEGER NOT NULL,
    score_delta REAL NOT NULL,
    latency_delta_ms BIGINT NOT NULL,
    overlap_count INTEGER NOT NULL,
    changed_rank_count INTEGER NOT NULL,
    comparison_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_trace_rerun_experiments_trace_id
ON trace_rerun_experiments(trace_id, created_at DESC);
