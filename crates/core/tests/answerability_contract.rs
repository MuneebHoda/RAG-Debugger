use rag_debugger_core::{
    AnswerSupportAssessment, AnswerSupportReason, AnswerSupportStatus, AnswerabilityConfig,
    RetrievalConfig,
};

#[test]
fn answer_support_uses_stable_snake_case_wire_values() {
    let assessment = AnswerSupportAssessment {
        status: AnswerSupportStatus::Unsupported,
        reason: AnswerSupportReason::SemanticOnlyMatch,
        matched_body_term_count: 0,
        query_term_count: 3,
        body_term_coverage: 0.0,
    };

    let json = serde_json::to_value(&assessment).expect("serialize answer support");
    assert_eq!(json["status"], "unsupported");
    assert_eq!(json["reason"], "semantic_only_match");
    assert_eq!(
        serde_json::from_value::<AnswerSupportAssessment>(json)
            .expect("deserialize answer support"),
        assessment
    );
}

#[test]
fn answerability_config_has_conservative_defaults_and_legacy_defaulting() {
    assert_eq!(
        AnswerabilityConfig::default(),
        AnswerabilityConfig {
            min_body_term_coverage: 0.5,
            min_body_term_matches: 2,
        }
    );

    let mut json = serde_json::to_value(RetrievalConfig::default()).expect("serialize config");
    json.as_object_mut()
        .expect("config object")
        .remove("answerability");
    let decoded: RetrievalConfig = serde_json::from_value(json).expect("legacy config");
    assert_eq!(decoded.answerability, AnswerabilityConfig::default());
}
