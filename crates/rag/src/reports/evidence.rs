use std::collections::{HashMap, HashSet};

use rag_debugger_core::{
    DebugReportEvidenceRef, DebugReportEvidenceRole, RetrievalEvalCaseId, RetrievalEvalExperiment,
    RetrievalEvalFailureLabel,
};

pub(super) fn experiment_evidence(
    experiment: &RetrievalEvalExperiment,
) -> (
    Vec<DebugReportEvidenceRef>,
    HashMap<RetrievalEvalCaseId, Vec<String>>,
) {
    let mut evidence = Vec::new();
    let mut labels_by_key = HashMap::new();
    let mut case_evidence = HashMap::<RetrievalEvalCaseId, Vec<String>>::new();

    for mode in &experiment.mode_results {
        for case in &mode.case_results {
            let retrieved = case
                .retrieved_chunk_ids
                .iter()
                .copied()
                .collect::<HashSet<_>>();
            for chunk_id in &case.retrieved_chunk_ids {
                add_evidence(
                    &mut evidence,
                    &mut labels_by_key,
                    &mut case_evidence,
                    case.case_id,
                    format!("retrieved:chunk:{}", chunk_id.0),
                    DebugReportEvidenceRef {
                        label: String::new(),
                        role: DebugReportEvidenceRole::Retrieved,
                        source_id: None,
                        document_id: None,
                        chunk_id: Some(*chunk_id),
                        rank: None,
                        document_path: None,
                        section_title: None,
                        checksum_prefix: None,
                        citation_label: None,
                        snippet: None,
                        evidence_strength: None,
                        chunk_quality_flags: Vec::new(),
                        retrieval_quality_flags: Vec::new(),
                    },
                );
            }
            for chunk_id in &case.expected_chunk_ids {
                let role = if retrieved.contains(chunk_id) {
                    DebugReportEvidenceRole::Expected
                } else {
                    DebugReportEvidenceRole::Missing
                };
                add_evidence(
                    &mut evidence,
                    &mut labels_by_key,
                    &mut case_evidence,
                    case.case_id,
                    format!("{role:?}:chunk:{}", chunk_id.0),
                    DebugReportEvidenceRef {
                        label: String::new(),
                        role,
                        source_id: None,
                        document_id: None,
                        chunk_id: Some(*chunk_id),
                        rank: case.top_hit_rank,
                        document_path: None,
                        section_title: None,
                        checksum_prefix: None,
                        citation_label: None,
                        snippet: None,
                        evidence_strength: None,
                        chunk_quality_flags: Vec::new(),
                        retrieval_quality_flags: Vec::new(),
                    },
                );
            }
            let missing_all = case
                .failures
                .iter()
                .any(|failure| failure.label == RetrievalEvalFailureLabel::ExpectedEvidenceMissing);
            for document_id in &case.expected_document_ids {
                let role = if missing_all {
                    DebugReportEvidenceRole::Missing
                } else {
                    DebugReportEvidenceRole::Expected
                };
                add_evidence(
                    &mut evidence,
                    &mut labels_by_key,
                    &mut case_evidence,
                    case.case_id,
                    format!("{role:?}:document:{}", document_id.0),
                    DebugReportEvidenceRef {
                        label: String::new(),
                        role,
                        source_id: None,
                        document_id: Some(*document_id),
                        chunk_id: None,
                        rank: case.top_hit_rank,
                        document_path: None,
                        section_title: None,
                        checksum_prefix: None,
                        citation_label: None,
                        snippet: None,
                        evidence_strength: None,
                        chunk_quality_flags: Vec::new(),
                        retrieval_quality_flags: Vec::new(),
                    },
                );
            }
        }
    }
    (evidence, case_evidence)
}

fn add_evidence(
    evidence: &mut Vec<DebugReportEvidenceRef>,
    labels_by_key: &mut HashMap<String, String>,
    case_evidence: &mut HashMap<RetrievalEvalCaseId, Vec<String>>,
    case_id: RetrievalEvalCaseId,
    key: String,
    mut reference: DebugReportEvidenceRef,
) {
    let label = if let Some(label) = labels_by_key.get(&key) {
        label.clone()
    } else {
        let label = format!("E{}", evidence.len() + 1);
        reference.label = label.clone();
        evidence.push(reference);
        labels_by_key.insert(key, label.clone());
        label
    };
    let labels = case_evidence.entry(case_id).or_default();
    if !labels.contains(&label) {
        labels.push(label);
    }
}
