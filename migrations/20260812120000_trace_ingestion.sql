ALTER TABLE debug_traces
ADD COLUMN ingestion_source TEXT,
ADD COLUMN external_trace_id TEXT,
ADD COLUMN ingestion_schema_version TEXT,
ADD COLUMN ingestion_mapper_version TEXT,
ADD COLUMN ingestion_mapping_status TEXT,
ADD COLUMN ingestion_privacy_mode TEXT;

ALTER TABLE debug_traces
ADD CONSTRAINT debug_traces_ingestion_source_check
CHECK (ingestion_source IS NULL OR ingestion_source IN ('native', 'otlp_http')),
ADD CONSTRAINT debug_traces_ingestion_mapping_status_check
CHECK (ingestion_mapping_status IS NULL OR ingestion_mapping_status IN ('complete', 'partially_mapped')),
ADD CONSTRAINT debug_traces_ingestion_privacy_mode_check
CHECK (ingestion_privacy_mode IS NULL OR ingestion_privacy_mode IN ('metadata_only', 'snippets_allowed', 'full_local_only')),
ADD CONSTRAINT debug_traces_ingestion_identity_check
CHECK (
    (ingestion_source IS NULL AND external_trace_id IS NULL)
    OR
    (ingestion_source IS NOT NULL AND external_trace_id IS NOT NULL
        AND ingestion_schema_version IS NOT NULL
        AND ingestion_mapper_version IS NOT NULL
        AND ingestion_mapping_status IS NOT NULL
        AND ingestion_privacy_mode IS NOT NULL)
);

CREATE UNIQUE INDEX idx_debug_traces_import_identity
ON debug_traces(workspace_id, project_id, ingestion_source, external_trace_id)
WHERE ingestion_source IS NOT NULL;
