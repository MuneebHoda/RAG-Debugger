use rag_debugger_core::{
    AnswerSupportAssessment, AnswerSupportReason, AnswerSupportStatus, ByteRange, ChunkPreview,
    ChunkSplitReason, ChunkingStrategy, DiagnosisOutcome, DiagnosisRemediationArea, Document,
    DocumentId, DocumentProfile, EmbeddingModelInfo, EvidenceStrength, ExtractionQuality,
    ExtractiveAnswer, ExtractiveAnswerStatus, ProjectId, RetrievalCitation,
    RetrievalEmbeddingReadiness, RetrievalEmbeddingStatus, RetrievalMode, RetrievalQualityFlag,
    RetrievalQueryHit, RetrievalQueryRun, RetrievalQueryRunId, RetrievalScoreBreakdown, Source,
    SourceId, SourceKind, SourceSyncPolicy,
};
use time::OffsetDateTime;
use uuid::Uuid;

use super::*;

#[test]
fn weak_evidence_is_deterministic_and_actionable() {
    let mut response = response(vec![hit(
        1,
        0.30,
        0.20,
        0.10,
        EvidenceStrength::Weak,
        Vec::new(),
    )]);
    response.answer.status = ExtractiveAnswerStatus::InsufficientEvidence;
    response.answer.citations.clear();

    let first = diagnose_retrieval(&response, &DebuggerConfig::default(), None);
    let second = diagnose_retrieval(&response, &DebuggerConfig::default(), None);

    assert_eq!(first, second);
    assert_eq!(first.outcome, DiagnosisOutcome::Weak);
    assert_eq!(
        first.primary_issue.as_ref().map(|issue| issue.code),
        Some(DiagnosisFailureCode::WeakEvidence)
    );
    assert!(first
        .recommendations
        .iter()
        .any(|recommendation| recommendation.area == DiagnosisRemediationArea::TopK));
}

#[test]
fn duplicate_evidence_identifies_the_affected_hit() {
    let mut duplicate = hit(
        1,
        1.0,
        0.6,
        0.4,
        EvidenceStrength::Strong,
        vec![RetrievalQualityFlag::Duplicate],
    );
    duplicate.duplicate_count = 2;
    let diagnosis =
        diagnose_retrieval(&response(vec![duplicate]), &DebuggerConfig::default(), None);

    let failure = diagnosis
        .failures
        .iter()
        .find(|failure| failure.code == DiagnosisFailureCode::DuplicateEvidence)
        .expect("duplicate failure");
    assert_eq!(failure.evidence_refs, vec!["E1"]);
    assert!(diagnosis
        .recommendations
        .iter()
        .any(|recommendation| recommendation.area == DiagnosisRemediationArea::Chunking));
}

#[test]
fn low_score_margin_uses_the_configured_relative_threshold() {
    let response = response(vec![
        hit(1, 1.0, 0.6, 0.4, EvidenceStrength::Strong, Vec::new()),
        hit(2, 0.95, 0.5, 0.3, EvidenceStrength::Strong, Vec::new()),
    ]);

    let diagnosis = diagnose_retrieval(
        &response,
        &DebuggerConfig {
            low_score_margin_ratio: 0.10,
        },
        None,
    );

    assert!(diagnosis
        .failures
        .iter()
        .any(|failure| failure.code == DiagnosisFailureCode::LowScoreMargin));
}

#[test]
fn hybrid_disagreement_compares_semantic_and_lexical_leaders() {
    let response = response(vec![
        hit(1, 1.2, 1.0, 0.2, EvidenceStrength::Strong, Vec::new()),
        hit(2, 1.1, 0.1, 1.0, EvidenceStrength::Strong, Vec::new()),
    ]);

    let diagnosis = diagnose_retrieval(&response, &DebuggerConfig::default(), None);

    assert!(diagnosis
        .failures
        .iter()
        .any(|failure| failure.code == DiagnosisFailureCode::VectorLexicalDisagreement));
    assert!(diagnosis
        .recommendations
        .iter()
        .any(|recommendation| { recommendation.area == DiagnosisRemediationArea::RetrievalMode }));
}

#[test]
fn expected_evidence_is_only_checked_with_eval_context() {
    let response = response(vec![hit(
        1,
        1.0,
        0.6,
        0.4,
        EvidenceStrength::Strong,
        Vec::new(),
    )]);
    let expected_chunk = ChunkId(Uuid::from_u128(900));

    let without_expected = diagnose_retrieval(&response, &DebuggerConfig::default(), None);
    let with_expected = diagnose_retrieval(
        &response,
        &DebuggerConfig::default(),
        Some(ExpectedEvidence {
            chunk_ids: &[expected_chunk],
            document_ids: &[],
        }),
    );

    assert!(!without_expected
        .failures
        .iter()
        .any(|failure| failure.code == DiagnosisFailureCode::MissingExpectedEvidence));
    assert!(with_expected
        .failures
        .iter()
        .any(|failure| failure.code == DiagnosisFailureCode::MissingExpectedEvidence));
}

#[test]
fn citation_diagnosis_distinguishes_missing_and_uncited_top_results() {
    let hits = vec![
        hit(1, 1.0, 0.6, 0.4, EvidenceStrength::Strong, Vec::new()),
        hit(2, 0.7, 0.4, 0.3, EvidenceStrength::Strong, Vec::new()),
    ];
    let mut missing = response(hits.clone());
    missing.answer.citations.clear();
    let missing_diagnosis = diagnose_retrieval(&missing, &DebuggerConfig::default(), None);
    assert!(missing_diagnosis
        .failures
        .iter()
        .any(|failure| failure.code == DiagnosisFailureCode::CitationMissing));

    let mut uncited_top = response(hits);
    uncited_top.answer.citations = vec![uncited_top.hits[1].citation.clone()];
    let uncited_diagnosis = diagnose_retrieval(&uncited_top, &DebuggerConfig::default(), None);
    assert!(uncited_diagnosis
        .failures
        .iter()
        .any(|failure| failure.code == DiagnosisFailureCode::TopResultNotCited));
}

#[test]
fn answerability_gap_is_primary_and_maps_deterministic_recommendations() {
    let mut candidate = hit(1, 1.0, 1.0, 0.0, EvidenceStrength::Strong, Vec::new());
    candidate.answer_support = unsupported(AnswerSupportReason::SemanticOnlyMatch);
    let mut response = response(vec![candidate]);
    response.answer.status = ExtractiveAnswerStatus::InsufficientEvidence;
    response.answer.citations.clear();

    let first = diagnose_retrieval(&response, &DebuggerConfig::default(), None);
    let second = diagnose_retrieval(&response, &DebuggerConfig::default(), None);

    assert_eq!(first, second);
    assert_eq!(first.outcome, DiagnosisOutcome::Failing);
    assert_eq!(
        first.primary_issue.as_ref().map(|issue| issue.code),
        Some(DiagnosisFailureCode::AnswerabilityGap)
    );
    assert!(first
        .failures
        .iter()
        .any(|failure| failure.code == DiagnosisFailureCode::SemanticOnlyMatch));
    assert_eq!(
        first.recommendations[0].code,
        "restore_direct_answer_support"
    );
}

#[test]
fn metadata_support_reasons_share_one_diagnosis_label() {
    for reason in [
        AnswerSupportReason::MetadataOnlyMatch,
        AnswerSupportReason::PathOnlyMatch,
        AnswerSupportReason::SectionOnlyMatch,
    ] {
        let mut candidate = hit(1, 1.0, 0.0, 0.8, EvidenceStrength::Strong, Vec::new());
        candidate.answer_support = unsupported(reason);
        let mut response = response(vec![candidate]);
        response.answer.status = ExtractiveAnswerStatus::InsufficientEvidence;
        response.answer.citations.clear();

        let diagnosis = diagnose_retrieval(&response, &DebuggerConfig::default(), None);
        assert!(diagnosis
            .failures
            .iter()
            .any(|failure| failure.code == DiagnosisFailureCode::MetadataOnlyMatch));
    }
}

#[test]
fn citation_grounding_uses_highest_ranked_supported_hit() {
    let mut unsupported_top = hit(1, 1.0, 1.0, 0.0, EvidenceStrength::Strong, Vec::new());
    unsupported_top.answer_support = unsupported(AnswerSupportReason::SemanticOnlyMatch);
    let mut supported_second = hit(2, 0.9, 0.2, 0.7, EvidenceStrength::Strong, Vec::new());
    supported_second.answer_support = supported();
    let mut response = response(vec![unsupported_top, supported_second]);
    response.answer.citations = vec![response.hits[1].citation.clone()];

    let diagnosis = diagnose_retrieval(&response, &DebuggerConfig::default(), None);
    assert!(!diagnosis
        .failures
        .iter()
        .any(|failure| failure.code == DiagnosisFailureCode::TopResultNotCited));
}

fn unsupported(reason: AnswerSupportReason) -> AnswerSupportAssessment {
    AnswerSupportAssessment {
        status: AnswerSupportStatus::Unsupported,
        reason,
        matched_body_term_count: 0,
        query_term_count: 2,
        body_term_coverage: 0.0,
    }
}

fn supported() -> AnswerSupportAssessment {
    AnswerSupportAssessment {
        status: AnswerSupportStatus::Supported,
        reason: AnswerSupportReason::DirectBodySupport,
        matched_body_term_count: 2,
        query_term_count: 2,
        body_term_coverage: 1.0,
    }
}

fn response(hits: Vec<RetrievalQueryHit>) -> RetrievalQueryResponse {
    let citations = hits
        .first()
        .map(|hit| vec![hit.citation.clone()])
        .unwrap_or_default();
    RetrievalQueryResponse {
        run: RetrievalQueryRun {
            id: RetrievalQueryRunId(Uuid::from_u128(100)),
            query: "synthetic query".to_owned(),
            top_k: hits.len() as u32,
            retrieval_mode: RetrievalMode::Hybrid,
            latency_ms: 4,
            created_at: OffsetDateTime::UNIX_EPOCH,
        },
        answer: ExtractiveAnswer {
            status: ExtractiveAnswerStatus::Answered,
            text: "Synthetic cited answer.".to_owned(),
            citations,
        },
        hits,
        embedding_status: RetrievalEmbeddingStatus {
            readiness: RetrievalEmbeddingReadiness::Ready,
            required: true,
            model: EmbeddingModelInfo::default(),
            total_chunks: 2,
            indexed_chunks: 2,
            missing_chunks: 0,
            stale_chunks: 0,
        },
        diagnosis: None,
    }
}

fn hit(
    rank: u32,
    score: f32,
    semantic: f32,
    lexical: f32,
    evidence_strength: EvidenceStrength,
    quality_flags: Vec<RetrievalQualityFlag>,
) -> RetrievalQueryHit {
    let source_id = SourceId(Uuid::from_u128(200));
    let document_id = DocumentId(Uuid::from_u128(300 + rank as u128));
    let chunk_id = ChunkId(Uuid::from_u128(400 + rank as u128));
    let score_breakdown = RetrievalScoreBreakdown {
        semantic,
        lexical,
        phrase: 0.0,
        section: 0.0,
        path: 0.0,
        metadata: 0.0,
    };
    RetrievalQueryHit {
        rank,
        score,
        chunk: ChunkPreview {
            id: chunk_id,
            document_id,
            ordinal: rank - 1,
            text: format!("Synthetic evidence {rank}"),
            token_count: 3,
            byte_range: ByteRange { start: 0, end: 20 },
            checksum: format!("checksum-{rank}"),
            strategy: ChunkingStrategy::Structured,
            section_title: Some("Synthetic section".to_owned()),
            split_reason: ChunkSplitReason::DocumentEnd,
            quality_flags: Vec::new(),
            is_duplicate: false,
            text_density: 1.0,
            evidence_score_hint: 1.0,
        },
        document: Document {
            id: document_id,
            source_id,
            path: format!("document-{rank}.md"),
            mime_type: Some("text/markdown".to_owned()),
            checksum: format!("document-checksum-{rank}"),
            byte_size: 20,
            profile: DocumentProfile::TechnicalDocs,
            extraction_quality: ExtractionQuality::High,
            warnings: Vec::new(),
        },
        source: Source {
            id: source_id,
            project_id: ProjectId(Uuid::from_u128(500)),
            name: "Synthetic corpus".to_owned(),
            kind: SourceKind::FileSet {
                root_hint: "synthetic".to_owned(),
            },
            sync_policy: SourceSyncPolicy::Manual,
            chunking: Default::default(),
        },
        matched_terms: Vec::new(),
        score_breakdown,
        normalized_score_breakdown: score_breakdown,
        snippet: format!("Synthetic evidence {rank}"),
        citation: RetrievalCitation {
            label: format!("[{rank}]"),
            chunk_id,
            document_id,
            document_path: format!("document-{rank}.md"),
            chunk_ordinal: rank - 1,
            section_title: Some("Synthetic section".to_owned()),
            checksum_prefix: format!("checksum-{rank}"),
            snippet: format!("Synthetic evidence {rank}"),
        },
        quality_flags,
        evidence_strength,
        duplicate_count: 1,
        answer_support: Default::default(),
    }
}
