ALTER TABLE sources
ADD COLUMN chunking_strategy TEXT NOT NULL DEFAULT 'whitespace';

ALTER TABLE chunks
ADD COLUMN strategy TEXT NOT NULL DEFAULT 'whitespace';

ALTER TABLE chunks
ADD COLUMN section_title TEXT;

ALTER TABLE chunks
ADD COLUMN split_reason TEXT NOT NULL DEFAULT 'token_limit';
