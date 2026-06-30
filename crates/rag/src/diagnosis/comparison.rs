use std::collections::HashSet;

use rag_debugger_core::{
    ChunkId, RerunDiagnosisSummary, RetrievalQueryHit, RetrievalQueryResponse,
};

pub fn compare_diagnoses(
    before: &RetrievalQueryResponse,
    after: &RetrievalQueryResponse,
) -> Option<RerunDiagnosisSummary> {
    let before_diagnosis = before.diagnosis.as_ref()?;
    let after_diagnosis = after.diagnosis.as_ref()?;
    let before_codes = before_diagnosis
        .failures
        .iter()
        .map(|failure| failure.code)
        .collect::<HashSet<_>>();
    let after_codes = after_diagnosis
        .failures
        .iter()
        .map(|failure| failure.code)
        .collect::<HashSet<_>>();
    let resolved_failures = before_diagnosis
        .failures
        .iter()
        .map(|failure| failure.code)
        .filter(|code| !after_codes.contains(code))
        .collect::<Vec<_>>();
    let introduced_failures = after_diagnosis
        .failures
        .iter()
        .map(|failure| failure.code)
        .filter(|code| !before_codes.contains(code))
        .collect::<Vec<_>>();

    let before_evidence = chunk_ids(&before.hits);
    let after_evidence = chunk_ids(&after.hits);
    let before_citations = citation_chunk_ids(before);
    let after_citations = citation_chunk_ids(after);
    let gained_evidence = ordered_difference(&after_evidence, &before_evidence);
    let lost_evidence = ordered_difference(&before_evidence, &after_evidence);
    let gained_citations = ordered_difference(&after_citations, &before_citations);
    let lost_citations = ordered_difference(&before_citations, &after_citations);
    let summary = format!(
        "The rerun changed diagnosis from {} to {}, resolving {} signal(s) and introducing {} signal(s).",
        before_diagnosis.outcome.as_str(),
        after_diagnosis.outcome.as_str(),
        resolved_failures.len(),
        introduced_failures.len()
    );

    Some(RerunDiagnosisSummary {
        before_outcome: before_diagnosis.outcome,
        after_outcome: after_diagnosis.outcome,
        summary,
        resolved_failures,
        introduced_failures,
        gained_evidence,
        lost_evidence,
        gained_citations,
        lost_citations,
    })
}

fn chunk_ids(hits: &[RetrievalQueryHit]) -> Vec<ChunkId> {
    hits.iter().map(|hit| hit.chunk.id).collect()
}

fn citation_chunk_ids(response: &RetrievalQueryResponse) -> Vec<ChunkId> {
    response
        .answer
        .citations
        .iter()
        .map(|citation| citation.chunk_id)
        .collect()
}

fn ordered_difference(left: &[ChunkId], right: &[ChunkId]) -> Vec<ChunkId> {
    let right = right.iter().copied().collect::<HashSet<_>>();
    left.iter()
        .copied()
        .filter(|id| !right.contains(id))
        .collect()
}
