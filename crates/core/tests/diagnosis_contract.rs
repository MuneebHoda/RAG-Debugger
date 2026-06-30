use rag_debugger_core::{
    ChunkId, DiagnosisFailure, DiagnosisFailureCode, DiagnosisOutcome, DiagnosisRecommendation,
    DiagnosisRecommendationPriority, DiagnosisRemediationArea, DiagnosisScoreSignal,
    DiagnosisSeverity, EvidenceDiagnosisSummary, EvidenceScoreExplanation, RetrievalScoreBreakdown,
};
use uuid::Uuid;

#[test]
fn diagnosis_round_trips_with_stable_wire_values() {
    let diagnosis = fixture();

    let value = serde_json::to_value(&diagnosis).expect("diagnosis serializes");
    let decoded: EvidenceDiagnosisSummary =
        serde_json::from_value(value.clone()).expect("diagnosis deserializes");

    assert_eq!(decoded, diagnosis);
    assert_eq!(value["outcome"], "mixed");
    assert_eq!(value["failures"][0]["code"], "low_score_margin");
    assert_eq!(value["failures"][0]["severity"], "warning");
    assert_eq!(value["recommendations"][0]["area"], "reranking");
    assert_eq!(value["score_explanations"][0]["dominant_signal"], "lexical");
}

#[test]
fn optional_collections_default_for_older_diagnosis_json() {
    let value = serde_json::json!({
        "outcome": "strong",
        "summary": "No deterministic failure signal.",
        "primary_issue": null
    });

    let decoded: EvidenceDiagnosisSummary =
        serde_json::from_value(value).expect("legacy diagnosis deserializes");

    assert!(decoded.failures.is_empty());
    assert!(decoded.score_explanations.is_empty());
    assert!(decoded.recommendations.is_empty());
}

fn fixture() -> EvidenceDiagnosisSummary {
    let failure = DiagnosisFailure {
        code: DiagnosisFailureCode::LowScoreMargin,
        severity: DiagnosisSeverity::Warning,
        title: "Top results are difficult to distinguish".to_owned(),
        summary: "The two leading scores are close.".to_owned(),
        evidence_refs: vec!["E1".to_owned(), "E2".to_owned()],
    };
    EvidenceDiagnosisSummary {
        outcome: DiagnosisOutcome::Mixed,
        summary: "This run looks mixed.".to_owned(),
        primary_issue: Some(failure.clone()),
        failures: vec![failure],
        score_explanations: vec![EvidenceScoreExplanation {
            evidence_ref: "E1".to_owned(),
            chunk_id: ChunkId(Uuid::from_u128(1)),
            rank: 1,
            final_score: 1.0,
            score_delta_from_previous: None,
            score_delta_to_next: Some(0.05),
            dominant_signal: DiagnosisScoreSignal::Lexical,
            score_breakdown: RetrievalScoreBreakdown {
                semantic: 0.2,
                lexical: 0.8,
                phrase: 0.0,
                section: 0.0,
                path: 0.0,
                metadata: 0.0,
            },
            normalized_score_breakdown: RetrievalScoreBreakdown {
                semantic: 0.25,
                lexical: 1.0,
                phrase: 0.0,
                section: 0.0,
                path: 0.0,
                metadata: 0.0,
            },
            summary: "Ranked #1 with lexical overlap as the strongest scoring signal.".to_owned(),
        }],
        recommendations: vec![DiagnosisRecommendation {
            code: "add_reranking".to_owned(),
            priority: DiagnosisRecommendationPriority::Medium,
            area: DiagnosisRemediationArea::Reranking,
            title: "Add a reranking stage".to_owned(),
            rationale: "The top scores are close.".to_owned(),
            action: "Rerank the candidate set.".to_owned(),
            failure_codes: vec![DiagnosisFailureCode::LowScoreMargin],
            evidence_refs: vec!["E1".to_owned(), "E2".to_owned()],
        }],
    }
}
