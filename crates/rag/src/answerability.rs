use rag_debugger_core::{
    AnswerSupportAssessment, AnswerSupportReason, AnswerSupportStatus, AnswerabilityConfig,
    EvidenceStrength, RetrievalQualityFlag, RetrievalQueryHit,
};

use crate::text::{normalized_tokens, query_terms, split_sentences, truncate_chars};

pub fn assess_hits(query: &str, hits: &mut [RetrievalQueryHit], config: &AnswerabilityConfig) {
    let terms = query_terms(query);
    for hit in hits {
        let (assessment, supporting_sentence) = assess_hit(&terms, hit, config);
        if let Some(supporting_sentence) = supporting_sentence {
            let snippet = truncate_chars(supporting_sentence.trim(), 280);
            hit.snippet.clone_from(&snippet);
            hit.citation.snippet = snippet;
        }
        hit.answer_support = assessment;
    }
}

pub fn hits_are_assessed(hits: &[RetrievalQueryHit]) -> bool {
    hits.iter()
        .all(|hit| hit.answer_support.status != AnswerSupportStatus::Unassessed)
}

fn assess_hit<'a>(
    query_terms: &[String],
    hit: &'a RetrievalQueryHit,
    config: &AnswerabilityConfig,
) -> (AnswerSupportAssessment, Option<&'a str>) {
    let query_term_count = query_terms.len() as u32;
    let best_match_count = best_sentence_match_count(&hit.chunk.text, query_terms);
    let coverage = if query_term_count == 0 {
        0.0
    } else {
        best_match_count as f32 / query_term_count as f32
    };
    let supporting_sentence = supporting_sentence(&hit.chunk.text, query_terms, config);
    let body_supports_answer = supporting_sentence.is_some();

    let (status, reason) = if hit
        .quality_flags
        .contains(&RetrievalQualityFlag::HeadingOnly)
    {
        (
            AnswerSupportStatus::Unsupported,
            AnswerSupportReason::HeadingOnlyEvidence,
        )
    } else if hit.evidence_strength == EvidenceStrength::Weak
        || hit
            .quality_flags
            .contains(&RetrievalQualityFlag::WeakEvidence)
    {
        (
            AnswerSupportStatus::Unsupported,
            AnswerSupportReason::WeakEvidence,
        )
    } else if body_supports_answer {
        (
            AnswerSupportStatus::Supported,
            AnswerSupportReason::DirectBodySupport,
        )
    } else {
        (
            AnswerSupportStatus::Unsupported,
            unsupported_reason(hit, best_match_count),
        )
    };

    (
        AnswerSupportAssessment {
            status,
            reason,
            matched_body_term_count: best_match_count,
            query_term_count,
            body_term_coverage: coverage,
        },
        if status == AnswerSupportStatus::Supported {
            supporting_sentence
        } else {
            None
        },
    )
}

fn required_match_count(query_term_count: u32, config: &AnswerabilityConfig) -> u32 {
    if query_term_count <= 1 {
        return query_term_count;
    }
    let coverage_matches = (query_term_count as f32 * config.min_body_term_coverage).ceil() as u32;
    config
        .min_body_term_matches
        .max(coverage_matches)
        .min(query_term_count)
}

fn best_sentence_match_count(text: &str, query_terms: &[String]) -> u32 {
    split_sentences(text)
        .into_iter()
        .map(|sentence| {
            let tokens = normalized_tokens(sentence);
            query_terms
                .iter()
                .filter(|term| tokens.iter().any(|token| token == *term))
                .count() as u32
        })
        .max()
        .unwrap_or(0)
}

fn supporting_sentence<'a>(
    text: &'a str,
    query_terms: &[String],
    config: &AnswerabilityConfig,
) -> Option<&'a str> {
    if query_terms.is_empty() {
        return None;
    }
    let required_matches = required_match_count(query_terms.len() as u32, config);
    let numeric_terms = query_terms
        .iter()
        .filter(|term| term.chars().any(|character| character.is_ascii_digit()))
        .collect::<Vec<_>>();

    let mut best = None::<(u32, &str)>;
    for sentence in split_sentences(text) {
        let tokens = normalized_tokens(sentence);
        let matched_count = query_terms
            .iter()
            .filter(|term| tokens.iter().any(|token| token == *term))
            .count() as u32;
        let coverage = matched_count as f32 / query_terms.len() as f32;
        let supports = matched_count >= required_matches
            && coverage >= config.min_body_term_coverage
            && numeric_terms
                .iter()
                .all(|term| tokens.iter().any(|token| token == *term));
        if supports && best.is_none_or(|(best_count, _)| matched_count > best_count) {
            best = Some((matched_count, sentence));
        }
    }
    best.map(|(_, sentence)| sentence)
}

fn unsupported_reason(hit: &RetrievalQueryHit, body_match_count: u32) -> AnswerSupportReason {
    if body_match_count > 0 {
        return AnswerSupportReason::InsufficientBodyOverlap;
    }
    if hit.score_breakdown.semantic > 0.0 {
        return AnswerSupportReason::SemanticOnlyMatch;
    }

    let has_section = hit.score_breakdown.section > 0.0;
    let has_path = hit.score_breakdown.path > 0.0;
    match (has_section, has_path) {
        (true, false) => AnswerSupportReason::SectionOnlyMatch,
        (false, true) => AnswerSupportReason::PathOnlyMatch,
        (true, true) => AnswerSupportReason::MetadataOnlyMatch,
        (false, false) if hit.score_breakdown.metadata > 0.0 => {
            AnswerSupportReason::MetadataOnlyMatch
        }
        (false, false) => AnswerSupportReason::InsufficientBodyOverlap,
    }
}
