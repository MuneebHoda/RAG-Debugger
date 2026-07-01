mod comparison;
mod recommendations;
mod rules;

use rag_debugger_core::{
    ChunkId, DebuggerConfig, DiagnosisFailureCode, EvidenceDiagnosisSummary, FailureLabel,
    RetrievalQueryResponse,
};

pub use comparison::compare_diagnoses;

pub struct ExpectedEvidence<'a> {
    pub chunk_ids: &'a [ChunkId],
    pub document_ids: &'a [rag_debugger_core::DocumentId],
}

pub fn attach_diagnosis(
    mut response: RetrievalQueryResponse,
    config: &DebuggerConfig,
) -> RetrievalQueryResponse {
    response.diagnosis = Some(diagnose_retrieval(&response, config, None));
    response
}

pub fn diagnose_retrieval(
    response: &RetrievalQueryResponse,
    config: &DebuggerConfig,
    expected: Option<ExpectedEvidence<'_>>,
) -> EvidenceDiagnosisSummary {
    let failures = rules::collect_failures(response, config, expected);
    let outcome = rules::diagnosis_outcome(response, &failures);
    let recommendations = recommendations::recommendations_for(&failures);
    let primary_issue = failures.first().cloned();
    let summary = rules::diagnosis_summary(outcome, primary_issue.as_ref(), response.hits.len());

    EvidenceDiagnosisSummary {
        outcome,
        summary,
        primary_issue,
        failures,
        score_explanations: rules::score_explanations(&response.hits),
        recommendations,
    }
}

pub fn legacy_failure_labels(diagnosis: &EvidenceDiagnosisSummary) -> Vec<FailureLabel> {
    let mut labels = Vec::new();
    for failure in &diagnosis.failures {
        match failure.code {
            DiagnosisFailureCode::MissingDocument => {
                push_unique(&mut labels, FailureLabel::MissingDocument)
            }
            DiagnosisFailureCode::MissingEmbeddingIndex => {
                push_unique(&mut labels, FailureLabel::MissingEmbeddingIndex);
                push_unique(&mut labels, FailureLabel::BadEmbedding);
            }
            DiagnosisFailureCode::PartialEmbeddingIndex => {
                push_unique(&mut labels, FailureLabel::BadEmbedding)
            }
            DiagnosisFailureCode::WeakEvidence => {
                push_unique(&mut labels, FailureLabel::WeakEvidence);
                push_unique(&mut labels, FailureLabel::BadRanking);
            }
            DiagnosisFailureCode::DuplicateEvidence => {
                push_unique(&mut labels, FailureLabel::DuplicateEvidence);
                push_unique(&mut labels, FailureLabel::BadChunking);
            }
            DiagnosisFailureCode::HeadingOnlyEvidence => {
                push_unique(&mut labels, FailureLabel::HeadingOnlyEvidence);
                push_unique(&mut labels, FailureLabel::BadChunking);
            }
            DiagnosisFailureCode::LowScoreMargin
            | DiagnosisFailureCode::VectorLexicalDisagreement => {
                push_unique(&mut labels, FailureLabel::BadRanking)
            }
            DiagnosisFailureCode::CitationMissing
            | DiagnosisFailureCode::TopResultNotCited
            | DiagnosisFailureCode::MissingExpectedEvidence => {}
        }
    }
    labels
}

fn push_unique<T: PartialEq>(values: &mut Vec<T>, value: T) {
    if !values.contains(&value) {
        values.push(value);
    }
}

#[cfg(test)]
mod tests;
