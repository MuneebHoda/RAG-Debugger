ALTER TABLE retrieval_eval_cases
ADD COLUMN source_trace_id UUID REFERENCES debug_traces(id) ON DELETE RESTRICT,
ADD COLUMN source_ingestion_source TEXT,
ADD COLUMN source_privacy_mode TEXT;

ALTER TABLE retrieval_eval_cases
ADD CONSTRAINT retrieval_eval_cases_source_ingestion_source_check
CHECK (source_ingestion_source IS NULL OR source_ingestion_source IN ('native', 'otlp_http')),
ADD CONSTRAINT retrieval_eval_cases_source_privacy_mode_check
CHECK (source_privacy_mode IS NULL OR source_privacy_mode IN ('metadata_only', 'snippets_allowed', 'full_local_only')),
ADD CONSTRAINT retrieval_eval_cases_source_provenance_complete_check
CHECK (
    (source_trace_id IS NULL AND source_ingestion_source IS NULL AND source_privacy_mode IS NULL)
    OR
    (source_trace_id IS NOT NULL AND source_ingestion_source IS NOT NULL AND source_privacy_mode IS NOT NULL)
);

CREATE INDEX idx_retrieval_eval_cases_source_trace_id
ON retrieval_eval_cases(source_trace_id)
WHERE source_trace_id IS NOT NULL;
