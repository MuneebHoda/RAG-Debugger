use rag_debugger_core::{
    AnswerSupportReason, AnswerSupportStatus, DebuggerConfig, DiagnosisFailure,
    DiagnosisFailureCode, DiagnosisOutcome, DiagnosisScoreSignal, DiagnosisSeverity,
    EvidenceScoreExplanation, EvidenceStrength, ExtractiveAnswerStatus,
    RetrievalEmbeddingReadiness, RetrievalMode, RetrievalQualityFlag, RetrievalQueryHit,
    RetrievalQueryResponse, RetrievalScoreBreakdown,
};

use super::ExpectedEvidence;

pub(super) fn collect_failures(
    response: &RetrievalQueryResponse,
    config: &DebuggerConfig,
    expected: Option<ExpectedEvidence<'_>>,
) -> Vec<DiagnosisFailure> {
    let mut failures = Vec::new();
    diagnose_availability(response, &mut failures);
    diagnose_evidence(response, config, &mut failures);
    diagnose_citations(response, &mut failures);
    if let Some(expected) = expected {
        diagnose_expected_evidence(response, expected, &mut failures);
    }
    failures.sort_by_key(|failure| {
        (
            severity_order(failure.severity),
            failure_code_order(failure.code),
        )
    });
    failures.dedup_by_key(|failure| failure.code);
    failures
}

pub(super) fn score_explanations(hits: &[RetrievalQueryHit]) -> Vec<EvidenceScoreExplanation> {
    hits.iter()
        .enumerate()
        .map(|(index, hit)| {
            let dominant_signal = dominant_signal(hit.score_breakdown);
            EvidenceScoreExplanation {
                evidence_ref: evidence_ref(hit),
                chunk_id: hit.chunk.id,
                rank: hit.rank,
                final_score: hit.score,
                score_delta_from_previous: index
                    .checked_sub(1)
                    .map(|previous| hits[previous].score - hit.score),
                score_delta_to_next: hits.get(index + 1).map(|next| hit.score - next.score),
                dominant_signal,
                score_breakdown: hit.score_breakdown,
                normalized_score_breakdown: hit.normalized_score_breakdown,
                summary: format!(
                    "Ranked #{} with {} as the strongest scoring signal.",
                    hit.rank,
                    dominant_signal.label()
                ),
            }
        })
        .collect()
}

pub(super) fn diagnosis_outcome(
    response: &RetrievalQueryResponse,
    failures: &[DiagnosisFailure],
) -> DiagnosisOutcome {
    if failures.iter().any(|failure| {
        matches!(
            failure.code,
            DiagnosisFailureCode::MissingDocument
                | DiagnosisFailureCode::MissingEmbeddingIndex
                | DiagnosisFailureCode::MissingExpectedEvidence
                | DiagnosisFailureCode::AnswerabilityGap
        ) && failure.severity == DiagnosisSeverity::Critical
    }) {
        return DiagnosisOutcome::Failing;
    }
    if failures.iter().any(|failure| {
        matches!(
            failure.code,
            DiagnosisFailureCode::WeakEvidence | DiagnosisFailureCode::CitationMissing
        ) && failure.severity == DiagnosisSeverity::Critical
    }) {
        return DiagnosisOutcome::Weak;
    }
    if !failures.is_empty()
        || response
            .hits
            .first()
            .is_some_and(|hit| hit.evidence_strength != EvidenceStrength::Strong)
    {
        return DiagnosisOutcome::Mixed;
    }
    DiagnosisOutcome::Strong
}

pub(super) fn diagnosis_summary(
    outcome: DiagnosisOutcome,
    primary_issue: Option<&DiagnosisFailure>,
    hit_count: usize,
) -> String {
    match primary_issue {
        Some(issue) => format!(
            "This run looks {}. Primary issue: {}",
            outcome.as_str(),
            issue.title
        ),
        None => format!(
            "This run looks strong. CorpusLab found {hit_count} ranked evidence item(s) without a deterministic failure signal."
        ),
    }
}

fn diagnose_availability(response: &RetrievalQueryResponse, failures: &mut Vec<DiagnosisFailure>) {
    if response.hits.is_empty()
        && response.embedding_status.readiness != RetrievalEmbeddingReadiness::Missing
    {
        failures.push(failure(
            DiagnosisFailureCode::MissingDocument,
            DiagnosisSeverity::Critical,
            "No evidence retrieved",
            "The retrieval run returned no evidence that could support an answer.",
            Vec::new(),
        ));
    }

    if response.embedding_status.required {
        match response.embedding_status.readiness {
            RetrievalEmbeddingReadiness::Missing => failures.push(failure(
                DiagnosisFailureCode::MissingEmbeddingIndex,
                DiagnosisSeverity::Critical,
                "Embedding index missing",
                "Semantic retrieval could not run because required chunk embeddings are missing.",
                Vec::new(),
            )),
            RetrievalEmbeddingReadiness::Partial => failures.push(failure(
                DiagnosisFailureCode::PartialEmbeddingIndex,
                DiagnosisSeverity::Warning,
                "Embedding index incomplete",
                "Some chunks were unavailable to semantic retrieval, so relevant evidence may have been skipped.",
                Vec::new(),
            )),
            RetrievalEmbeddingReadiness::NotRequired | RetrievalEmbeddingReadiness::Ready => {}
        }
    }
}

fn diagnose_evidence(
    response: &RetrievalQueryResponse,
    config: &DebuggerConfig,
    failures: &mut Vec<DiagnosisFailure>,
) {
    let supported_refs = response
        .hits
        .iter()
        .filter(|hit| hit.answer_support.status == AnswerSupportStatus::Supported)
        .map(evidence_ref)
        .collect::<Vec<_>>();
    let answerability_assessed = response
        .hits
        .iter()
        .all(|hit| hit.answer_support.status != AnswerSupportStatus::Unassessed);
    if !response.hits.is_empty() && answerability_assessed && supported_refs.is_empty() {
        failures.push(failure(
            DiagnosisFailureCode::AnswerabilityGap,
            DiagnosisSeverity::Critical,
            "Retrieved candidates cannot support an answer",
            "Ranked chunks were found, but none contains enough direct body-text support for the question.",
            response.hits.iter().map(evidence_ref).collect(),
        ));
    }

    let semantic_only_refs = response
        .hits
        .iter()
        .filter(|hit| hit.answer_support.reason == AnswerSupportReason::SemanticOnlyMatch)
        .map(evidence_ref)
        .collect::<Vec<_>>();
    if !semantic_only_refs.is_empty() {
        failures.push(failure(
            DiagnosisFailureCode::SemanticOnlyMatch,
            DiagnosisSeverity::Warning,
            "Semantic similarity lacks direct support",
            "Semantic scoring retrieved candidates whose body text does not directly support the question.",
            semantic_only_refs,
        ));
    }

    let metadata_only_refs = response
        .hits
        .iter()
        .filter(|hit| {
            matches!(
                hit.answer_support.reason,
                AnswerSupportReason::MetadataOnlyMatch
                    | AnswerSupportReason::PathOnlyMatch
                    | AnswerSupportReason::SectionOnlyMatch
            )
        })
        .map(evidence_ref)
        .collect::<Vec<_>>();
    if !metadata_only_refs.is_empty() {
        failures.push(failure(
            DiagnosisFailureCode::MetadataOnlyMatch,
            DiagnosisSeverity::Warning,
            "Metadata matched without body support",
            "Path, section, or metadata signals retrieved candidates whose body text does not support the question.",
            metadata_only_refs,
        ));
    }

    let weak_refs = response
        .hits
        .iter()
        .filter(|hit| hit.evidence_strength == EvidenceStrength::Weak)
        .map(evidence_ref)
        .collect::<Vec<_>>();
    if !weak_refs.is_empty() {
        let top_is_weak = response
            .hits
            .first()
            .is_some_and(|hit| hit.evidence_strength == EvidenceStrength::Weak);
        failures.push(failure(
            DiagnosisFailureCode::WeakEvidence,
            if top_is_weak {
                DiagnosisSeverity::Critical
            } else {
                DiagnosisSeverity::Warning
            },
            "Evidence is too weak",
            "The returned evidence does not provide a sufficiently strong basis for a defensible answer.",
            weak_refs,
        ));
    }

    let duplicate_refs = response
        .hits
        .iter()
        .filter(|hit| {
            hit.duplicate_count > 1 || hit.quality_flags.contains(&RetrievalQualityFlag::Duplicate)
        })
        .map(evidence_ref)
        .collect::<Vec<_>>();
    if !duplicate_refs.is_empty() {
        failures.push(failure(
            DiagnosisFailureCode::DuplicateEvidence,
            DiagnosisSeverity::Warning,
            "Duplicate evidence crowded the ranking",
            "Repeated chunks reduced the diversity of evidence available in the result set.",
            duplicate_refs,
        ));
    }

    let heading_refs = response
        .hits
        .iter()
        .filter(|hit| {
            hit.quality_flags
                .contains(&RetrievalQualityFlag::HeadingOnly)
        })
        .map(evidence_ref)
        .collect::<Vec<_>>();
    if !heading_refs.is_empty() {
        failures.push(failure(
            DiagnosisFailureCode::HeadingOnlyEvidence,
            DiagnosisSeverity::Warning,
            "Heading-only evidence ranked",
            "A heading ranked without enough supporting text to serve as reliable evidence.",
            heading_refs,
        ));
    }

    if let [top, second, ..] = response.hits.as_slice() {
        if top.score > f32::EPSILON {
            let margin_ratio = ((top.score - second.score) / top.score).max(0.0);
            if margin_ratio <= config.low_score_margin_ratio {
                failures.push(failure(
                    DiagnosisFailureCode::LowScoreMargin,
                    DiagnosisSeverity::Warning,
                    "Top results are difficult to distinguish",
                    "The leading chunks have nearly equal scores, so small scoring changes could reverse their order.",
                    vec![evidence_ref(top), evidence_ref(second)],
                ));
            }
        }
    }

    if response.run.retrieval_mode == RetrievalMode::Hybrid {
        if let (Some(semantic_leader), Some(lexical_leader)) = (
            semantic_leader(&response.hits),
            lexical_leader(&response.hits),
        ) {
            if semantic_leader.chunk.id != lexical_leader.chunk.id {
                failures.push(failure(
                    DiagnosisFailureCode::VectorLexicalDisagreement,
                    DiagnosisSeverity::Warning,
                    "Vector and lexical signals disagree",
                    "Semantic and lexical scoring prefer different leading chunks in this hybrid run.",
                    vec![evidence_ref(semantic_leader), evidence_ref(lexical_leader)],
                ));
            }
        }
    }
}

fn diagnose_citations(response: &RetrievalQueryResponse, failures: &mut Vec<DiagnosisFailure>) {
    if response.answer.status != ExtractiveAnswerStatus::Answered || response.hits.is_empty() {
        return;
    }
    if response.answer.citations.is_empty() {
        failures.push(failure(
            DiagnosisFailureCode::CitationMissing,
            DiagnosisSeverity::Critical,
            "Answer has no citations",
            "The evidence summary produced an answer without a verifiable citation.",
            response
                .hits
                .first()
                .map(evidence_ref)
                .into_iter()
                .collect(),
        ));
        return;
    }

    let top = response
        .hits
        .iter()
        .find(|hit| hit.answer_support.status == AnswerSupportStatus::Supported)
        .unwrap_or(&response.hits[0]);
    if !response
        .answer
        .citations
        .iter()
        .any(|citation| citation.chunk_id == top.chunk.id)
    {
        failures.push(failure(
            DiagnosisFailureCode::TopResultNotCited,
            DiagnosisSeverity::Warning,
            "Top-ranked evidence was not cited",
            "The answer cited evidence, but it did not use the highest-ranked chunk.",
            vec![evidence_ref(top)],
        ));
    }
}

fn diagnose_expected_evidence(
    response: &RetrievalQueryResponse,
    expected: ExpectedEvidence<'_>,
    failures: &mut Vec<DiagnosisFailure>,
) {
    let missing_chunks = expected
        .chunk_ids
        .iter()
        .filter(|chunk_id| !response.hits.iter().any(|hit| hit.chunk.id == **chunk_id))
        .map(|chunk_id| format!("chunk:{}", chunk_id.0));
    let missing_documents = expected
        .document_ids
        .iter()
        .filter(|document_id| {
            !response
                .hits
                .iter()
                .any(|hit| hit.document.id == **document_id)
        })
        .map(|document_id| format!("document:{}", document_id.0));
    let missing_refs = missing_chunks.chain(missing_documents).collect::<Vec<_>>();
    if missing_refs.is_empty() {
        return;
    }

    let expected_count = expected.chunk_ids.len() + expected.document_ids.len();
    failures.push(failure(
        DiagnosisFailureCode::MissingExpectedEvidence,
        if missing_refs.len() == expected_count {
            DiagnosisSeverity::Critical
        } else {
            DiagnosisSeverity::Warning
        },
        "Expected evidence was not retrieved",
        "One or more expected chunks or documents were absent from the ranked evidence.",
        missing_refs,
    ));
}

fn dominant_signal(breakdown: RetrievalScoreBreakdown) -> DiagnosisScoreSignal {
    let signals = [
        (DiagnosisScoreSignal::Semantic, breakdown.semantic),
        (DiagnosisScoreSignal::Lexical, breakdown.lexical),
        (DiagnosisScoreSignal::Phrase, breakdown.phrase),
        (DiagnosisScoreSignal::Section, breakdown.section),
        (DiagnosisScoreSignal::Path, breakdown.path),
        (DiagnosisScoreSignal::Metadata, breakdown.metadata),
    ];
    signals
        .into_iter()
        .fold((DiagnosisScoreSignal::None, 0.0_f32), |leader, next| {
            if next.1 > leader.1 {
                next
            } else {
                leader
            }
        })
        .0
}

fn semantic_leader(hits: &[RetrievalQueryHit]) -> Option<&RetrievalQueryHit> {
    signal_leader(hits, |hit| hit.score_breakdown.semantic)
}

fn lexical_leader(hits: &[RetrievalQueryHit]) -> Option<&RetrievalQueryHit> {
    signal_leader(hits, |hit| {
        hit.score_breakdown.lexical
            + hit.score_breakdown.phrase
            + hit.score_breakdown.section
            + hit.score_breakdown.path
            + hit.score_breakdown.metadata
    })
}

fn signal_leader(
    hits: &[RetrievalQueryHit],
    score: impl Fn(&RetrievalQueryHit) -> f32,
) -> Option<&RetrievalQueryHit> {
    let mut leader = None;
    let mut leader_score = 0.0_f32;
    for hit in hits {
        let candidate_score = score(hit);
        if candidate_score > leader_score {
            leader = Some(hit);
            leader_score = candidate_score;
        }
    }
    leader
}

fn failure(
    code: DiagnosisFailureCode,
    severity: DiagnosisSeverity,
    title: &str,
    summary: &str,
    evidence_refs: Vec<String>,
) -> DiagnosisFailure {
    DiagnosisFailure {
        code,
        severity,
        title: title.to_owned(),
        summary: summary.to_owned(),
        evidence_refs,
    }
}

fn evidence_ref(hit: &RetrievalQueryHit) -> String {
    format!("E{}", hit.rank)
}

fn severity_order(severity: DiagnosisSeverity) -> u8 {
    match severity {
        DiagnosisSeverity::Critical => 0,
        DiagnosisSeverity::Warning => 1,
        DiagnosisSeverity::Info => 2,
    }
}

fn failure_code_order(code: DiagnosisFailureCode) -> u8 {
    match code {
        DiagnosisFailureCode::MissingDocument => 0,
        DiagnosisFailureCode::MissingEmbeddingIndex => 1,
        DiagnosisFailureCode::MissingExpectedEvidence => 2,
        DiagnosisFailureCode::AnswerabilityGap => 3,
        DiagnosisFailureCode::CitationMissing => 4,
        DiagnosisFailureCode::WeakEvidence => 5,
        DiagnosisFailureCode::PartialEmbeddingIndex => 6,
        DiagnosisFailureCode::SemanticOnlyMatch => 7,
        DiagnosisFailureCode::MetadataOnlyMatch => 8,
        DiagnosisFailureCode::DuplicateEvidence => 9,
        DiagnosisFailureCode::HeadingOnlyEvidence => 10,
        DiagnosisFailureCode::LowScoreMargin => 11,
        DiagnosisFailureCode::VectorLexicalDisagreement => 12,
        DiagnosisFailureCode::TopResultNotCited => 13,
    }
}
