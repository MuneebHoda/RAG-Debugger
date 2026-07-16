CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE INDEX idx_sources_name_trgm
ON sources USING GIN (lower(name) gin_trgm_ops);

CREATE INDEX idx_documents_path_trgm
ON documents USING GIN (lower(path) gin_trgm_ops);

CREATE INDEX idx_chunks_section_title_trgm
ON chunks USING GIN (lower(COALESCE(section_title, '')) gin_trgm_ops);

CREATE INDEX idx_chunks_text_trgm
ON chunks USING GIN (lower(text) gin_trgm_ops);
