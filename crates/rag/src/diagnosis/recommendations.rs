use rag_debugger_core::{
    DiagnosisFailure, DiagnosisFailureCode, DiagnosisRecommendation,
    DiagnosisRecommendationPriority, DiagnosisRemediationArea,
};

pub(super) fn recommendations_for(failures: &[DiagnosisFailure]) -> Vec<DiagnosisRecommendation> {
    let mut recommendations = Vec::<DiagnosisRecommendation>::new();
    for failure in failures {
        let recommendation = recommendation_for(failure);
        if let Some(existing) = recommendations
            .iter_mut()
            .find(|existing| existing.code == recommendation.code)
        {
            push_unique(&mut existing.failure_codes, failure.code);
            for evidence_ref in &failure.evidence_refs {
                push_unique(&mut existing.evidence_refs, evidence_ref.clone());
            }
        } else {
            recommendations.push(recommendation);
        }
    }
    recommendations.sort_by_key(|recommendation| {
        (
            priority_order(recommendation.priority),
            recommendation.code.clone(),
        )
    });
    recommendations
}

fn recommendation_for(failure: &DiagnosisFailure) -> DiagnosisRecommendation {
    let (code, priority, area, title, rationale, action) = match failure.code {
        DiagnosisFailureCode::MissingDocument => (
            "restore_corpus_coverage",
            DiagnosisRecommendationPriority::Critical,
            DiagnosisRemediationArea::CorpusCoverage,
            "Restore corpus coverage",
            "No usable evidence was available for the query.",
            "Confirm the required source is ingested, extracted successfully, and included by the active filters.",
        ),
        DiagnosisFailureCode::MissingExpectedEvidence => (
            "review_expected_evidence_filters",
            DiagnosisRecommendationPriority::Critical,
            DiagnosisRemediationArea::MetadataFilters,
            "Review expected evidence and filters",
            "The eval case expected evidence that did not appear in the ranking.",
            "Verify corpus coverage and metadata filters, then rerun the case with a larger candidate set.",
        ),
        DiagnosisFailureCode::MissingEmbeddingIndex
        | DiagnosisFailureCode::PartialEmbeddingIndex => (
            "repair_embedding_index",
            if failure.code == DiagnosisFailureCode::MissingEmbeddingIndex {
                DiagnosisRecommendationPriority::Critical
            } else {
                DiagnosisRecommendationPriority::High
            },
            DiagnosisRemediationArea::Embeddings,
            "Repair the embedding index",
            "Semantic retrieval did not have complete embedding coverage.",
            "Index missing or stale chunks with the configured model before comparing retrieval modes.",
        ),
        DiagnosisFailureCode::WeakEvidence => (
            "broaden_candidate_pool",
            DiagnosisRecommendationPriority::High,
            DiagnosisRemediationArea::TopK,
            "Broaden the candidate pool",
            "The current result set does not contain strong enough evidence.",
            "Increase top_k and inspect whether stronger evidence appears before changing answer generation.",
        ),
        DiagnosisFailureCode::DuplicateEvidence
        | DiagnosisFailureCode::HeadingOnlyEvidence => (
            "improve_chunk_boundaries",
            DiagnosisRecommendationPriority::High,
            DiagnosisRemediationArea::Chunking,
            "Improve chunk boundaries",
            "Chunk quality reduced evidence diversity or completeness.",
            "Rechunk the affected documents and prevent heading-only or duplicate chunks from entering the index.",
        ),
        DiagnosisFailureCode::LowScoreMargin => (
            "add_reranking",
            DiagnosisRecommendationPriority::Medium,
            DiagnosisRemediationArea::Reranking,
            "Add a reranking stage",
            "The top candidates are too close for the baseline scorer to separate confidently.",
            "Rerank the candidate set and add an eval case that checks the expected top result.",
        ),
        DiagnosisFailureCode::VectorLexicalDisagreement => (
            "compare_retrieval_modes",
            DiagnosisRecommendationPriority::Medium,
            DiagnosisRemediationArea::RetrievalMode,
            "Compare retrieval modes",
            "Semantic and lexical signals disagree about the strongest evidence.",
            "Rerun lexical, vector, and hybrid modes and preserve the best behavior in Eval Lab.",
        ),
        DiagnosisFailureCode::CitationMissing
        | DiagnosisFailureCode::TopResultNotCited => (
            "repair_citation_grounding",
            DiagnosisRecommendationPriority::Critical,
            DiagnosisRemediationArea::Citations,
            "Repair citation grounding",
            "The evidence summary is not fully grounded in the ranked evidence.",
            "Require cited evidence for every answer claim and reject summaries without usable citations.",
        ),
    };
    DiagnosisRecommendation {
        code: code.to_owned(),
        priority,
        area,
        title: title.to_owned(),
        rationale: rationale.to_owned(),
        action: action.to_owned(),
        failure_codes: vec![failure.code],
        evidence_refs: failure.evidence_refs.clone(),
    }
}

fn push_unique<T: PartialEq>(values: &mut Vec<T>, value: T) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn priority_order(priority: DiagnosisRecommendationPriority) -> u8 {
    match priority {
        DiagnosisRecommendationPriority::Critical => 0,
        DiagnosisRecommendationPriority::High => 1,
        DiagnosisRecommendationPriority::Medium => 2,
        DiagnosisRecommendationPriority::Low => 3,
    }
}
