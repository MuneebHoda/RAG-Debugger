ALTER TABLE retrieval_eval_datasets
ADD COLUMN workspace_id UUID REFERENCES workspaces(id) ON DELETE CASCADE;

ALTER TABLE retrieval_eval_datasets
ADD COLUMN is_default BOOLEAN NOT NULL DEFAULT FALSE;

UPDATE retrieval_eval_datasets
SET is_default = TRUE
WHERE id = '018f7a2a-6e2e-7000-a000-00000000e001';

WITH ownership_signals AS (
    SELECT dataset_id, workspace_id
    FROM ci_eval_runs
    UNION
    SELECT c.dataset_id, p.workspace_id
    FROM retrieval_eval_cases c
    CROSS JOIN LATERAL unnest(c.expected_document_ids) AS expected(document_id)
    INNER JOIN documents d ON d.id = expected.document_id
    INNER JOIN sources s ON s.id = d.source_id
    INNER JOIN projects p ON p.id = s.project_id
    WHERE c.dataset_id IS NOT NULL AND p.workspace_id IS NOT NULL
    UNION
    SELECT c.dataset_id, p.workspace_id
    FROM retrieval_eval_cases c
    CROSS JOIN LATERAL unnest(c.expected_chunk_ids) AS expected(chunk_id)
    INNER JOIN chunks chunk ON chunk.id = expected.chunk_id
    INNER JOIN sources s ON s.id = chunk.source_id
    INNER JOIN projects p ON p.id = s.project_id
    WHERE c.dataset_id IS NOT NULL AND p.workspace_id IS NOT NULL
),
unambiguous_owners AS (
    SELECT dataset_id, (array_agg(workspace_id ORDER BY workspace_id))[1] AS workspace_id
    FROM ownership_signals
    GROUP BY dataset_id
    HAVING COUNT(DISTINCT workspace_id) = 1
)
UPDATE retrieval_eval_datasets dataset
SET workspace_id = owner.workspace_id
FROM unambiguous_owners owner
WHERE dataset.id = owner.dataset_id
  AND dataset.workspace_id IS NULL;

WITH sole_workspace AS (
    SELECT (array_agg(id ORDER BY id))[1] AS id
    FROM workspaces
    HAVING COUNT(*) = 1
)
UPDATE projects
SET workspace_id = sole_workspace.id
FROM sole_workspace
WHERE projects.workspace_id IS NULL;

WITH sole_workspace AS (
    SELECT (array_agg(id ORDER BY id))[1] AS id
    FROM workspaces
    HAVING COUNT(*) = 1
)
UPDATE retrieval_eval_datasets
SET workspace_id = sole_workspace.id
FROM sole_workspace
WHERE retrieval_eval_datasets.workspace_id IS NULL;

ALTER TABLE retrieval_eval_runs
ADD COLUMN workspace_id UUID REFERENCES workspaces(id) ON DELETE CASCADE;

WITH run_ownership AS (
    SELECT
        result.run_id,
        (array_agg(dataset.workspace_id ORDER BY dataset.workspace_id))[1] AS workspace_id
    FROM retrieval_eval_results result
    INNER JOIN retrieval_eval_cases eval_case ON eval_case.id = result.case_id
    INNER JOIN retrieval_eval_datasets dataset ON dataset.id = eval_case.dataset_id
    WHERE dataset.workspace_id IS NOT NULL
    GROUP BY result.run_id
    HAVING COUNT(DISTINCT dataset.workspace_id) = 1
)
UPDATE retrieval_eval_runs run
SET workspace_id = owner.workspace_id
FROM run_ownership owner
WHERE run.id = owner.run_id
  AND run.workspace_id IS NULL;

WITH sole_workspace AS (
    SELECT (array_agg(id ORDER BY id))[1] AS id
    FROM workspaces
    HAVING COUNT(*) = 1
)
UPDATE retrieval_eval_runs
SET workspace_id = sole_workspace.id
FROM sole_workspace
WHERE retrieval_eval_runs.workspace_id IS NULL;

CREATE INDEX idx_projects_workspace_created
ON projects(workspace_id, created_at ASC)
WHERE workspace_id IS NOT NULL;

CREATE INDEX idx_retrieval_eval_datasets_workspace_updated
ON retrieval_eval_datasets(workspace_id, updated_at DESC)
WHERE workspace_id IS NOT NULL;

CREATE UNIQUE INDEX idx_retrieval_eval_datasets_workspace_default
ON retrieval_eval_datasets(workspace_id)
WHERE workspace_id IS NOT NULL AND is_default;

CREATE INDEX idx_retrieval_eval_runs_workspace_created
ON retrieval_eval_runs(workspace_id, created_at DESC)
WHERE workspace_id IS NOT NULL;
