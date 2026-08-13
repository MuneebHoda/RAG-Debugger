ALTER TABLE debug_traces
DROP CONSTRAINT debug_traces_ingestion_identity_check;

ALTER TABLE debug_traces
ADD CONSTRAINT debug_traces_ingestion_identity_check
CHECK (
    (
        ingestion_source IS NULL
        AND external_trace_id IS NULL
        AND ingestion_schema_version IS NULL
        AND ingestion_mapper_version IS NULL
        AND ingestion_mapping_status IS NULL
        AND ingestion_privacy_mode IS NULL
    )
    OR
    (
        ingestion_source IS NOT NULL
        AND external_trace_id IS NOT NULL
        AND ingestion_schema_version IS NOT NULL
        AND ingestion_mapper_version IS NOT NULL
        AND ingestion_mapping_status IS NOT NULL
        AND ingestion_privacy_mode IS NOT NULL
    )
);
