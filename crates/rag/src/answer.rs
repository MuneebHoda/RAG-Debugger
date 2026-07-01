use std::collections::BTreeSet;

use rag_debugger_core::{
    AnswerSupportStatus, ExtractiveAnswer, ExtractiveAnswerStatus, RetrievalQueryHit,
};

pub fn build_extractive_answer(
    hits: &[RetrievalQueryHit],
    citation_limit: u32,
) -> ExtractiveAnswer {
    let mut seen_chunks = BTreeSet::new();
    let citations = hits
        .iter()
        .filter(|hit| hit.answer_support.status == AnswerSupportStatus::Supported)
        .filter(|hit| seen_chunks.insert(hit.chunk.id.0))
        .take(citation_limit as usize)
        .map(|hit| hit.citation.clone())
        .collect::<Vec<_>>();

    if citations.is_empty() {
        return insufficient_answer(!hits.is_empty());
    }

    let text = citations
        .iter()
        .map(|citation| format!("{} {}", citation.snippet, citation.label))
        .collect::<Vec<_>>()
        .join("\n");

    ExtractiveAnswer {
        status: ExtractiveAnswerStatus::Answered,
        text,
        citations,
    }
}

pub fn insufficient_answer(has_candidates: bool) -> ExtractiveAnswer {
    let text = if has_candidates {
        "Ranked candidates were found, but no chunk body directly supports this question."
    } else {
        "Not enough local evidence was found in the indexed chunks."
    };
    ExtractiveAnswer {
        status: ExtractiveAnswerStatus::InsufficientEvidence,
        text: text.to_owned(),
        citations: Vec::new(),
    }
}
