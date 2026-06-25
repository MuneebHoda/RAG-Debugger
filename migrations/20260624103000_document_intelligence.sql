ALTER TABLE documents
ADD COLUMN document_profile TEXT NOT NULL DEFAULT 'general';

ALTER TABLE documents
ADD COLUMN extraction_quality TEXT NOT NULL DEFAULT 'unknown';

ALTER TABLE documents
ADD COLUMN warnings TEXT[] NOT NULL DEFAULT '{}';

ALTER TABLE chunks
ADD COLUMN quality_flags TEXT[] NOT NULL DEFAULT '{}';

ALTER TABLE chunks
ADD COLUMN is_duplicate BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE chunks
ADD COLUMN text_density REAL NOT NULL DEFAULT 0;

ALTER TABLE chunks
ADD COLUMN evidence_score_hint REAL NOT NULL DEFAULT 0;

CREATE INDEX idx_documents_profile
ON documents(document_profile);

CREATE INDEX idx_chunks_quality_flags
ON chunks USING GIN(quality_flags);
