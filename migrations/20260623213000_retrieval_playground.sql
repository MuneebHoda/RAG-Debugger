CREATE TABLE retrieval_playground_runs (
    id UUID PRIMARY KEY,
    query TEXT NOT NULL,
    top_k INTEGER NOT NULL,
    answer_status TEXT NOT NULL,
    answer_text TEXT NOT NULL,
    latency_ms BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_retrieval_playground_runs_created_at
ON retrieval_playground_runs(created_at DESC);

CREATE TABLE retrieval_playground_hits (
    id UUID PRIMARY KEY,
    run_id UUID NOT NULL REFERENCES retrieval_playground_runs(id) ON DELETE CASCADE,
    chunk_id UUID NOT NULL REFERENCES chunks(id) ON DELETE CASCADE,
    rank INTEGER NOT NULL,
    score REAL NOT NULL,
    lexical_score REAL NOT NULL,
    phrase_score REAL NOT NULL,
    section_score REAL NOT NULL,
    path_score REAL NOT NULL,
    metadata_score REAL NOT NULL,
    matched_terms TEXT NOT NULL,
    snippet TEXT NOT NULL,
    citation_label TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    UNIQUE(run_id, rank)
);

CREATE INDEX idx_retrieval_playground_hits_run_id
ON retrieval_playground_hits(run_id, rank);

CREATE INDEX idx_retrieval_playground_hits_chunk_id
ON retrieval_playground_hits(chunk_id);
