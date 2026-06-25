use std::collections::{HashMap, HashSet};

use rag_debugger_core::{
    Chunk, ChunkQualityFlag, DocumentProfile, DocumentWarning, ExtractionQuality,
};

const MIN_GOOD_EVIDENCE_TOKENS: u32 = 8;
const TOO_SHORT_TOKENS: u32 = 4;
const TOO_LONG_TOKENS: u32 = 900;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DocumentIntelligence {
    pub profile: DocumentProfile,
    pub extraction_quality: ExtractionQuality,
    pub warnings: Vec<DocumentWarning>,
}

pub fn analyze_document(path: &str, text: &str) -> DocumentIntelligence {
    let normalized = text.to_ascii_lowercase();
    let token_count = text.split_whitespace().count();
    let profile = detect_profile(path, &normalized);
    let mut warnings = Vec::new();

    if token_count < 20 {
        warnings.push(DocumentWarning {
            code: "low_text_volume".to_owned(),
            message: "The document produced very little readable text.".to_owned(),
        });
    }

    if normalized.contains('\u{fffd}') {
        warnings.push(DocumentWarning {
            code: "replacement_characters".to_owned(),
            message: "The extracted text contains replacement characters.".to_owned(),
        });
    }

    let extraction_quality = if token_count >= 80 && warnings.is_empty() {
        ExtractionQuality::High
    } else if token_count >= 20 {
        ExtractionQuality::Medium
    } else {
        ExtractionQuality::Low
    };

    DocumentIntelligence {
        profile,
        extraction_quality,
        warnings,
    }
}

pub fn annotate_chunk_quality(chunks: &mut [Chunk]) {
    let mut normalized_counts = HashMap::new();
    for chunk in chunks.iter() {
        *normalized_counts
            .entry(normalize_for_duplicate(&chunk.text))
            .or_insert(0u32) += 1;
    }

    for chunk in chunks {
        let mut flags = HashSet::new();
        let text_density = text_density(&chunk.text);
        let normalized = normalize_for_duplicate(&chunk.text);
        let is_duplicate = normalized_counts.get(&normalized).copied().unwrap_or(0) > 1;
        let is_heading_only = is_heading_only(&chunk.text, chunk.section_title.as_deref());

        if is_heading_only {
            flags.insert(ChunkQualityFlag::HeadingOnly);
        }
        if chunk.token_count <= TOO_SHORT_TOKENS {
            flags.insert(ChunkQualityFlag::TooShort);
        }
        if chunk.token_count >= TOO_LONG_TOKENS {
            flags.insert(ChunkQualityFlag::TooLong);
        }
        if is_duplicate {
            flags.insert(ChunkQualityFlag::Duplicate);
        }
        if text_density < 0.45 {
            flags.insert(ChunkQualityFlag::LowTextDensity);
        }
        if chunk.token_count >= MIN_GOOD_EVIDENCE_TOKENS && !is_heading_only && text_density >= 0.55
        {
            flags.insert(ChunkQualityFlag::GoodEvidenceCandidate);
        }

        let mut flags = flags.into_iter().collect::<Vec<_>>();
        flags.sort_by_key(|flag| format!("{flag:?}"));

        chunk.quality_flags = flags;
        chunk.is_duplicate = is_duplicate;
        chunk.text_density = text_density;
        chunk.evidence_score_hint = evidence_score_hint(chunk);
    }
}

pub fn is_heading_only(text: &str, section_title: Option<&str>) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return true;
    }

    if let Some(section_title) = section_title {
        if trimmed.eq_ignore_ascii_case(section_title.trim()) {
            return true;
        }
    }

    let line_count = trimmed
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    let token_count = trimmed.split_whitespace().count();
    line_count <= 2 && token_count <= 6 && !trimmed.contains(['.', ':', ';'])
}

pub fn text_density(text: &str) -> f32 {
    let total = text.chars().count();
    if total == 0 {
        return 0.0;
    }

    let useful = text
        .chars()
        .filter(|character| character.is_alphanumeric() || character.is_whitespace())
        .count();
    useful as f32 / total as f32
}

pub fn normalize_for_duplicate(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn detect_profile(path: &str, normalized_text: &str) -> DocumentProfile {
    let path = path.to_ascii_lowercase();
    let contains_any = |terms: &[&str]| {
        terms
            .iter()
            .any(|term| normalized_text.contains(term) || path.contains(term))
    };

    if contains_any(&["abstract", "references", "methodology", "doi", "arxiv"]) {
        DocumentProfile::ResearchPaper
    } else if contains_any(&["policy", "agreement", "terms", "contract", "liability"]) {
        DocumentProfile::PolicyOrLegal
    } else if contains_any(&["faq", "support", "troubleshooting", "help center", "ticket"]) {
        DocumentProfile::SupportKb
    } else if contains_any(&[
        "api",
        "sdk",
        "readme",
        "endpoint",
        "installation",
        "configuration",
    ]) {
        DocumentProfile::TechnicalDocs
    } else if contains_any(&[
        "function",
        "class",
        "module",
        "crate",
        "package",
        "repository",
    ]) {
        DocumentProfile::CodeDocs
    } else if contains_any(&["experience", "education", "skills", "resume", "cv"]) {
        DocumentProfile::Resume
    } else {
        DocumentProfile::General
    }
}

fn evidence_score_hint(chunk: &Chunk) -> f32 {
    let mut score: f32 = 0.5;
    if chunk
        .quality_flags
        .contains(&ChunkQualityFlag::GoodEvidenceCandidate)
    {
        score += 0.35;
    }
    if chunk.quality_flags.contains(&ChunkQualityFlag::HeadingOnly) {
        score -= 0.35;
    }
    if chunk.quality_flags.contains(&ChunkQualityFlag::Duplicate) {
        score -= 0.1;
    }
    if chunk
        .quality_flags
        .contains(&ChunkQualityFlag::LowTextDensity)
    {
        score -= 0.15;
    }

    score.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use rag_debugger_core::{
        ByteRange, ChunkId, ChunkSplitReason, ChunkingStrategy, DocumentId, SourceId,
    };
    use uuid::Uuid;

    use super::*;

    #[test]
    fn detects_general_document_profiles() {
        assert_eq!(
            analyze_document("paper.pdf", "Abstract\nMethodology\nReferences").profile,
            DocumentProfile::ResearchPaper
        );
        assert_eq!(
            analyze_document("terms.pdf", "This agreement limits liability.").profile,
            DocumentProfile::PolicyOrLegal
        );
        assert_eq!(
            analyze_document("api.md", "Endpoint configuration and SDK installation.").profile,
            DocumentProfile::TechnicalDocs
        );
    }

    #[test]
    fn flags_duplicate_and_heading_only_chunks() {
        let mut chunks = vec![chunk("Projects", "Projects"), chunk("Projects", "Projects")];
        annotate_chunk_quality(&mut chunks);

        assert!(chunks[0].is_duplicate);
        assert!(chunks[0]
            .quality_flags
            .contains(&ChunkQualityFlag::Duplicate));
        assert!(chunks[0]
            .quality_flags
            .contains(&ChunkQualityFlag::HeadingOnly));
    }

    fn chunk(section: &str, text: &str) -> Chunk {
        Chunk {
            id: ChunkId(Uuid::now_v7()),
            source_id: SourceId(Uuid::now_v7()),
            document_id: DocumentId(Uuid::now_v7()),
            ordinal: 0,
            text: text.to_owned(),
            token_count: text.split_whitespace().count() as u32,
            byte_range: ByteRange {
                start: 0,
                end: text.len() as u64,
            },
            checksum: text.to_owned(),
            strategy: ChunkingStrategy::Structured,
            section_title: Some(section.to_owned()),
            split_reason: ChunkSplitReason::DocumentEnd,
            quality_flags: Vec::new(),
            is_duplicate: false,
            text_density: 0.0,
            evidence_score_hint: 0.0,
        }
    }
}
