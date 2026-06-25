CREATE TABLE chunk_embeddings (
    chunk_id UUID PRIMARY KEY REFERENCES chunks(id) ON DELETE CASCADE,
    model_provider TEXT NOT NULL,
    model_name TEXT NOT NULL,
    dimension INTEGER NOT NULL,
    vector REAL[] NOT NULL,
    chunk_checksum TEXT NOT NULL,
    indexed_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_chunk_embeddings_model
ON chunk_embeddings(model_provider, model_name);

ALTER TABLE retrieval_playground_runs
ADD COLUMN retrieval_mode TEXT NOT NULL DEFAULT 'hybrid';

ALTER TABLE retrieval_playground_hits
ADD COLUMN semantic_score REAL NOT NULL DEFAULT 0;

CREATE TABLE retrieval_eval_cases (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    query TEXT NOT NULL,
    top_k INTEGER NOT NULL,
    expected_chunk_ids UUID[] NOT NULL DEFAULT '{}',
    expected_document_ids UUID[] NOT NULL DEFAULT '{}',
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_retrieval_eval_cases_created_at
ON retrieval_eval_cases(created_at DESC);

CREATE TABLE retrieval_eval_runs (
    id UUID PRIMARY KEY,
    retrieval_mode TEXT NOT NULL,
    case_count INTEGER NOT NULL,
    passed_count INTEGER NOT NULL,
    average_recall_at_k REAL NOT NULL,
    average_precision_at_k REAL NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_retrieval_eval_runs_created_at
ON retrieval_eval_runs(created_at DESC);

CREATE TABLE retrieval_eval_results (
    id UUID PRIMARY KEY,
    run_id UUID NOT NULL REFERENCES retrieval_eval_runs(id) ON DELETE CASCADE,
    case_id UUID NOT NULL REFERENCES retrieval_eval_cases(id) ON DELETE CASCADE,
    query TEXT NOT NULL,
    top_k INTEGER NOT NULL,
    recall_at_k REAL NOT NULL,
    precision_at_k REAL NOT NULL,
    top_hit_rank INTEGER,
    passed BOOLEAN NOT NULL,
    expected_chunk_ids UUID[] NOT NULL DEFAULT '{}',
    expected_document_ids UUID[] NOT NULL DEFAULT '{}',
    retrieved_chunk_ids UUID[] NOT NULL DEFAULT '{}',
    latency_ms BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_retrieval_eval_results_run_id
ON retrieval_eval_results(run_id);
