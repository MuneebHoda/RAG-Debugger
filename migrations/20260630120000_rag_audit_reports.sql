CREATE TABLE debug_reports (
    id UUID PRIMARY KEY,
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    source_type TEXT NOT NULL CHECK (
        source_type IN ('trace', 'eval_experiment', 'ci_eval_run', 'manual')
    ),
    source_id UUID,
    privacy_mode TEXT NOT NULL CHECK (
        privacy_mode IN ('metadata_only', 'snippets_allowed', 'full_local_only')
    ),
    title TEXT NOT NULL,
    subject TEXT NOT NULL,
    report_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_debug_reports_workspace_created
ON debug_reports(workspace_id, created_at DESC);

CREATE INDEX idx_debug_reports_source
ON debug_reports(source_type, source_id)
WHERE source_id IS NOT NULL;
