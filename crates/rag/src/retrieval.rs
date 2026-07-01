use std::{collections::BTreeMap, time::Instant};

use async_trait::async_trait;
use rag_debugger_core::{
    AnswerSupportAssessment, Chunk, ChunkEmbedding, ChunkPreview, ChunkQualityFlag, DebuggerConfig,
    EmbeddingModelInfo, EvidenceStrength, ExtractiveAnswer, ExtractiveAnswerStatus,
    RetrievalCitation, RetrievalConfig, RetrievalEmbeddingReadiness, RetrievalEmbeddingStatus,
    RetrievalMatchedTerm, RetrievalMode, RetrievalQualityFlag, RetrievalQueryHit,
    RetrievalQueryRequest, RetrievalQueryResponse, RetrievalQueryRun, RetrievalQueryRunId,
    RetrievalRun, RetrievalScoreBreakdown, RetrievalWeights, SearchableChunk, TraceId,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    answer::build_extractive_answer,
    answerability::{assess_hits, hits_are_assessed},
    diagnosis::attach_diagnosis,
    embedding::{cosine_similarity, EmbeddingProvider, LocalHashEmbeddingProvider},
    text::{best_snippet, normalize_text, normalized_tokens, query_terms},
    RagError,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RetrievalRequest {
    pub trace_id: TraceId,
    pub query: String,
    pub top_k: u32,
}

#[async_trait]
pub trait Retriever: Send + Sync {
    async fn retrieve(&self, request: RetrievalRequest) -> Result<RetrievalRun, RagError>;
}

#[derive(Debug, Default)]
pub struct PlaceholderRetriever;

#[async_trait]
impl Retriever for PlaceholderRetriever {
    async fn retrieve(&self, _request: RetrievalRequest) -> Result<RetrievalRun, RagError> {
        Err(RagError::NotImplemented("retrieval engine"))
    }
}

#[derive(Debug, Default)]
pub struct LocalHybridRetriever {
    embedding_provider: LocalHashEmbeddingProvider,
    config: RetrievalConfig,
    debugger_config: DebuggerConfig,
}

impl LocalHybridRetriever {
    pub fn new(embedding_provider: LocalHashEmbeddingProvider, config: RetrievalConfig) -> Self {
        Self {
            embedding_provider,
            config,
            debugger_config: DebuggerConfig::default(),
        }
    }

    pub fn with_debugger_config(mut self, debugger_config: DebuggerConfig) -> Self {
        self.debugger_config = debugger_config;
        self
    }

    pub fn retrieve(
        &self,
        mut request: RetrievalQueryRequest,
        candidates: Vec<SearchableChunk>,
    ) -> Result<RetrievalQueryResponse, RagError> {
        let started_at = Instant::now();
        request.query = request.query.trim().to_owned();
        if request.query.is_empty() {
            return Err(RagError::InvalidConfig("query must not be empty"));
        }

        if request.top_k == 0 {
            request.top_k = self.config.default_top_k;
        }
        request.top_k = request.top_k.min(self.config.max_top_k);

        let embedding_model = self.embedding_provider.model();
        let query_terms = query_terms(&request.query);
        if query_terms.is_empty() && matches!(request.retrieval_mode, RetrievalMode::Lexical) {
            return Ok(attach_diagnosis(
                response_without_evidence(
                    request,
                    started_at.elapsed().as_millis() as u64,
                    not_required_embedding_status(embedding_model, candidates.len() as u32),
                    None,
                ),
                &self.debugger_config,
            ));
        }

        let embedding_status =
            embedding_query_status(&candidates, &embedding_model, request.retrieval_mode);
        let query_embedding = if request.retrieval_mode.requires_embeddings() {
            Some(self.embedding_provider.embed_one(&request.query)?)
        } else {
            None
        };

        if request.retrieval_mode.requires_embeddings()
            && embedding_status.indexed_chunks == 0
            && embedding_status.total_chunks > 0
        {
            return Ok(attach_diagnosis(
                response_without_evidence(
                request,
                started_at.elapsed().as_millis() as u64,
                embedding_status,
                Some("Embeddings are not indexed yet. Index local embeddings, then run this query again."),
                ),
                &self.debugger_config,
            ));
        }

        let normalized_query = normalize_text(&request.query);
        let mut hits = candidates
            .into_iter()
            .filter_map(|candidate| {
                score_candidate(
                    candidate,
                    &query_terms,
                    &normalized_query,
                    request.retrieval_mode,
                    query_embedding.as_deref(),
                    &embedding_model,
                    &self.config,
                )
            })
            .filter(|hit| hit.score > 0.0)
            .collect::<Vec<_>>();

        hits.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.document.path.cmp(&right.document.path))
                .then_with(|| left.chunk.ordinal.cmp(&right.chunk.ordinal))
        });
        let mut hits = dedupe_hits(hits);
        hits.truncate(request.top_k as usize);

        for (index, hit) in hits.iter_mut().enumerate() {
            hit.rank = (index + 1) as u32;
            hit.citation.label = format!("[{}]", index + 1);
        }

        assess_hits(&request.query, &mut hits, &self.config.answerability);
        let answer = build_extractive_answer(&hits, self.config.answer_citation_limit);
        let run = RetrievalQueryRun {
            id: RetrievalQueryRunId(Uuid::now_v7()),
            query: request.query,
            top_k: request.top_k,
            retrieval_mode: request.retrieval_mode,
            latency_ms: started_at.elapsed().as_millis() as u64,
            created_at: OffsetDateTime::now_utc(),
        };

        Ok(attach_diagnosis(
            RetrievalQueryResponse {
                run,
                answer,
                hits,
                embedding_status,
                diagnosis: None,
            },
            &self.debugger_config,
        ))
    }
}

pub fn ensure_response_answerability(
    mut response: RetrievalQueryResponse,
    retrieval_config: &RetrievalConfig,
    debugger_config: &DebuggerConfig,
) -> RetrievalQueryResponse {
    if !response.hits.is_empty() && !hits_are_assessed(&response.hits) {
        assess_hits(
            &response.run.query,
            &mut response.hits,
            &retrieval_config.answerability,
        );
        response.answer =
            build_extractive_answer(&response.hits, retrieval_config.answer_citation_limit);
        response.diagnosis = None;
    }

    if response.diagnosis.is_none() {
        attach_diagnosis(response, debugger_config)
    } else {
        response
    }
}

fn response_without_evidence(
    request: RetrievalQueryRequest,
    latency_ms: u64,
    embedding_status: RetrievalEmbeddingStatus,
    message: Option<&str>,
) -> RetrievalQueryResponse {
    RetrievalQueryResponse {
        run: RetrievalQueryRun {
            id: RetrievalQueryRunId(Uuid::now_v7()),
            query: request.query,
            top_k: request.top_k,
            retrieval_mode: request.retrieval_mode,
            latency_ms,
            created_at: OffsetDateTime::now_utc(),
        },
        answer: insufficient_answer_with_message(message),
        hits: Vec::new(),
        embedding_status,
        diagnosis: None,
    }
}

fn score_candidate(
    candidate: SearchableChunk,
    query_terms: &[String],
    normalized_query: &str,
    retrieval_mode: RetrievalMode,
    query_embedding: Option<&[f32]>,
    embedding_model: &EmbeddingModelInfo,
    config: &RetrievalConfig,
) -> Option<RetrievalQueryHit> {
    let chunk_tokens = normalized_tokens(&candidate.chunk.text);
    let section_tokens = candidate
        .chunk
        .section_title
        .as_deref()
        .map(normalized_tokens)
        .unwrap_or_default();
    let path_tokens = normalized_tokens(&candidate.document.path);
    let combined_tokens = chunk_tokens
        .iter()
        .chain(section_tokens.iter())
        .chain(path_tokens.iter())
        .cloned()
        .collect::<Vec<_>>();

    let matched_terms = matched_terms(query_terms, &combined_tokens);
    let semantic = semantic_score(
        &candidate.embedding,
        query_embedding,
        embedding_model,
        config.min_semantic_similarity,
    );
    if matched_terms.is_empty()
        && (semantic == 0.0 || matches!(retrieval_mode, RetrievalMode::Lexical))
    {
        return None;
    }

    let lexical = if matched_terms.is_empty() {
        0.0
    } else {
        lexical_score(query_terms, &matched_terms, &config.weights)
    };
    let phrase = if matched_terms.is_empty() {
        0.0
    } else {
        phrase_score(
            &candidate.chunk.text,
            normalized_query,
            query_terms,
            &config.weights,
        )
    };
    let section = field_score(query_terms, &section_tokens) * config.weights.section;
    let path = field_score(query_terms, &path_tokens) * config.weights.path;
    let metadata = metadata_score(&candidate) * config.weights.metadata;
    let score = match retrieval_mode {
        RetrievalMode::Lexical => lexical + phrase + section + path + metadata,
        RetrievalMode::Vector => semantic * config.weights.semantic_vector,
        RetrievalMode::Hybrid => {
            lexical + phrase + section + path + metadata + semantic * config.weights.semantic_hybrid
        }
    };
    let semantic_breakdown = match retrieval_mode {
        RetrievalMode::Lexical => 0.0,
        RetrievalMode::Vector => semantic * config.weights.semantic_vector,
        RetrievalMode::Hybrid => semantic * config.weights.semantic_hybrid,
    };
    let snippet = best_snippet(&candidate.chunk.text, query_terms);
    let checksum_prefix = candidate
        .chunk
        .checksum
        .chars()
        .take(12)
        .collect::<String>();
    let citation = RetrievalCitation {
        label: String::new(),
        chunk_id: candidate.chunk.id,
        document_id: candidate.document.id,
        document_path: candidate.document.path.clone(),
        chunk_ordinal: candidate.chunk.ordinal,
        section_title: candidate.chunk.section_title.clone(),
        checksum_prefix,
        snippet: snippet.clone(),
    };

    let score_breakdown = RetrievalScoreBreakdown {
        semantic: semantic_breakdown,
        lexical,
        phrase,
        section,
        path,
        metadata,
    };
    let quality_flags =
        retrieval_quality_flags(&candidate.chunk, &matched_terms, semantic, score_breakdown);
    let evidence_strength = evidence_strength(score, &quality_flags, config);

    Some(RetrievalQueryHit {
        rank: 0,
        score,
        chunk: ChunkPreview::from(candidate.chunk),
        document: candidate.document,
        source: candidate.source,
        matched_terms,
        normalized_score_breakdown: normalize_score_breakdown(score_breakdown),
        score_breakdown,
        snippet,
        citation,
        quality_flags,
        evidence_strength,
        duplicate_count: 1,
        answer_support: AnswerSupportAssessment::default(),
    })
}

fn semantic_score(
    embedding: &Option<ChunkEmbedding>,
    query_embedding: Option<&[f32]>,
    embedding_model: &EmbeddingModelInfo,
    min_semantic_similarity: f32,
) -> f32 {
    let (Some(embedding), Some(query_embedding)) = (embedding, query_embedding) else {
        return 0.0;
    };

    if embedding.model != *embedding_model
        || embedding.vector.len() != query_embedding.len()
        || embedding.chunk_checksum.is_empty()
    {
        return 0.0;
    }

    let similarity = cosine_similarity(query_embedding, &embedding.vector).max(0.0);
    if similarity < min_semantic_similarity {
        0.0
    } else {
        similarity
    }
}

fn embedding_query_status(
    candidates: &[SearchableChunk],
    model: &EmbeddingModelInfo,
    retrieval_mode: RetrievalMode,
) -> RetrievalEmbeddingStatus {
    if !retrieval_mode.requires_embeddings() {
        return not_required_embedding_status(model.clone(), candidates.len() as u32);
    }

    let mut indexed_chunks = 0u32;
    let mut missing_chunks = 0u32;
    let mut stale_chunks = 0u32;

    for candidate in candidates {
        match &candidate.embedding {
            Some(embedding)
                if embedding.model == *model
                    && embedding.chunk_checksum == candidate.chunk.checksum
                    && embedding.vector.len() == model.dimension as usize =>
            {
                indexed_chunks += 1;
            }
            Some(_) => stale_chunks += 1,
            None => missing_chunks += 1,
        }
    }

    let total_chunks = candidates.len() as u32;
    let readiness = if total_chunks == 0 || indexed_chunks == total_chunks {
        RetrievalEmbeddingReadiness::Ready
    } else if indexed_chunks == 0 {
        RetrievalEmbeddingReadiness::Missing
    } else {
        RetrievalEmbeddingReadiness::Partial
    };

    RetrievalEmbeddingStatus {
        readiness,
        required: true,
        model: model.clone(),
        total_chunks,
        indexed_chunks,
        missing_chunks,
        stale_chunks,
    }
}

fn not_required_embedding_status(
    model: EmbeddingModelInfo,
    total_chunks: u32,
) -> RetrievalEmbeddingStatus {
    RetrievalEmbeddingStatus {
        readiness: RetrievalEmbeddingReadiness::NotRequired,
        required: false,
        model,
        total_chunks,
        indexed_chunks: 0,
        missing_chunks: 0,
        stale_chunks: 0,
    }
}

fn matched_terms(query_terms: &[String], tokens: &[String]) -> Vec<RetrievalMatchedTerm> {
    let mut counts = BTreeMap::new();
    for token in tokens {
        if query_terms.iter().any(|term| term == token) {
            *counts.entry(token.clone()).or_insert(0u32) += 1;
        }
    }

    counts
        .into_iter()
        .map(|(term, count)| RetrievalMatchedTerm { term, count })
        .collect()
}

fn lexical_score(
    query_terms: &[String],
    matched_terms: &[RetrievalMatchedTerm],
    weights: &RetrievalWeights,
) -> f32 {
    let coverage = matched_terms.len() as f32 / query_terms.len().max(1) as f32;
    let frequency = matched_terms
        .iter()
        .map(|term| term.count.min(4) as f32)
        .sum::<f32>()
        / (query_terms.len().max(1) as f32 * 4.0);

    coverage * weights.lexical + frequency * weights.frequency
}

fn phrase_score(
    text: &str,
    normalized_query: &str,
    query_terms: &[String],
    weights: &RetrievalWeights,
) -> f32 {
    let normalized_text = normalize_text(text);
    if !normalized_query.is_empty() && normalized_text.contains(normalized_query) {
        return weights.phrase;
    }

    query_terms
        .windows(2)
        .filter(|pair| normalized_text.contains(&pair.join(" ")))
        .count() as f32
        * (weights.phrase / 4.0)
}

fn field_score(query_terms: &[String], field_tokens: &[String]) -> f32 {
    if field_tokens.is_empty() {
        return 0.0;
    }

    let matches = query_terms
        .iter()
        .filter(|term| field_tokens.iter().any(|token| token == *term))
        .count();

    matches as f32 / query_terms.len().max(1) as f32
}

fn metadata_score(candidate: &SearchableChunk) -> f32 {
    let mut score = 0.0;
    if candidate.chunk.section_title.is_some() {
        score += 0.08;
    }
    if candidate.chunk.token_count > 0 {
        score += 0.02;
    }
    score
}

fn dedupe_hits(hits: Vec<RetrievalQueryHit>) -> Vec<RetrievalQueryHit> {
    let mut deduped: Vec<RetrievalQueryHit> = Vec::new();
    let mut seen = BTreeMap::<String, usize>::new();

    for mut hit in hits {
        let key = dedupe_key(&hit);
        if let Some(index) = seen.get(&key).copied() {
            let existing = &mut deduped[index];
            existing.duplicate_count += 1;
            if !existing
                .quality_flags
                .contains(&RetrievalQualityFlag::Duplicate)
            {
                existing.quality_flags.push(RetrievalQualityFlag::Duplicate);
            }
            existing.chunk.is_duplicate = true;
            if hit.score > existing.score {
                hit.duplicate_count = existing.duplicate_count;
                hit.quality_flags = existing.quality_flags.clone();
                deduped[index] = hit;
            }
            continue;
        }

        seen.insert(key, deduped.len());
        deduped.push(hit);
    }

    deduped
}

fn dedupe_key(hit: &RetrievalQueryHit) -> String {
    let normalized = normalize_text(&hit.chunk.text);
    if !hit.chunk.checksum.is_empty() {
        return format!("checksum:{}", hit.chunk.checksum);
    }
    if !normalized.is_empty() {
        return format!("text:{normalized}");
    }
    format!("chunk:{}", hit.chunk.id.0)
}

fn normalize_score_breakdown(breakdown: RetrievalScoreBreakdown) -> RetrievalScoreBreakdown {
    let max = [
        breakdown.semantic,
        breakdown.lexical,
        breakdown.phrase,
        breakdown.section,
        breakdown.path,
        breakdown.metadata,
    ]
    .into_iter()
    .fold(0.0_f32, f32::max);

    if max <= f32::EPSILON {
        return RetrievalScoreBreakdown::zero();
    }

    RetrievalScoreBreakdown {
        semantic: breakdown.semantic / max,
        lexical: breakdown.lexical / max,
        phrase: breakdown.phrase / max,
        section: breakdown.section / max,
        path: breakdown.path / max,
        metadata: breakdown.metadata / max,
    }
}

fn retrieval_quality_flags(
    chunk: &Chunk,
    matched_terms: &[RetrievalMatchedTerm],
    semantic_score: f32,
    score_breakdown: RetrievalScoreBreakdown,
) -> Vec<RetrievalQualityFlag> {
    let mut flags = Vec::new();
    if chunk.is_duplicate || chunk.quality_flags.contains(&ChunkQualityFlag::Duplicate) {
        flags.push(RetrievalQualityFlag::Duplicate);
    }
    if chunk.quality_flags.contains(&ChunkQualityFlag::HeadingOnly) {
        flags.push(RetrievalQualityFlag::HeadingOnly);
    }
    if chunk.quality_flags.contains(&ChunkQualityFlag::TooShort) {
        flags.push(RetrievalQualityFlag::TooShort);
    }
    if semantic_score > 0.0 {
        flags.push(RetrievalQualityFlag::SemanticMatch);
    }
    if !matched_terms.is_empty() {
        flags.push(RetrievalQualityFlag::ExactTermMatch);
    }
    if score_breakdown.section > 0.0
        && score_breakdown.lexical == 0.0
        && score_breakdown.semantic == 0.0
    {
        flags.push(RetrievalQualityFlag::SectionOnlyMatch);
    }
    if chunk.evidence_score_hint < 0.35 {
        flags.push(RetrievalQualityFlag::WeakEvidence);
    }
    flags
}

fn evidence_strength(
    score: f32,
    flags: &[RetrievalQualityFlag],
    config: &RetrievalConfig,
) -> EvidenceStrength {
    if flags.contains(&RetrievalQualityFlag::HeadingOnly)
        || flags.contains(&RetrievalQualityFlag::SectionOnlyMatch)
        || score < config.min_evidence_score
    {
        return EvidenceStrength::Weak;
    }

    if flags.contains(&RetrievalQualityFlag::WeakEvidence)
        || flags.contains(&RetrievalQualityFlag::TooShort)
        || score < config.min_evidence_score * 2.0
    {
        return EvidenceStrength::Medium;
    }

    EvidenceStrength::Strong
}

fn insufficient_answer_with_message(message: Option<&str>) -> ExtractiveAnswer {
    ExtractiveAnswer {
        status: ExtractiveAnswerStatus::InsufficientEvidence,
        text: message
            .unwrap_or("Not enough local evidence was found in the indexed chunks.")
            .to_owned(),
        citations: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use rag_debugger_core::{
        AnswerSupportReason, AnswerSupportStatus, ByteRange, Chunk, ChunkId, ChunkQualityFlag,
        ChunkSplitReason, ChunkingConfig, ChunkingStrategy, Document, DocumentId, DocumentProfile,
        ExtractionQuality, ProjectId, Source, SourceId, SourceKind, SourceSyncPolicy,
    };
    use uuid::Uuid;

    use super::*;

    #[test]
    fn normalizes_query_tokens_and_removes_stop_words() {
        assert_eq!(
            query_terms("What GPU-indexing skills?"),
            vec!["gpu", "indexing", "skills"]
        );
    }

    #[test]
    fn lexical_matches_rank_relevant_chunks() {
        let response = LocalHybridRetriever::default()
            .retrieve(
                lexical_request("gpu indexing"),
                vec![
                    candidate("resume.md", "Summary", "Built frontend dashboards."),
                    candidate("resume.md", "Projects", "Built GPU indexing experiments."),
                ],
            )
            .expect("retrieval");

        assert_eq!(response.hits.len(), 1);
        assert_eq!(response.hits[0].document.path, "resume.md");
        assert!(response.hits[0].score_breakdown.lexical > 0.0);
    }

    #[test]
    fn phrase_match_adds_score() {
        let response = LocalHybridRetriever::default()
            .retrieve(
                lexical_request("rag debugger"),
                vec![candidate(
                    "resume.md",
                    "Projects",
                    "Created a RAG debugger for chunk analysis.",
                )],
            )
            .expect("retrieval");

        assert!(response.hits[0].score_breakdown.phrase > 0.0);
    }

    #[test]
    fn section_and_path_boosts_are_reported() {
        let response = LocalHybridRetriever::default()
            .retrieve(
                lexical_request("projects resume"),
                vec![candidate(
                    "resume.md",
                    "Projects",
                    "Built a local search tool.",
                )],
            )
            .expect("retrieval");

        assert!(response.hits[0].score_breakdown.section > 0.0);
        assert!(response.hits[0].score_breakdown.path > 0.0);
    }

    #[test]
    fn no_result_returns_insufficient_evidence() {
        let response = LocalHybridRetriever::default()
            .retrieve(
                lexical_request("kubernetes"),
                vec![candidate(
                    "resume.md",
                    "Projects",
                    "Built a local search tool.",
                )],
            )
            .expect("retrieval");

        assert!(response.hits.is_empty());
        assert_eq!(
            response.answer.status,
            ExtractiveAnswerStatus::InsufficientEvidence
        );
    }

    #[test]
    fn extractive_answer_contains_citation_labels() {
        let response = LocalHybridRetriever::default()
            .retrieve(
                lexical_request("local search"),
                vec![candidate(
                    "resume.md",
                    "Projects",
                    "Built a local search tool.",
                )],
            )
            .expect("retrieval");

        assert_eq!(response.answer.status, ExtractiveAnswerStatus::Answered);
        assert!(response.answer.text.contains("[1]"));
        assert_eq!(response.answer.citations[0].label, "[1]");
    }

    #[test]
    fn direct_body_support_answers_across_document_domains() {
        let cases = [
            (
                "retention period",
                "The retention period is 30 days for inactive records.",
            ),
            (
                "gpu indexing project",
                "The GPU indexing project batches document vectors on CUDA workers.",
            ),
            (
                "platform engineer role",
                "The platform engineer role owns retrieval reliability and deployment.",
            ),
            (
                "salary range",
                "The salary range is 120000 to 150000 dollars.",
            ),
            (
                "rag retrieval debugging",
                "RAG retrieval debugging compares ranked chunks, citations, and eval failures.",
            ),
        ];

        for (query, body) in cases {
            let response = LocalHybridRetriever::default()
                .retrieve(
                    lexical_request(query),
                    vec![candidate("general.md", "Details", body)],
                )
                .expect("retrieval");

            assert_eq!(response.answer.status, ExtractiveAnswerStatus::Answered);
            assert_eq!(response.answer.citations.len(), 1);
            assert_eq!(
                response.hits[0].answer_support.status,
                AnswerSupportStatus::Supported
            );
            assert_eq!(
                response.hits[0].answer_support.reason,
                AnswerSupportReason::DirectBodySupport
            );
        }
    }

    #[test]
    fn path_only_candidates_remain_ranked_but_cannot_answer() {
        let response = LocalHybridRetriever::default()
            .retrieve(
                lexical_request("salary range"),
                vec![candidate(
                    "salary-range.md",
                    "Overview",
                    "Employees receive health and leave benefits.",
                )],
            )
            .expect("retrieval");

        assert_eq!(response.hits.len(), 1);
        assert_eq!(
            response.hits[0].answer_support.reason,
            AnswerSupportReason::PathOnlyMatch
        );
        assert_insufficient_without_citations(&response);
    }

    #[test]
    fn arbitrary_unsupported_questions_abstain_without_query_specific_rules() {
        for query in [
            "orbital launch window",
            "vendor cancellation fee",
            "medical coverage limit",
        ] {
            let path = format!("{}.md", query.replace(' ', "-"));
            let response = LocalHybridRetriever::default()
                .retrieve(
                    lexical_request(query),
                    vec![candidate(
                        &path,
                        "Overview",
                        "The engineering team maintains internal dashboards.",
                    )],
                )
                .expect("retrieval");

            assert_eq!(response.hits.len(), 1);
            assert_insufficient_without_citations(&response);
        }
    }

    #[test]
    fn section_only_candidates_remain_ranked_but_cannot_answer() {
        let response = LocalHybridRetriever::default()
            .retrieve(
                lexical_request("salary range"),
                vec![candidate(
                    "employee-guide.md",
                    "Salary range",
                    "Employees receive health and leave benefits.",
                )],
            )
            .expect("retrieval");

        assert_eq!(response.hits.len(), 1);
        assert_eq!(
            response.hits[0].answer_support.reason,
            AnswerSupportReason::SectionOnlyMatch
        );
        assert_insufficient_without_citations(&response);
    }

    #[test]
    fn metadata_only_candidates_cannot_answer() {
        let mut response = LocalHybridRetriever::default()
            .retrieve(
                lexical_request("salary range"),
                vec![candidate(
                    "salary-range.md",
                    "Overview",
                    "Employees receive health and leave benefits.",
                )],
            )
            .expect("retrieval");
        let hit = response.hits.first_mut().expect("ranked candidate");
        hit.score_breakdown.semantic = 0.0;
        hit.score_breakdown.section = 0.0;
        hit.score_breakdown.path = 0.0;
        hit.score_breakdown.metadata = 0.5;
        hit.answer_support = Default::default();
        assess_hits(
            &response.run.query,
            &mut response.hits,
            &RetrievalConfig::default().answerability,
        );
        response.answer = build_extractive_answer(&response.hits, 3);

        assert_eq!(
            response.hits[0].answer_support.reason,
            AnswerSupportReason::MetadataOnlyMatch
        );
        assert_insufficient_without_citations(&response);
    }

    #[test]
    fn weak_and_heading_only_body_matches_cannot_answer() {
        let mut weak = candidate(
            "policy.md",
            "Compensation",
            "The salary range is 120000 to 150000 dollars.",
        );
        weak.chunk.evidence_score_hint = 0.1;
        let mut heading = candidate(
            "policy.md",
            "Compensation",
            "The salary range is 120000 to 150000 dollars.",
        );
        heading.chunk.quality_flags = vec![ChunkQualityFlag::HeadingOnly];

        for (candidate, expected_reason) in [
            (weak, AnswerSupportReason::WeakEvidence),
            (heading, AnswerSupportReason::HeadingOnlyEvidence),
        ] {
            let response = LocalHybridRetriever::default()
                .retrieve(lexical_request("salary range"), vec![candidate])
                .expect("retrieval");
            assert_eq!(response.hits[0].answer_support.reason, expected_reason);
            assert_insufficient_without_citations(&response);
        }
    }

    #[test]
    fn numeric_terms_must_be_supported_in_the_same_sentence() {
        let response = LocalHybridRetriever::default()
            .retrieve(
                lexical_request("2026 salary range"),
                vec![candidate(
                    "policy.md",
                    "Compensation",
                    "The salary range is 120000 to 150000 dollars. This policy applies in 2026.",
                )],
            )
            .expect("retrieval");

        assert_insufficient_without_citations(&response);
    }

    #[test]
    fn answer_citation_uses_the_sentence_that_passed_numeric_support() {
        let response = LocalHybridRetriever::default()
            .retrieve(
                lexical_request("2026 2027 salary range"),
                vec![candidate(
                    "policy.md",
                    "Compensation",
                    "The 2026 salary range is under review. The 2026 and 2027 salary values are published.",
                )],
            )
            .expect("retrieval");

        assert_eq!(response.answer.status, ExtractiveAnswerStatus::Answered);
        assert_eq!(
            response.answer.citations[0].snippet,
            "The 2026 and 2027 salary values are published"
        );
    }

    #[test]
    fn hybrid_reports_missing_embeddings_before_falling_back_to_lexical() {
        let response = LocalHybridRetriever::default()
            .retrieve(
                RetrievalQueryRequest {
                    query: "gpu indexing".to_owned(),
                    top_k: 3,
                    retrieval_mode: RetrievalMode::Hybrid,
                    source_ids: Vec::new(),
                    document_ids: Vec::new(),
                },
                vec![candidate(
                    "resume.md",
                    "Projects",
                    "Built GPU indexing experiments.",
                )],
            )
            .expect("retrieval");

        assert!(response.hits.is_empty());
        assert_eq!(
            response.embedding_status.readiness,
            RetrievalEmbeddingReadiness::Missing
        );
        assert!(response.answer.text.contains("not indexed yet"));
    }

    #[test]
    fn vector_search_can_rank_semantically_related_chunks() {
        let provider = LocalHashEmbeddingProvider::default();
        let model = provider.model();
        let query = RetrievalQueryRequest {
            query: "gpu acceleration".to_owned(),
            top_k: 3,
            retrieval_mode: RetrievalMode::Vector,
            source_ids: Vec::new(),
            document_ids: Vec::new(),
        };
        let response = LocalHybridRetriever::default()
            .retrieve(
                query,
                vec![
                    embedded_candidate(
                        &provider,
                        &model,
                        "resume.md",
                        "Projects",
                        "CUDA parallel kernels for local inference.",
                    ),
                    embedded_candidate(
                        &provider,
                        &model,
                        "resume.md",
                        "Experience",
                        "Designed React dashboards.",
                    ),
                ],
            )
            .expect("retrieval");

        assert_eq!(
            response.embedding_status.readiness,
            RetrievalEmbeddingReadiness::Ready
        );
        assert_eq!(
            response.hits[0].chunk.section_title.as_deref(),
            Some("Projects")
        );
        assert!(response.hits[0].score_breakdown.semantic > 0.0);
        assert_eq!(
            response.hits[0].answer_support.reason,
            AnswerSupportReason::SemanticOnlyMatch
        );
        assert_insufficient_without_citations(&response);
    }

    fn assert_insufficient_without_citations(response: &RetrievalQueryResponse) {
        assert_eq!(
            response.answer.status,
            ExtractiveAnswerStatus::InsufficientEvidence
        );
        assert!(response.answer.citations.is_empty());
    }

    fn lexical_request(query: &str) -> RetrievalQueryRequest {
        RetrievalQueryRequest {
            query: query.to_owned(),
            top_k: 3,
            retrieval_mode: RetrievalMode::Lexical,
            source_ids: Vec::new(),
            document_ids: Vec::new(),
        }
    }

    fn candidate(path: &str, section_title: &str, text: &str) -> SearchableChunk {
        candidate_with_embedding(path, section_title, text, None)
    }

    fn embedded_candidate(
        provider: &LocalHashEmbeddingProvider,
        model: &EmbeddingModelInfo,
        path: &str,
        section_title: &str,
        text: &str,
    ) -> SearchableChunk {
        let vector = provider.embed_one(text).expect("embedding");
        candidate_with_embedding(path, section_title, text, Some((model.clone(), vector)))
    }

    fn candidate_with_embedding(
        path: &str,
        section_title: &str,
        text: &str,
        embedding: Option<(EmbeddingModelInfo, Vec<f32>)>,
    ) -> SearchableChunk {
        let source_id = SourceId(Uuid::now_v7());
        let document_id = DocumentId(Uuid::now_v7());
        let source = Source {
            id: source_id,
            project_id: ProjectId(Uuid::now_v7()),
            name: "Browser upload".to_owned(),
            kind: SourceKind::FileSet {
                root_hint: "browser-upload".to_owned(),
            },
            sync_policy: SourceSyncPolicy::Manual,
            chunking: ChunkingConfig {
                target_tokens: 512,
                overlap_tokens: 64,
                strategy: ChunkingStrategy::SmartSections,
            },
        };
        let document = Document {
            id: document_id,
            source_id,
            path: path.to_owned(),
            mime_type: Some("text/markdown".to_owned()),
            checksum: "document".to_owned(),
            byte_size: text.len() as u64,
            profile: DocumentProfile::TechnicalDocs,
            extraction_quality: ExtractionQuality::High,
            warnings: Vec::new(),
        };
        let checksum = "1234567890abcdef".to_owned();
        let chunk = Chunk {
            id: ChunkId(Uuid::now_v7()),
            source_id,
            document_id,
            ordinal: 0,
            text: text.to_owned(),
            token_count: text.split_whitespace().count() as u32,
            byte_range: ByteRange {
                start: 0,
                end: text.len() as u64,
            },
            checksum: checksum.clone(),
            strategy: ChunkingStrategy::SmartSections,
            section_title: Some(section_title.to_owned()),
            split_reason: ChunkSplitReason::DocumentEnd,
            quality_flags: Vec::new(),
            is_duplicate: false,
            text_density: 1.0,
            evidence_score_hint: 0.8,
        };
        let embedding = embedding.map(|(model, vector)| ChunkEmbedding {
            chunk_id: chunk.id,
            chunk_checksum: checksum,
            model,
            vector,
            indexed_at: OffsetDateTime::now_utc(),
        });

        SearchableChunk {
            source,
            document,
            chunk,
            embedding,
        }
    }
}
