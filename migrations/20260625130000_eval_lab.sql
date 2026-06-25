CREATE TABLE retrieval_eval_datasets (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

INSERT INTO retrieval_eval_datasets (id, name, description, created_at, updated_at)
VALUES (
    '018f7a2a-6e2e-7000-a000-00000000e001',
    'Default retrieval dataset',
    'Backfilled and manually saved retrieval eval cases.',
    NOW(),
    NOW()
)
ON CONFLICT (id) DO NOTHING;

ALTER TABLE retrieval_eval_cases
ADD COLUMN dataset_id UUID REFERENCES retrieval_eval_datasets(id) ON DELETE SET NULL;

UPDATE retrieval_eval_cases
SET dataset_id = '018f7a2a-6e2e-7000-a000-00000000e001'
WHERE dataset_id IS NULL;

CREATE INDEX idx_retrieval_eval_cases_dataset_id
ON retrieval_eval_cases(dataset_id, created_at DESC);

CREATE TABLE retrieval_eval_experiments (
    id UUID PRIMARY KEY,
    dataset_id UUID NOT NULL REFERENCES retrieval_eval_datasets(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    modes TEXT[] NOT NULL,
    top_k INTEGER NOT NULL,
    best_mode TEXT,
    gate_status TEXT NOT NULL,
    average_recall_at_k REAL NOT NULL,
    average_precision_at_k REAL NOT NULL,
    failure_count INTEGER NOT NULL,
    experiment_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_retrieval_eval_experiments_dataset_id
ON retrieval_eval_experiments(dataset_id, created_at DESC);

CREATE INDEX idx_retrieval_eval_experiments_created_at
ON retrieval_eval_experiments(created_at DESC);
