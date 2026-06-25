use std::collections::VecDeque;

use rag_debugger_core::{
    ByteRange, Chunk, ChunkId, ChunkSplitReason, ChunkingConfig, ChunkingStrategy, Document,
};
use sha2::{Digest, Sha256};
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
        validate_config(config)?;
        Ok(chunk_by_whitespace(
            document,
            text,
            config,
            ChunkingStrategy::Whitespace,
            None,
            ChunkSplitReason::TokenLimit,
            ChunkSplitReason::DocumentEnd,
            0,
        ))
    }
}

#[derive(Debug, Default)]
pub struct SmartSectionChunker;

impl Chunker for SmartSectionChunker {
    fn chunk(
        &self,
        document: &Document,
        text: &str,
        config: ChunkingConfig,
    ) -> Result<Vec<Chunk>, RagError> {
        validate_config(config)?;

        let blocks = section_blocks(text);
        if blocks.is_empty() {
            return Ok(Vec::new());
        }

        let target = config.target_tokens as usize;
        let overlap = config.overlap_tokens as usize;
        let mut chunks = Vec::new();
        let mut pending = PendingChunk::default();

        for block in blocks {
            if block.token_count > target {
                if !pending.is_empty() {
                    let reason = if pending.same_section(&block) {
                        ChunkSplitReason::TokenLimit
                    } else {
                        ChunkSplitReason::SectionBoundary
                    };
                    push_pending_chunk(document, &pending, reason, &mut chunks);
                    pending = PendingChunk::default();
                }

                let fallback_chunks = chunk_by_whitespace(
                    document,
                    &block.text,
                    config,
                    ChunkingStrategy::Structured,
                    block.section_title.clone(),
                    ChunkSplitReason::FallbackWhitespace,
                    ChunkSplitReason::FallbackWhitespace,
                    block.byte_start,
                );
                push_reindexed_chunks(&mut chunks, fallback_chunks);
                continue;
            }

            if pending.is_empty() {
                pending.push(block);
                continue;
            }

            if pending.can_accept(&block, target) {
                pending.push(block);
                continue;
            }

            let split_reason = if pending.same_section(&block) {
                ChunkSplitReason::TokenLimit
            } else {
                ChunkSplitReason::SectionBoundary
            };
            let overlap_block =
                if split_reason == ChunkSplitReason::TokenLimit && pending.same_section(&block) {
                    overlap_block(&pending.blocks, overlap, pending.section_title.clone())
                } else {
                    None
                };

            push_pending_chunk(document, &pending, split_reason, &mut chunks);
            pending = PendingChunk::default();

            if let Some(overlap_block) = overlap_block {
                if overlap_block.token_count + block.token_count <= target {
                    pending.push(overlap_block);
                }
            }
            pending.push(block);
        }

        if !pending.is_empty() {
            push_pending_chunk(
                document,
                &pending,
                ChunkSplitReason::DocumentEnd,
                &mut chunks,
            );
        }

        Ok(chunks)
    }
}

fn validate_config(config: ChunkingConfig) -> Result<(), RagError> {
    if config.target_tokens == 0 {
        return Err(RagError::InvalidConfig(
            "target_tokens must be greater than zero",
        ));
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn chunk_by_whitespace(
    document: &Document,
    text: &str,
    config: ChunkingConfig,
    strategy: ChunkingStrategy,
    section_title: Option<String>,
    split_reason: ChunkSplitReason,
    final_split_reason: ChunkSplitReason,
    byte_offset: usize,
) -> Vec<Chunk> {
    let spans = token_spans(text);
    if spans.is_empty() {
        return Vec::new();
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
        let reason = if end == spans.len() {
            final_split_reason
        } else {
            split_reason
        };

        chunks.push(Chunk {
            id: ChunkId(Uuid::now_v7()),
            source_id: document.source_id,
            document_id: document.id,
            ordinal: chunks.len() as u32,
            token_count: (end - start) as u32,
            byte_range: ByteRange {
                start: (byte_offset + byte_start) as u64,
                end: (byte_offset + byte_end) as u64,
            },
            checksum: checksum_text(&chunk_text),
            text: chunk_text,
            strategy,
            section_title: section_title.clone(),
            split_reason: reason,
            quality_flags: Vec::new(),
            is_duplicate: false,
            text_density: 0.0,
            evidence_score_hint: 0.0,
        });

        start += step;
    }

    chunks
}

fn push_reindexed_chunks(chunks: &mut Vec<Chunk>, next_chunks: Vec<Chunk>) {
    for mut chunk in next_chunks {
        chunk.ordinal = chunks.len() as u32;
        chunks.push(chunk);
    }
}

fn push_pending_chunk(
    document: &Document,
    pending: &PendingChunk,
    split_reason: ChunkSplitReason,
    chunks: &mut Vec<Chunk>,
) {
    let text = pending.text();
    let byte_range = pending.byte_range();

    chunks.push(Chunk {
        id: ChunkId(Uuid::now_v7()),
        source_id: document.source_id,
        document_id: document.id,
        ordinal: chunks.len() as u32,
        token_count: token_spans(&text).len() as u32,
        byte_range: ByteRange {
            start: byte_range.start,
            end: byte_range.end,
        },
        checksum: checksum_text(&text),
        text,
        strategy: ChunkingStrategy::Structured,
        section_title: pending.section_title.clone(),
        split_reason,
        quality_flags: Vec::new(),
        is_duplicate: false,
        text_density: 0.0,
        evidence_score_hint: 0.0,
    });
}

fn checksum_text(text: &str) -> String {
    let digest = Sha256::digest(text.as_bytes());
    hex::encode(digest)
}

#[derive(Debug, Clone)]
struct TextBlock {
    section_title: Option<String>,
    text: String,
    byte_start: usize,
    byte_end: usize,
    token_count: usize,
}

impl TextBlock {
    fn from_range(
        source: &str,
        section_title: Option<String>,
        byte_start: usize,
        byte_end: usize,
    ) -> Option<Self> {
        let raw_text = source[byte_start..byte_end].trim();
        Self::from_text(section_title, raw_text.to_owned(), byte_start, byte_end)
    }

    fn from_text(
        section_title: Option<String>,
        text: String,
        byte_start: usize,
        byte_end: usize,
    ) -> Option<Self> {
        let token_count = token_spans(&text).len();
        if token_count == 0 {
            return None;
        }

        Some(Self {
            section_title,
            text,
            byte_start,
            byte_end,
            token_count,
        })
    }
}

#[derive(Debug, Default)]
struct PendingChunk {
    blocks: Vec<TextBlock>,
    section_title: Option<String>,
    token_count: usize,
}

impl PendingChunk {
    fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    fn same_section(&self, block: &TextBlock) -> bool {
        self.section_title == block.section_title
    }

    fn can_accept(&self, block: &TextBlock, target_tokens: usize) -> bool {
        self.same_section(block) && self.token_count + block.token_count <= target_tokens
    }

    fn push(&mut self, block: TextBlock) {
        if self.blocks.is_empty() {
            self.section_title = block.section_title.clone();
        }
        self.token_count += block.token_count;
        self.blocks.push(block);
    }

    fn text(&self) -> String {
        self.blocks
            .iter()
            .map(|block| block.text.as_str())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn byte_range(&self) -> ByteRange {
        let start = self
            .blocks
            .first()
            .map_or(0, |block| block.byte_start as u64);
        let end = self
            .blocks
            .last()
            .map_or(start, |block| block.byte_end as u64);

        ByteRange { start, end }
    }
}

#[derive(Debug)]
struct BlockBuilder {
    section_title: Option<String>,
    byte_start: usize,
    byte_end: usize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum BlockKind {
    BulletGroup,
    Paragraph,
}

#[derive(Debug)]
struct LineSpan<'a> {
    trimmed: &'a str,
    byte_start: usize,
    byte_end: usize,
}

fn section_blocks(text: &str) -> Vec<TextBlock> {
    let mut blocks = Vec::new();
    let mut current_section = None;
    let mut builder = None;
    let mut current_kind = None;

    for line in line_spans(text) {
        if line.trimmed.is_empty() {
            flush_block_builder(text, &mut builder, &mut blocks);
            current_kind = None;
            continue;
        }

        if let Some(section_title) = detect_section_heading(line.trimmed) {
            flush_block_builder(text, &mut builder, &mut blocks);
            current_kind = None;
            current_section = Some(section_title);

            if let Some(block) = TextBlock::from_range(
                text,
                current_section.clone(),
                line.byte_start,
                line.byte_end,
            ) {
                blocks.push(block);
            }
            continue;
        }

        let kind = if is_bullet_line(line.trimmed) {
            BlockKind::BulletGroup
        } else {
            BlockKind::Paragraph
        };

        if current_kind.is_some_and(|current_kind| current_kind != kind) {
            flush_block_builder(text, &mut builder, &mut blocks);
        }

        match builder.as_mut() {
            Some(builder) => builder.byte_end = line.byte_end,
            None => {
                builder = Some(BlockBuilder {
                    section_title: current_section.clone(),
                    byte_start: line.byte_start,
                    byte_end: line.byte_end,
                });
            }
        }
        current_kind = Some(kind);
    }

    flush_block_builder(text, &mut builder, &mut blocks);
    blocks
}

fn flush_block_builder(
    text: &str,
    builder: &mut Option<BlockBuilder>,
    blocks: &mut Vec<TextBlock>,
) {
    let Some(builder) = builder.take() else {
        return;
    };

    if let Some(block) = TextBlock::from_range(
        text,
        builder.section_title,
        builder.byte_start,
        builder.byte_end,
    ) {
        blocks.push(block);
    }
}

fn line_spans(text: &str) -> Vec<LineSpan<'_>> {
    let mut spans = Vec::new();
    let mut offset = 0usize;

    for segment in text.split_inclusive('\n') {
        let segment_start = offset;
        offset += segment.len();
        let line_body = segment.trim_end_matches(['\r', '\n']);
        let left_trimmed = line_body.trim_start();
        let leading = line_body.len() - left_trimmed.len();
        let trimmed = left_trimmed.trim_end();
        let byte_start = segment_start + leading;
        let byte_end = byte_start + trimmed.len();

        spans.push(LineSpan {
            trimmed,
            byte_start,
            byte_end,
        });
    }

    spans
}

fn detect_section_heading(line: &str) -> Option<String> {
    let trimmed = line.trim();
    common_section_title(trimmed).or_else(|| {
        if is_generic_heading(trimmed) {
            Some(trimmed.to_owned())
        } else {
            None
        }
    })
}

fn common_section_title(line: &str) -> Option<String> {
    let key = line
        .trim()
        .trim_end_matches(':')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    let title = match key.as_str() {
        "summary" | "professional summary" | "profile" | "objective" => "Summary",
        "experience"
        | "work experience"
        | "professional experience"
        | "employment"
        | "employment history" => "Experience",
        "projects" | "selected projects" | "personal projects" => "Projects",
        "education" | "academic background" => "Education",
        "skills" | "technical skills" | "core skills" | "technologies" => "Skills",
        "certification" | "certifications" | "licenses" => "Certifications",
        "awards" | "honors" => "Awards",
        "publications" => "Publications",
        "leadership" | "volunteer experience" | "volunteering" => "Leadership",
        _ => return None,
    };

    Some(title.to_owned())
}

fn is_generic_heading(line: &str) -> bool {
    if line.is_empty() || line.len() > 80 || is_bullet_line(line) || has_terminal_punctuation(line)
    {
        return false;
    }

    let words = line.split_whitespace().collect::<Vec<_>>();
    if words.is_empty() || words.len() > 8 {
        return false;
    }

    if is_uppercase_heading(line) {
        return true;
    }

    words.iter().all(|word| is_title_word(word))
}

fn has_terminal_punctuation(line: &str) -> bool {
    line.chars()
        .last()
        .is_some_and(|character| matches!(character, '.' | ',' | ';' | ':' | '!' | '?'))
}

fn is_uppercase_heading(line: &str) -> bool {
    let letters = line
        .chars()
        .filter(|character| character.is_alphabetic())
        .collect::<Vec<_>>();

    letters.len() >= 3 && letters.iter().all(|character| character.is_uppercase())
}

fn is_title_word(word: &str) -> bool {
    let trimmed = word.trim_matches(|character: char| {
        matches!(
            character,
            '(' | ')' | '[' | ']' | '{' | '}' | '/' | '&' | '-' | '_'
        )
    });

    if trimmed.is_empty() {
        return true;
    }

    let lower = trimmed.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "and" | "or" | "of" | "for" | "to" | "in" | "with"
    ) {
        return true;
    }

    trimmed
        .chars()
        .next()
        .is_some_and(|character| character.is_uppercase() || character.is_numeric())
}

fn is_bullet_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with("- ")
        || trimmed.starts_with("* ")
        || trimmed.starts_with("• ")
        || trimmed.starts_with("● ")
        || trimmed.starts_with("◦ ")
    {
        return true;
    }

    let mut chars = trimmed.chars().peekable();
    let mut saw_digit = false;
    while chars
        .peek()
        .is_some_and(|character| character.is_ascii_digit())
    {
        saw_digit = true;
        chars.next();
    }

    if !saw_digit {
        return false;
    }

    matches!(chars.next(), Some('.') | Some(')'))
        && chars
            .peek()
            .is_some_and(|character| character.is_whitespace())
}

fn overlap_block(
    blocks: &[TextBlock],
    overlap_tokens: usize,
    section_title: Option<String>,
) -> Option<TextBlock> {
    if overlap_tokens == 0 {
        return None;
    }

    let mut remaining = overlap_tokens;
    let mut slices = VecDeque::new();

    for block in blocks.iter().rev() {
        if remaining == 0 {
            break;
        }

        if block.token_count <= remaining {
            slices.push_front((
                block.text.clone(),
                block.byte_start,
                block.byte_end,
                block.token_count,
            ));
            remaining -= block.token_count;
            continue;
        }

        let spans = token_spans(&block.text);
        let start_index = spans.len().saturating_sub(remaining);
        let byte_start = spans[start_index].start;
        let byte_end = spans[spans.len() - 1].end;
        let text = block.text[byte_start..byte_end].to_owned();
        slices.push_front((
            text,
            block.byte_start + byte_start,
            block.byte_start + byte_end,
            remaining,
        ));
        remaining = 0;
    }

    let byte_start = slices.front()?.1;
    let byte_end = slices.back()?.2;
    let text = slices
        .into_iter()
        .map(|(text, _, _, _)| text)
        .collect::<Vec<_>>()
        .join("\n\n");

    TextBlock::from_text(section_title, text, byte_start, byte_end)
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
    use rag_debugger_core::{Document, DocumentId, DocumentProfile, ExtractionQuality, SourceId};
    use uuid::Uuid;

    use super::*;

    #[test]
    fn chunks_text_by_target_size() {
        let chunks = WhitespaceChunker
            .chunk(
                &document(),
                "one two three four five",
                ChunkingConfig {
                    target_tokens: 2,
                    overlap_tokens: 0,
                    strategy: ChunkingStrategy::Whitespace,
                },
            )
            .expect("chunks");

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].text, "one two");
        assert_eq!(chunks[0].strategy, ChunkingStrategy::Whitespace);
        assert_eq!(chunks[0].split_reason, ChunkSplitReason::TokenLimit);
    }

    #[test]
    fn detects_resume_headings_and_preserves_section_titles() {
        let text = "Summary\nBuilder of useful tools.\n\nExperience\n- Built RAG systems.\n\nProjects\n- Resume Debugger";
        let chunks = SmartSectionChunker
            .chunk(
                &document(),
                text,
                ChunkingConfig {
                    target_tokens: 20,
                    overlap_tokens: 0,
                    strategy: ChunkingStrategy::SmartSections,
                },
            )
            .expect("chunks");

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].section_title.as_deref(), Some("Summary"));
        assert_eq!(chunks[1].section_title.as_deref(), Some("Experience"));
        assert_eq!(chunks[2].section_title.as_deref(), Some("Projects"));
        assert_eq!(chunks[0].split_reason, ChunkSplitReason::SectionBoundary);
    }

    #[test]
    fn keeps_bullet_groups_together_when_they_fit() {
        let text = "Projects\n- Built upload flow\n- Added chunk preview\n- Wrote tests";
        let chunks = SmartSectionChunker
            .chunk(
                &document(),
                text,
                ChunkingConfig {
                    target_tokens: 20,
                    overlap_tokens: 0,
                    strategy: ChunkingStrategy::SmartSections,
                },
            )
            .expect("chunks");

        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].text.contains("- Built upload flow"));
        assert!(chunks[0].text.contains("- Wrote tests"));
        assert_eq!(chunks[0].section_title.as_deref(), Some("Projects"));
    }

    #[test]
    fn falls_back_to_whitespace_for_oversized_blocks() {
        let text = "Summary\none two three four five six seven eight";
        let chunks = SmartSectionChunker
            .chunk(
                &document(),
                text,
                ChunkingConfig {
                    target_tokens: 3,
                    overlap_tokens: 0,
                    strategy: ChunkingStrategy::SmartSections,
                },
            )
            .expect("chunks");

        assert!(chunks
            .iter()
            .any(|chunk| chunk.split_reason == ChunkSplitReason::FallbackWhitespace));
        assert!(chunks
            .iter()
            .all(|chunk| chunk.strategy == ChunkingStrategy::Structured));
    }

    #[test]
    fn applies_overlap_for_same_section_token_splits() {
        let text = "Experience\nFirst paragraph one two.\n\nSecond paragraph three four.";
        let chunks = SmartSectionChunker
            .chunk(
                &document(),
                text,
                ChunkingConfig {
                    target_tokens: 5,
                    overlap_tokens: 1,
                    strategy: ChunkingStrategy::SmartSections,
                },
            )
            .expect("chunks");

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].split_reason, ChunkSplitReason::TokenLimit);
        assert!(chunks[1].text.starts_with("two."));
    }

    #[test]
    fn produces_stable_checksums_for_same_text() {
        let first = WhitespaceChunker
            .chunk(
                &document(),
                "alpha beta",
                ChunkingConfig {
                    target_tokens: 10,
                    overlap_tokens: 0,
                    strategy: ChunkingStrategy::Whitespace,
                },
            )
            .expect("first");
        let second = WhitespaceChunker
            .chunk(
                &document(),
                "alpha beta",
                ChunkingConfig {
                    target_tokens: 10,
                    overlap_tokens: 0,
                    strategy: ChunkingStrategy::Whitespace,
                },
            )
            .expect("second");

        assert_eq!(first[0].checksum, second[0].checksum);
    }

    fn document() -> Document {
        Document {
            id: DocumentId(Uuid::now_v7()),
            source_id: SourceId(Uuid::now_v7()),
            path: "guide.md".to_owned(),
            mime_type: Some("text/markdown".to_owned()),
            checksum: "abc".to_owned(),
            byte_size: 23,
            profile: DocumentProfile::General,
            extraction_quality: ExtractionQuality::High,
            warnings: Vec::new(),
        }
    }
}
