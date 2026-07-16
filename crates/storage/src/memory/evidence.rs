use std::{cmp::Ordering, collections::BinaryHeap, collections::HashSet};

use async_trait::async_trait;
use rag_debugger_core::{
    Chunk, ChunkId, Document, DocumentId, EvalLabEvidenceChunk, EvalLabEvidenceDocument,
    EvalLabEvidenceSearchQuery, EvalLabEvidenceSearchRequest, EvalLabEvidenceSearchResult, Source,
    EVAL_LAB_EVIDENCE_PREVIEW_CHAR_LIMIT,
};
use uuid::Uuid;

use super::{MemoryStore, MemoryStoreInner};
use crate::{repository::EvidenceRepository, StorageError};

#[async_trait]
impl EvidenceRepository for MemoryStore {
    async fn resolve_evidence_documents(
        &self,
        document_ids: &[DocumentId],
    ) -> Result<Vec<EvalLabEvidenceDocument>, StorageError> {
        let inner = self.lock()?;
        Ok(deduplicated(document_ids)
            .filter_map(|document_id| {
                inner
                    .documents
                    .get(document_id)
                    .and_then(|document| evidence_document(&inner, document))
            })
            .collect())
    }

    async fn resolve_evidence_chunks(
        &self,
        chunk_ids: &[ChunkId],
    ) -> Result<Vec<EvalLabEvidenceChunk>, StorageError> {
        let inner = self.lock()?;
        Ok(deduplicated(chunk_ids)
            .filter_map(|chunk_id| {
                inner
                    .chunks_by_id
                    .get(chunk_id)
                    .and_then(|chunk| evidence_chunk(&inner, chunk))
            })
            .collect())
    }

    async fn search_evidence(
        &self,
        request: &EvalLabEvidenceSearchRequest,
    ) -> Result<EvalLabEvidenceSearchResult, StorageError> {
        if request.document_limit == 0 && request.chunk_limit == 0 {
            return Ok(EvalLabEvidenceSearchResult::default());
        }

        // Memory search preserves one consistent corpus snapshot while scanning. CPU is
        // O(D log document_limit + C log chunk_limit), and temporary candidate memory is
        // O(document_limit + chunk_limit). Browse and text modes still scan the in-memory
        // corpus under this lock; the heap only bounds retained candidates.
        let inner = self.lock()?;
        let excluded_documents = request
            .excluded_document_ids
            .iter()
            .copied()
            .collect::<HashSet<_>>();
        let excluded_chunks = request
            .excluded_chunk_ids
            .iter()
            .copied()
            .collect::<HashSet<_>>();
        let mut document_winners = BoundedTopK::new(request.document_limit as usize);
        if request.document_limit > 0 {
            for document in inner.documents.values() {
                if excluded_documents.contains(&document.id) {
                    continue;
                }
                let Some(source) = inner.sources.get(&document.source_id) else {
                    continue;
                };
                let Some(priority) =
                    evidence_document_match_priority(document, source, &request.query)
                else {
                    continue;
                };
                let source_order = match &request.query {
                    EvalLabEvidenceSearchQuery::Browse => String::new(),
                    _ => source.name.to_lowercase(),
                };
                document_winners.push(
                    (
                        priority,
                        document.path.to_lowercase(),
                        source_order,
                        document.id.0,
                    ),
                    document.id.0,
                );
            }
        }

        let mut chunk_winners = BoundedTopK::new(request.chunk_limit as usize);
        if request.chunk_limit > 0 {
            for chunk in inner.chunks_by_id.values() {
                if excluded_chunks.contains(&chunk.id) {
                    continue;
                }
                let Some(document) = inner.documents.get(&chunk.document_id) else {
                    continue;
                };
                let Some(source) = inner.sources.get(&chunk.source_id) else {
                    continue;
                };
                let Some(priority) =
                    evidence_chunk_match_priority(chunk, document, source, &request.query)
                else {
                    continue;
                };
                let source_order = match &request.query {
                    EvalLabEvidenceSearchQuery::Browse => String::new(),
                    _ => source.name.to_lowercase(),
                };
                chunk_winners.push(
                    (
                        priority,
                        document.path.to_lowercase(),
                        source_order,
                        document.id.0,
                        chunk.ordinal,
                        chunk.id.0,
                    ),
                    chunk.id.0,
                );
            }
        }

        let documents = document_winners
            .into_sorted_ids()
            .into_iter()
            .filter_map(|id| {
                inner
                    .documents
                    .get(&rag_debugger_core::DocumentId(id))
                    .and_then(|document| evidence_document(&inner, document))
            })
            .collect();
        let chunks = chunk_winners
            .into_sorted_ids()
            .into_iter()
            .filter_map(|id| {
                inner
                    .chunks_by_id
                    .get(&rag_debugger_core::ChunkId(id))
                    .and_then(|chunk| evidence_chunk(&inner, chunk))
            })
            .collect();

        Ok(EvalLabEvidenceSearchResult { documents, chunks })
    }
}

fn evidence_document(
    inner: &MemoryStoreInner,
    document: &Document,
) -> Option<EvalLabEvidenceDocument> {
    let source = inner.sources.get(&document.source_id)?;
    Some(EvalLabEvidenceDocument {
        id: document.id,
        source_id: source.id,
        source_name: source.name.clone(),
        path: document.path.clone(),
        profile: document.profile,
        extraction_quality: document.extraction_quality,
        warnings: document.warnings.clone(),
        chunk_count: inner
            .chunks
            .get(&document.id)
            .map_or(0, |chunks| chunks.len() as u32),
    })
}

fn evidence_chunk(inner: &MemoryStoreInner, chunk: &Chunk) -> Option<EvalLabEvidenceChunk> {
    let document = inner.documents.get(&chunk.document_id)?;
    let source = inner.sources.get(&chunk.source_id)?;
    let (text_preview, preview_truncated) = evidence_preview(&chunk.text);
    Some(EvalLabEvidenceChunk {
        id: chunk.id,
        document_id: document.id,
        source_id: source.id,
        source_name: source.name.clone(),
        document_path: document.path.clone(),
        ordinal: chunk.ordinal,
        text_preview,
        preview_truncated,
        token_count: chunk.token_count,
        checksum: chunk.checksum.clone(),
        section_title: chunk.section_title.clone(),
        quality_flags: chunk.quality_flags.clone(),
        is_duplicate: chunk.is_duplicate,
        text_density: chunk.text_density,
        evidence_score_hint: chunk.evidence_score_hint,
    })
}

fn evidence_preview(text: &str) -> (String, bool) {
    let mut characters = text.chars();
    let preview = characters
        .by_ref()
        .take(EVAL_LAB_EVIDENCE_PREVIEW_CHAR_LIMIT)
        .collect();
    (preview, characters.next().is_some())
}

fn evidence_document_match_priority(
    document: &Document,
    source: &Source,
    query: &EvalLabEvidenceSearchQuery,
) -> Option<u8> {
    match query {
        EvalLabEvidenceSearchQuery::Browse => Some(0),
        EvalLabEvidenceSearchQuery::ExactId(query_id) => (document.id.0 == *query_id).then_some(0),
        EvalLabEvidenceSearchQuery::Text(query) => {
            if document.path.to_lowercase().contains(query) {
                Some(1)
            } else if source.name.to_lowercase().contains(query) {
                Some(2)
            } else {
                None
            }
        }
    }
}

fn evidence_chunk_match_priority(
    chunk: &Chunk,
    document: &Document,
    source: &Source,
    query: &EvalLabEvidenceSearchQuery,
) -> Option<u8> {
    match query {
        EvalLabEvidenceSearchQuery::Browse => Some(0),
        EvalLabEvidenceSearchQuery::ExactId(query_id) => {
            if chunk.id.0 == *query_id {
                Some(0)
            } else if document.id.0 == *query_id {
                Some(1)
            } else {
                None
            }
        }
        EvalLabEvidenceSearchQuery::Text(query) => {
            if document.path.to_lowercase().contains(query) {
                Some(2)
            } else if chunk
                .section_title
                .as_ref()
                .is_some_and(|section| section.to_lowercase().contains(query))
            {
                Some(3)
            } else if source.name.to_lowercase().contains(query) {
                Some(4)
            } else if chunk.text.to_lowercase().contains(query) {
                Some(5)
            } else {
                None
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct RankedId<K> {
    key: K,
    id: Uuid,
}

impl<K: Ord> Ord for RankedId<K> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key
            .cmp(&other.key)
            .then_with(|| self.id.cmp(&other.id))
    }
}

impl<K: Ord> PartialOrd for RankedId<K> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct BoundedTopK<K> {
    limit: usize,
    heap: BinaryHeap<RankedId<K>>,
    #[cfg(test)]
    peak_len: usize,
}

impl<K: Ord> BoundedTopK<K> {
    fn new(limit: usize) -> Self {
        Self {
            limit,
            heap: BinaryHeap::with_capacity(limit),
            #[cfg(test)]
            peak_len: 0,
        }
    }

    fn push(&mut self, key: K, id: Uuid) {
        if self.limit == 0 {
            return;
        }
        let candidate = RankedId { key, id };
        if self.heap.len() < self.limit {
            self.heap.push(candidate);
        } else if self
            .heap
            .peek()
            .is_some_and(|worst| candidate.cmp(worst).is_lt())
        {
            let _ = self.heap.pop();
            self.heap.push(candidate);
        }
        #[cfg(test)]
        {
            self.peak_len = self.peak_len.max(self.heap.len());
        }
    }

    fn into_sorted_ids(self) -> Vec<Uuid> {
        self.heap
            .into_sorted_vec()
            .into_iter()
            .map(|ranked| ranked.id)
            .collect()
    }
}

fn deduplicated<T>(ids: &[T]) -> impl Iterator<Item = &T>
where
    T: Eq + std::hash::Hash,
{
    let mut seen = HashSet::new();
    ids.iter().filter(move |id| seen.insert(*id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_top_k_retains_only_the_best_of_ten_thousand_candidates() {
        let mut winners = BoundedTopK::new(1);
        let expected = Uuid::from_u128(1);

        for value in (1_u128..=10_000).rev() {
            winners.push(value, Uuid::from_u128(value));
        }

        assert_eq!(winners.peak_len, 1);
        assert_eq!(winners.into_sorted_ids(), vec![expected]);
    }

    #[test]
    fn zero_capacity_heap_never_retains_candidates() {
        let mut winners = BoundedTopK::new(0);
        for value in 1_u128..=10_000 {
            winners.push(value, Uuid::from_u128(value));
        }

        assert_eq!(winners.peak_len, 0);
        assert!(winners.into_sorted_ids().is_empty());
    }
}
