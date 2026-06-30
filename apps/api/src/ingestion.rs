use rag_debugger_core::{Chunk, ChunkingConfig, ChunkingStrategy, Document, DocumentId, SourceId};
use rag_debugger_rag::{
    chunking::{Chunker, SmartSectionChunker, WhitespaceChunker},
    intelligence::{analyze_document, annotate_chunk_quality},
    ExtractionError, TextExtractor,
};
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub(crate) struct IngestionFile<'a> {
    pub file_name: &'a str,
    pub content_type: Option<&'a str>,
    pub bytes: &'a [u8],
}

#[derive(Debug)]
pub(crate) struct PreparedDocument {
    pub document: Document,
    pub chunks: Vec<Chunk>,
}

#[derive(Debug)]
pub(crate) struct DocumentPreparationError {
    pub code: &'static str,
    pub message: String,
}

pub(crate) fn prepare_document(
    source_id: SourceId,
    document_id: DocumentId,
    chunking: ChunkingConfig,
    file: IngestionFile<'_>,
) -> Result<PreparedDocument, DocumentPreparationError> {
    let extracted = TextExtractor
        .extract(file.file_name, file.content_type, file.bytes)
        .map_err(extraction_error)?;

    if extracted.text.trim().is_empty() {
        return Err(DocumentPreparationError {
            code: "empty_extracted_text",
            message: "no readable text was extracted from the file".to_owned(),
        });
    }

    let intelligence = analyze_document(file.file_name, &extracted.text);
    let document = Document {
        id: document_id,
        source_id,
        path: file.file_name.to_owned(),
        mime_type: extracted.mime_type,
        checksum: hex::encode(Sha256::digest(file.bytes)),
        byte_size: file.bytes.len() as u64,
        profile: intelligence.profile,
        extraction_quality: intelligence.extraction_quality,
        warnings: intelligence.warnings,
    };
    let mut chunks = chunk_document(&document, &extracted.text, chunking).map_err(|error| {
        DocumentPreparationError {
            code: "chunking_failed",
            message: error.to_string(),
        }
    })?;
    annotate_chunk_quality(&mut chunks);

    if chunks.is_empty() {
        return Err(DocumentPreparationError {
            code: "empty_chunks",
            message: "text extraction produced no chunks".to_owned(),
        });
    }

    Ok(PreparedDocument { document, chunks })
}

fn chunk_document(
    document: &Document,
    text: &str,
    chunking: ChunkingConfig,
) -> Result<Vec<Chunk>, rag_debugger_rag::RagError> {
    match chunking.strategy.normalized() {
        ChunkingStrategy::Structured | ChunkingStrategy::SmartSections => {
            SmartSectionChunker.chunk(document, text, chunking)
        }
        ChunkingStrategy::Whitespace => WhitespaceChunker.chunk(document, text, chunking),
    }
}

fn extraction_error(error: ExtractionError) -> DocumentPreparationError {
    let (code, message) = match error {
        ExtractionError::UnsupportedFileType => (
            "unsupported_file_type",
            "supported files are .txt, .md, .markdown, .html, .htm, and .pdf".to_owned(),
        ),
        ExtractionError::InvalidUtf8 => (
            "invalid_utf8",
            "text and markdown files must be valid UTF-8".to_owned(),
        ),
        ExtractionError::Html(message) => ("html_extraction_failed", message),
        ExtractionError::Pdf(message) => ("pdf_extraction_failed", message),
    };
    DocumentPreparationError { code, message }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rag_debugger_core::{DocumentProfile, ExtractionQuality};
    use uuid::Uuid;

    #[test]
    fn prepares_fixture_text_through_the_production_ingestion_pipeline() {
        let prepared = prepare_document(
            SourceId(Uuid::nil()),
            DocumentId(Uuid::from_u128(1)),
            ChunkingConfig {
                target_tokens: 128,
                overlap_tokens: 16,
                strategy: ChunkingStrategy::Structured,
            },
            IngestionFile {
                file_name: "policy.md",
                content_type: Some("text/markdown"),
                bytes: b"# Policy\n\n## Retention\n\nContent is retained for 30 days.",
            },
        )
        .expect("fixture should prepare");

        assert_eq!(prepared.document.profile, DocumentProfile::PolicyOrLegal);
        assert_eq!(prepared.document.extraction_quality, ExtractionQuality::Low);
        assert!(!prepared.chunks.is_empty());
        assert!(prepared
            .chunks
            .iter()
            .all(|chunk| chunk.source_id == SourceId(Uuid::nil())));
    }
}
