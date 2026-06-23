use rag_debugger_core::{ByteRange, Chunk, ChunkId, ChunkingConfig, Document};
use uuid::Uuid;

use crate::RagError;

pub trait Chunker: Send + Sync {
    fn chunk(
        &self,
        document: &Document,
        text: &str,
        config: ChunkingConfig,
    ) -> Result<Vec<Chunk>, RagError>;
}

#[derive(Debug, Default)]
pub struct WhitespaceChunker;

impl Chunker for WhitespaceChunker {
    fn chunk(
        &self,
        document: &Document,
        text: &str,
        config: ChunkingConfig,
    ) -> Result<Vec<Chunk>, RagError> {
        if config.target_tokens == 0 {
            return Err(RagError::InvalidConfig(
                "target_tokens must be greater than zero",
            ));
        }

        let spans = token_spans(text);
        if spans.is_empty() {
            return Ok(Vec::new());
        }

        let step = config
            .target_tokens
            .saturating_sub(config.overlap_tokens)
            .max(1) as usize;
        let target = config.target_tokens as usize;

        let mut chunks = Vec::new();
        let mut start = 0usize;

        while start < spans.len() {
            let end = (start + target).min(spans.len());
            let byte_start = spans[start].start;
            let byte_end = spans[end - 1].end;
            let chunk_text = text[byte_start..byte_end].to_owned();

            chunks.push(Chunk {
                id: ChunkId(Uuid::now_v7()),
                source_id: document.source_id,
                document_id: document.id,
                ordinal: chunks.len() as u32,
                token_count: (end - start) as u32,
                byte_range: ByteRange {
                    start: byte_start as u64,
                    end: byte_end as u64,
                },
                checksum: format!("{}:{}:{}", document.checksum, byte_start, byte_end),
                text: chunk_text,
            });

            start += step;
        }

        Ok(chunks)
    }
}

#[derive(Debug, Clone, Copy)]
struct TokenSpan {
    start: usize,
    end: usize,
}

fn token_spans(text: &str) -> Vec<TokenSpan> {
    let mut spans = Vec::new();
    let mut token_start = None;

    for (index, character) in text.char_indices() {
        if character.is_whitespace() {
            if let Some(start) = token_start.take() {
                spans.push(TokenSpan { start, end: index });
            }
        } else if token_start.is_none() {
            token_start = Some(index);
        }
    }

    if let Some(start) = token_start {
        spans.push(TokenSpan {
            start,
            end: text.len(),
        });
    }

    spans
}

#[cfg(test)]
mod tests {
    use rag_debugger_core::{ChunkingConfig, Document, DocumentId, SourceId};
    use uuid::Uuid;

    use super::*;

    #[test]
    fn chunks_text_by_target_size() {
        let document = Document {
            id: DocumentId(Uuid::now_v7()),
            source_id: SourceId(Uuid::now_v7()),
            path: "guide.md".to_owned(),
            mime_type: Some("text/markdown".to_owned()),
            checksum: "abc".to_owned(),
        };

        let chunks = WhitespaceChunker
            .chunk(
                &document,
                "one two three four five",
                ChunkingConfig {
                    target_tokens: 2,
                    overlap_tokens: 0,
                },
            )
            .expect("chunks");

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].text, "one two");
    }
}
