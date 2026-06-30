use std::collections::HashSet;

use rag_debugger_core::{
    DebugReportRecommendation, DebugReportRecommendationArea, DebugReportRecommendationPriority,
};

pub(super) fn recommendations_for_failure_codes(
    failure_codes: &[String],
) -> Vec<DebugReportRecommendation> {
    let mut seen = HashSet::new();
    failure_codes
        .iter()
        .filter_map(|code| recommendation_for(code))
        .filter(|recommendation| seen.insert(recommendation.code.clone()))
        .collect()
}

pub(super) fn retrieval_mode_recommendation(best_mode: &str) -> DebugReportRecommendation {
    DebugReportRecommendation {
        code: "use_best_retrieval_mode".to_owned(),
        priority: DebugReportRecommendationPriority::Medium,
        area: DebugReportRecommendationArea::RetrievalMode,
        title: format!("Validate {best_mode} as the preferred retrieval mode"),
        rationale: "The evaluated modes produced a measurable retrieval-quality difference."
            .to_owned(),
        action: format!(
            "Rerun the dataset with {best_mode} and confirm the improvement against release gates."
        ),
        finding_codes: vec!["retrieval_mode_comparison".to_owned()],
    }
}

fn recommendation_for(failure_code: &str) -> Option<DebugReportRecommendation> {
    let recommendation = match failure_code {
        "missing_document" | "expected_evidence_missing" => recommendation(
            "expand_corpus_coverage",
            DebugReportRecommendationPriority::Critical,
            DebugReportRecommendationArea::CorpusCoverage,
            "Restore the missing evidence",
            "The expected document or chunk was not available in ranked evidence.",
            "Verify ingestion coverage, source filters, and whether the expected document is indexed.",
            failure_code,
        ),
        "missing_embedding_index" | "missing_embeddings" | "bad_embedding" => recommendation(
            "repair_embedding_index",
            DebugReportRecommendationPriority::Critical,
            DebugReportRecommendationArea::Embeddings,
            "Repair embedding coverage",
            "Vector signals were missing or incomplete for the evaluated corpus.",
            "Re-index missing or stale chunks with the configured embedding model before comparing retrieval modes.",
            failure_code,
        ),
        "bad_chunking" | "correct_document_wrong_chunk" | "duplicate_evidence"
        | "heading_only_evidence" => recommendation(
            "improve_chunk_boundaries",
            DebugReportRecommendationPriority::High,
            DebugReportRecommendationArea::Chunking,
            "Improve chunk boundaries and deduplication",
            "Chunk structure prevented the strongest evidence from ranking cleanly.",
            "Inspect the affected sections, remove normalized duplicates, and rerun structured chunking.",
            failure_code,
        ),
        "bad_ranking" | "low_precision" => recommendation(
            "tune_ranking",
            DebugReportRecommendationPriority::High,
            DebugReportRecommendationArea::Reranking,
            "Tune ranking precision",
            "Relevant evidence was diluted or ranked below weaker candidates.",
            "Compare lexical, vector, and hybrid signals, then add a reranking stage or adjust scoring weights.",
            failure_code,
        ),
        "weak_evidence" => recommendation(
            "review_top_k_and_coverage",
            DebugReportRecommendationPriority::Medium,
            DebugReportRecommendationArea::TopK,
            "Review top-k and evidence coverage",
            "Retrieved evidence did not meet the strength required for a defensible answer.",
            "Compare adjacent top-k values and confirm the corpus contains a direct answer before increasing recall.",
            failure_code,
        ),
        "missing_citations" | "hallucinated_answer" => recommendation(
            "repair_citation_grounding",
            DebugReportRecommendationPriority::Critical,
            DebugReportRecommendationArea::Citations,
            "Repair citation grounding",
            "The evidence summary was not backed by usable citations.",
            "Require cited evidence for every answer claim and reject answers without sufficient local evidence.",
            failure_code,
        ),
        "bad_prompt" | "unsupported_question" => recommendation(
            "clarify_query_contract",
            DebugReportRecommendationPriority::Medium,
            DebugReportRecommendationArea::Other,
            "Clarify the query contract",
            "The query or downstream prompt did not align with available corpus evidence.",
            "Rewrite the query or prompt constraints and add the case to Eval Lab before release.",
            failure_code,
        ),
        _ => return None,
    };
    Some(recommendation)
}

fn recommendation(
    code: &str,
    priority: DebugReportRecommendationPriority,
    area: DebugReportRecommendationArea,
    title: &str,
    rationale: &str,
    action: &str,
    finding_code: &str,
) -> DebugReportRecommendation {
    DebugReportRecommendation {
        code: code.to_owned(),
        priority,
        area,
        title: title.to_owned(),
        rationale: rationale.to_owned(),
        action: action.to_owned(),
        finding_codes: vec![finding_code.to_owned()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommendation_codes_are_deduplicated_in_failure_order() {
        let recommendations = recommendations_for_failure_codes(&[
            "bad_chunking".to_owned(),
            "duplicate_evidence".to_owned(),
            "missing_embeddings".to_owned(),
        ]);

        assert_eq!(
            recommendations
                .iter()
                .map(|recommendation| recommendation.code.as_str())
                .collect::<Vec<_>>(),
            vec!["improve_chunk_boundaries", "repair_embedding_index"]
        );
    }
}
