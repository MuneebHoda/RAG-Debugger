ALTER TABLE retrieval_playground_runs
ADD COLUMN workspace_id UUID REFERENCES workspaces(id) ON DELETE CASCADE;

WITH ownership_signals AS (
    SELECT hit.run_id, project.workspace_id
    FROM retrieval_playground_hits hit
    INNER JOIN chunks chunk ON chunk.id = hit.chunk_id
    INNER JOIN sources source ON source.id = chunk.source_id
    INNER JOIN projects project ON project.id = source.project_id
    WHERE project.workspace_id IS NOT NULL
    UNION
    SELECT trace.source_run_id AS run_id, project.workspace_id
    FROM debug_traces trace
    INNER JOIN projects project ON project.id = trace.project_id
    WHERE trace.source_run_id IS NOT NULL
      AND project.workspace_id IS NOT NULL
),
unambiguous_owners AS (
    SELECT
        run_id,
        (array_agg(workspace_id ORDER BY workspace_id))[1] AS workspace_id
    FROM ownership_signals
    GROUP BY run_id
    HAVING COUNT(DISTINCT workspace_id) = 1
)
UPDATE retrieval_playground_runs run
SET workspace_id = owner.workspace_id
FROM unambiguous_owners owner
WHERE run.id = owner.run_id
  AND run.workspace_id IS NULL;

WITH sole_workspace AS (
    SELECT (array_agg(id ORDER BY id))[1] AS id
    FROM workspaces
    HAVING COUNT(*) = 1
)
UPDATE retrieval_playground_runs
SET workspace_id = sole_workspace.id
FROM sole_workspace
WHERE retrieval_playground_runs.workspace_id IS NULL;

ALTER TABLE debug_traces
ADD COLUMN workspace_id UUID REFERENCES workspaces(id) ON DELETE CASCADE;

WITH ownership_signals AS (
    SELECT trace.id AS trace_id, project.workspace_id
    FROM debug_traces trace
    INNER JOIN projects project ON project.id = trace.project_id
    WHERE project.workspace_id IS NOT NULL
    UNION
    SELECT trace.id AS trace_id, run.workspace_id
    FROM debug_traces trace
    INNER JOIN retrieval_playground_runs run ON run.id = trace.source_run_id
    WHERE run.workspace_id IS NOT NULL
),
unambiguous_owners AS (
    SELECT
        trace_id,
        (array_agg(workspace_id ORDER BY workspace_id))[1] AS workspace_id
    FROM ownership_signals
    GROUP BY trace_id
    HAVING COUNT(DISTINCT workspace_id) = 1
)
UPDATE debug_traces trace
SET workspace_id = owner.workspace_id
FROM unambiguous_owners owner
WHERE trace.id = owner.trace_id
  AND trace.workspace_id IS NULL;

WITH sole_workspace AS (
    SELECT (array_agg(id ORDER BY id))[1] AS id
    FROM workspaces
    HAVING COUNT(*) = 1
)
UPDATE debug_traces
SET workspace_id = sole_workspace.id
FROM sole_workspace
WHERE debug_traces.workspace_id IS NULL;

CREATE INDEX idx_retrieval_playground_runs_workspace_created
ON retrieval_playground_runs(workspace_id, created_at DESC)
WHERE workspace_id IS NOT NULL;

CREATE INDEX idx_debug_traces_workspace_created
ON debug_traces(workspace_id, created_at DESC)
WHERE workspace_id IS NOT NULL;
