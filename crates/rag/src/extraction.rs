use std::{io::Cursor, path::Path};

use thiserror::Error;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExtractedText {
    pub text: String,
    pub mime_type: Option<String>,
    pub kind: SupportedFileKind,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SupportedFileKind {
    Text,
    Markdown,
    Html,
    Pdf,
}

#[derive(Debug, Error)]
pub enum ExtractionError {
    #[error("unsupported file type")]
    UnsupportedFileType,
    #[error("file is not valid UTF-8")]
    InvalidUtf8,
    #[error("HTML text extraction failed: {0}")]
    Html(String),
    #[error("PDF text extraction failed: {0}")]
    Pdf(String),
}

#[derive(Debug, Default)]
pub struct TextExtractor;

impl TextExtractor {
    pub fn extract(
        &self,
        file_name: &str,
        content_type: Option<&str>,
        bytes: &[u8],
    ) -> Result<ExtractedText, ExtractionError> {
        let kind = detect_kind(file_name, content_type)?;
        let text = match kind {
            SupportedFileKind::Text | SupportedFileKind::Markdown => std::str::from_utf8(bytes)
                .map_err(|_| ExtractionError::InvalidUtf8)?
                .to_owned(),
            SupportedFileKind::Html => html2text::from_read(Cursor::new(bytes), 120)
                .map_err(|error| ExtractionError::Html(error.to_string()))?,
            SupportedFileKind::Pdf => pdf_extract::extract_text_from_mem(bytes)
                .map_err(|error| ExtractionError::Pdf(error.to_string()))?,
        };

        Ok(ExtractedText {
            text,
            mime_type: normalized_mime_type(file_name, content_type, kind),
            kind,
        })
    }
}

fn detect_kind(
    file_name: &str,
    content_type: Option<&str>,
) -> Result<SupportedFileKind, ExtractionError> {
    let extension = Path::new(file_name)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);

    match extension.as_deref() {
        Some("txt") => return Ok(SupportedFileKind::Text),
        Some("md" | "markdown") => return Ok(SupportedFileKind::Markdown),
        Some("html" | "htm") => return Ok(SupportedFileKind::Html),
        Some("pdf") => return Ok(SupportedFileKind::Pdf),
        _ => {}
    }

    match content_type {
        Some("text/plain") => Ok(SupportedFileKind::Text),
        Some("text/markdown") => Ok(SupportedFileKind::Markdown),
        Some("text/html") => Ok(SupportedFileKind::Html),
        Some("application/pdf") => Ok(SupportedFileKind::Pdf),
        _ => Err(ExtractionError::UnsupportedFileType),
    }
}

fn normalized_mime_type(
    file_name: &str,
    content_type: Option<&str>,
    kind: SupportedFileKind,
) -> Option<String> {
    content_type
        .map(ToOwned::to_owned)
        .or_else(|| {
            mime_guess::from_path(file_name)
                .first_raw()
                .map(ToOwned::to_owned)
        })
        .or_else(|| {
            Some(
                match kind {
                    SupportedFileKind::Text => "text/plain",
                    SupportedFileKind::Markdown => "text/markdown",
                    SupportedFileKind::Html => "text/html",
                    SupportedFileKind::Pdf => "application/pdf",
                }
                .to_owned(),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_markdown_as_utf8_text() {
        let extracted = TextExtractor
            .extract("notes.md", None, b"# Title\n\nUseful context.")
            .expect("markdown extraction");

        assert_eq!(extracted.kind, SupportedFileKind::Markdown);
        assert!(extracted.text.contains("Useful context"));
    }

    #[test]
    fn renders_html_to_readable_text() {
        let extracted = TextExtractor
            .extract(
                "page.html",
                None,
                b"<html><body><h1>Guide</h1><p>Chunk this paragraph.</p></body></html>",
            )
            .expect("html extraction");

        assert_eq!(extracted.kind, SupportedFileKind::Html);
        assert!(extracted.text.contains("Guide"));
        assert!(extracted.text.contains("Chunk this paragraph"));
    }

    #[test]
    fn rejects_invalid_utf8_text() {
        let error = TextExtractor
            .extract("bad.txt", None, &[0xff, 0xfe])
            .expect_err("invalid utf8");

        assert!(matches!(error, ExtractionError::InvalidUtf8));
    }

    #[test]
    fn rejects_unsupported_extensions() {
        let error = TextExtractor
            .extract("archive.zip", None, b"content")
            .expect_err("unsupported type");

        assert!(matches!(error, ExtractionError::UnsupportedFileType));
    }

    #[test]
    fn reports_pdf_extraction_failure() {
        let error = TextExtractor
            .extract("not-a-real.pdf", None, b"%PDF-1.7 invalid")
            .expect_err("invalid pdf");

        assert!(matches!(error, ExtractionError::Pdf(_)));
    }
}
