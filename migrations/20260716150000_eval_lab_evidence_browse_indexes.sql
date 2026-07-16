CREATE INDEX idx_documents_evidence_browse
ON documents ((lower(path) COLLATE "C"), id);

CREATE INDEX idx_documents_source_evidence_order
ON documents (source_id, (lower(path) COLLATE "C"), id);

CREATE INDEX idx_chunks_evidence_browse
ON chunks (document_id, ordinal, id);
