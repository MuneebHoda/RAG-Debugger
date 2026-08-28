ALTER TABLE retrieval_eval_cases
ADD COLUMN case_key TEXT;

WITH bases AS (
    SELECT
        id,
        dataset_id,
        COALESCE(
            NULLIF(
                TRIM(BOTH '-' FROM REGEXP_REPLACE(LOWER(name), '[^a-z0-9]+', '-', 'g')),
                ''
            ),
            'case'
        ) AS base_key,
        created_at
    FROM retrieval_eval_cases
), truncated AS (
    SELECT
        id,
        dataset_id,
        LEFT(base_key, 100) AS base_key,
        created_at
    FROM bases
), ranked AS (
    SELECT
        id,
        base_key,
        ROW_NUMBER() OVER (
            PARTITION BY dataset_id, base_key
            ORDER BY created_at, id
        ) AS occurrence
    FROM truncated
)
UPDATE retrieval_eval_cases AS eval_case
SET case_key = CASE
    WHEN ranked.occurrence = 1 THEN ranked.base_key
    ELSE ranked.base_key || '-' || ranked.occurrence::TEXT
END
FROM ranked
WHERE eval_case.id = ranked.id;

ALTER TABLE retrieval_eval_cases
ALTER COLUMN case_key SET NOT NULL;

ALTER TABLE retrieval_eval_cases
ADD CONSTRAINT retrieval_eval_cases_case_key_format_check
CHECK (case_key ~ '^[a-z0-9][a-z0-9._-]{0,127}$');

CREATE UNIQUE INDEX idx_retrieval_eval_cases_dataset_case_key
ON retrieval_eval_cases(dataset_id, case_key);
