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

        let inner = self.lock()?;
        Ok(search_evidence_snapshot(&inner, request).0)
    }
}

fn search_evidence_snapshot(
    inner: &MemoryStoreInner,
    request: &EvalLabEvidenceSearchRequest,
) -> (EvalLabEvidenceSearchResult, EvidenceSearchStats) {
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

    match &request.query {
        EvalLabEvidenceSearchQuery::Browse => {
            browse_evidence(inner, request, &excluded_documents, &excluded_chunks)
        }
        EvalLabEvidenceSearchQuery::ExactId(id) => {
            exact_evidence(inner, *id, request, &excluded_documents, &excluded_chunks)
        }
        EvalLabEvidenceSearchQuery::Text(query) => {
            text_evidence(inner, query, request, &excluded_documents, &excluded_chunks)
        }
    }
}

fn browse_evidence(
    inner: &MemoryStoreInner,
    request: &EvalLabEvidenceSearchRequest,
    excluded_documents: &HashSet<DocumentId>,
    excluded_chunks: &HashSet<ChunkId>,
) -> (EvalLabEvidenceSearchResult, EvidenceSearchStats) {
    let selected_documents = inner.evidence_browse_indexes.browse_documents(
        excluded_documents,
        request.document_limit as usize,
        |id| {
            inner
                .documents
                .get(&id)
                .is_some_and(|document| inner.sources.contains_key(&document.source_id))
        },
    );
    let selected_chunks = inner.evidence_browse_indexes.browse_chunks(
        excluded_chunks,
        request.chunk_limit as usize,
        |id| {
            inner.chunks_by_id.get(&id).is_some_and(|chunk| {
                inner.documents.contains_key(&chunk.document_id)
                    && inner.sources.contains_key(&chunk.source_id)
            })
        },
    );
    let stats = EvidenceSearchStats {
        browse_document_entries_examined: selected_documents.examined,
        browse_chunk_entries_examined: selected_chunks.examined,
        ..EvidenceSearchStats::default()
    };
    let documents = selected_documents
        .ids
        .into_iter()
        .filter_map(|id| {
            inner
                .documents
                .get(&id)
                .and_then(|document| evidence_document(inner, document))
        })
        .collect();
    let chunks = selected_chunks
        .ids
        .into_iter()
        .filter_map(|id| {
            inner
                .chunks_by_id
                .get(&id)
                .and_then(|chunk| evidence_chunk(inner, chunk))
        })
        .collect();

    (EvalLabEvidenceSearchResult { documents, chunks }, stats)
}

fn exact_evidence(
    inner: &MemoryStoreInner,
    id: Uuid,
    request: &EvalLabEvidenceSearchRequest,
    excluded_documents: &HashSet<DocumentId>,
    excluded_chunks: &HashSet<ChunkId>,
) -> (EvalLabEvidenceSearchResult, EvidenceSearchStats) {
    let document_id = DocumentId(id);
    let mut documents = Vec::with_capacity(usize::from(request.document_limit > 0));
    if request.document_limit > 0 && !excluded_documents.contains(&document_id) {
        if let Some(document) = inner.documents.get(&document_id) {
            if let Some(evidence) = evidence_document(inner, document) {
                documents.push(evidence);
            }
        }
    }

    let mut chunks = Vec::with_capacity(request.chunk_limit as usize);
    let mut seen_chunks = HashSet::new();
    if request.chunk_limit > 0 {
        let chunk_id = ChunkId(id);
        if !excluded_chunks.contains(&chunk_id) {
            if let Some(chunk) = inner.chunks_by_id.get(&chunk_id) {
                if let Some(evidence) = evidence_chunk(inner, chunk) {
                    seen_chunks.insert(chunk_id);
                    chunks.push(evidence);
                }
            }
        }

        if chunks.len() < request.chunk_limit as usize {
            if let Some(document_chunks) = inner.chunks.get(&document_id) {
                for chunk in document_chunks {
                    if chunks.len() == request.chunk_limit as usize {
                        break;
                    }
                    if excluded_chunks.contains(&chunk.id) || !seen_chunks.insert(chunk.id) {
                        continue;
                    }
                    if let Some(evidence) = evidence_chunk(inner, chunk) {
                        chunks.push(evidence);
                    }
                }
            }
        }
    }

    (
        EvalLabEvidenceSearchResult { documents, chunks },
        EvidenceSearchStats::default(),
    )
}

fn text_evidence(
    inner: &MemoryStoreInner,
    query: &str,
    request: &EvalLabEvidenceSearchRequest,
    excluded_documents: &HashSet<DocumentId>,
    excluded_chunks: &HashSet<ChunkId>,
) -> (EvalLabEvidenceSearchResult, EvidenceSearchStats) {
    // Text search intentionally scans one consistent in-memory snapshot. The bounded
    // heaps cap temporary candidate retention, while Postgres remains the indexed
    // scalable path for production-sized corpora.
    let query = EvalLabEvidenceSearchQuery::Text(query.to_owned());
    let mut stats = EvidenceSearchStats::default();
    let mut document_winners = BoundedTopK::new(request.document_limit as usize);
    if request.document_limit > 0 {
        for document in inner.documents.values() {
            stats.text_document_records_examined += 1;
            if excluded_documents.contains(&document.id) {
                continue;
            }
            let Some(source) = inner.sources.get(&document.source_id) else {
                continue;
            };
            let Some(priority) = evidence_document_match_priority(document, source, &query) else {
                continue;
            };
            document_winners.push(
                (
                    priority,
                    document.path.to_lowercase(),
                    source.name.to_lowercase(),
                    document.id.0,
                ),
                document.id.0,
            );
        }
    }

    let mut chunk_winners = BoundedTopK::new(request.chunk_limit as usize);
    if request.chunk_limit > 0 {
        for chunk in inner.chunks_by_id.values() {
            stats.text_chunk_records_examined += 1;
            if excluded_chunks.contains(&chunk.id) {
                continue;
            }
            let Some(document) = inner.documents.get(&chunk.document_id) else {
                continue;
            };
            let Some(source) = inner.sources.get(&chunk.source_id) else {
                continue;
            };
            let Some(priority) = evidence_chunk_match_priority(chunk, document, source, &query)
            else {
                continue;
            };
            chunk_winners.push(
                (
                    priority,
                    document.path.to_lowercase(),
                    source.name.to_lowercase(),
                    document.id.0,
                    chunk.ordinal,
                    chunk.id.0,
                ),
                chunk.id.0,
            );
        }
    }
    stats.peak_document_candidates = document_winners.peak_len();
    stats.peak_chunk_candidates = chunk_winners.peak_len();

    let documents = document_winners
        .into_sorted_ids()
        .into_iter()
        .filter_map(|id| {
            inner
                .documents
                .get(&DocumentId(id))
                .and_then(|document| evidence_document(inner, document))
        })
        .collect();
    let chunks = chunk_winners
        .into_sorted_ids()
        .into_iter()
        .filter_map(|id| {
            inner
                .chunks_by_id
                .get(&ChunkId(id))
                .and_then(|chunk| evidence_chunk(inner, chunk))
        })
        .collect();

    (EvalLabEvidenceSearchResult { documents, chunks }, stats)
}

#[derive(Debug, Default, Eq, PartialEq)]
struct EvidenceSearchStats {
    browse_document_entries_examined: usize,
    browse_chunk_entries_examined: usize,
    text_document_records_examined: usize,
    text_chunk_records_examined: usize,
    peak_document_candidates: usize,
    peak_chunk_candidates: usize,
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
    peak_len: usize,
}

impl<K: Ord> BoundedTopK<K> {
    fn new(limit: usize) -> Self {
        Self {
            limit,
            heap: BinaryHeap::with_capacity(limit),
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
        self.peak_len = self.peak_len.max(self.heap.len());
    }

    fn peak_len(&self) -> usize {
        self.peak_len
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
    use rag_debugger_core::{
        ByteRange, ChunkQualityFlag, ChunkSplitReason, ChunkingConfig, ChunkingStrategy,
        DocumentProfile, ExtractionQuality, ProjectId, SourceId, SourceKind, SourceSyncPolicy,
    };

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

    #[test]
    fn large_snapshot_proves_bounded_browse_direct_exact_lookup_and_bounded_text_retention() {
        let inner = large_evidence_snapshot();
        assert_eq!(inner.evidence_browse_indexes.document_len(), 10_000);
        assert_eq!(inner.evidence_browse_indexes.chunk_len(), 60_000);

        let (first, first_stats) = search_evidence_snapshot(&inner, &browse_request(1, 1));
        let (repeated, repeated_stats) = search_evidence_snapshot(&inner, &browse_request(1, 1));

        assert_eq!(first, repeated);
        assert_eq!(first.documents[0].path, "documents/00000.md");
        assert_eq!(first.chunks[0].document_path, "documents/00000.md");
        assert_eq!(first.chunks[0].ordinal, 0);
        assert_eq!(first_stats.browse_document_entries_examined, 1);
        assert_eq!(first_stats.browse_chunk_entries_examined, 1);
        assert_eq!(repeated_stats, first_stats);

        let mut excluded_request = browse_request(1, 1);
        excluded_request.excluded_document_ids = (0_u128..5)
            .map(|index| fixture_document_id(index as usize))
            .collect();
        excluded_request.excluded_chunk_ids = (0_u128..5)
            .map(|ordinal| fixture_chunk_id(0, ordinal as u32))
            .collect();
        let (excluded_result, excluded_stats) = search_evidence_snapshot(&inner, &excluded_request);
        assert_eq!(excluded_result.documents[0].path, "documents/00005.md");
        assert_eq!(excluded_result.chunks[0].ordinal, 5);
        assert_eq!(excluded_stats.browse_document_entries_examined, 6);
        assert_eq!(excluded_stats.browse_chunk_entries_examined, 6);

        let document_id = fixture_document_id(9_999);
        let chunk_id = fixture_chunk_id(9_999, 5);
        let (document_result, document_stats) = search_evidence_snapshot(
            &inner,
            &EvalLabEvidenceSearchRequest {
                query: EvalLabEvidenceSearchQuery::ExactId(document_id.0),
                excluded_document_ids: Vec::new(),
                excluded_chunk_ids: Vec::new(),
                document_limit: 1,
                chunk_limit: 1,
            },
        );
        let (chunk_result, chunk_stats) = search_evidence_snapshot(
            &inner,
            &EvalLabEvidenceSearchRequest {
                query: EvalLabEvidenceSearchQuery::ExactId(chunk_id.0),
                excluded_document_ids: Vec::new(),
                excluded_chunk_ids: Vec::new(),
                document_limit: 1,
                chunk_limit: 1,
            },
        );

        assert_eq!(document_result.documents[0].id, document_id);
        assert_eq!(document_result.chunks[0].document_id, document_id);
        assert_eq!(chunk_result.chunks[0].id, chunk_id);
        assert_eq!(document_stats, EvidenceSearchStats::default());
        assert_eq!(chunk_stats, EvidenceSearchStats::default());

        let text_request = EvalLabEvidenceSearchRequest {
            query: EvalLabEvidenceSearchQuery::Text("bounded evidence".to_owned()),
            excluded_document_ids: Vec::new(),
            excluded_chunk_ids: Vec::new(),
            document_limit: 1,
            chunk_limit: 1,
        };
        let (text_result, text_stats) = search_evidence_snapshot(&inner, &text_request);
        assert_eq!(text_result.chunks.len(), 1);
        assert_eq!(text_stats.text_document_records_examined, 10_000);
        assert_eq!(text_stats.text_chunk_records_examined, 60_000);
        assert_eq!(text_stats.peak_document_candidates, 0);
        assert_eq!(text_stats.peak_chunk_candidates, 1);
    }

    #[test]
    fn document_replacement_and_removal_keep_browse_indexes_synchronized() {
        let source = fixture_source();
        let mut inner = MemoryStoreInner::default();
        inner.sources.insert(source.id, source.clone());
        let zeta_document = fixture_document(source.id, 1, "zeta/guide.md");
        let beta_document = fixture_document(source.id, 2, "beta/guide.md");
        let old_chunk = fixture_chunk(source.id, zeta_document.id, 100, 0);
        let beta_chunk = fixture_chunk(source.id, beta_document.id, 200, 0);
        inner.replace_document_with_chunks(zeta_document.clone(), vec![old_chunk.clone()]);
        inner.replace_document_with_chunks(beta_document.clone(), vec![beta_chunk.clone()]);

        let (before, _) = search_evidence_snapshot(&inner, &browse_request(10, 10));
        assert_eq!(
            before
                .documents
                .iter()
                .map(|document| document.id)
                .collect::<Vec<_>>(),
            vec![beta_document.id, zeta_document.id]
        );

        let alpha_document = Document {
            path: "alpha/guide.md".to_owned(),
            ..zeta_document
        };
        let replacement_chunk = fixture_chunk(source.id, alpha_document.id, 300, 3);
        inner.replace_document_with_chunks(alpha_document.clone(), vec![replacement_chunk.clone()]);

        let (replaced, _) = search_evidence_snapshot(&inner, &browse_request(10, 10));
        assert_eq!(replaced.documents[0].id, alpha_document.id);
        assert_eq!(replaced.chunks[0].id, replacement_chunk.id);
        assert!(!inner.chunks_by_id.contains_key(&old_chunk.id));
        assert_eq!(inner.evidence_browse_indexes.document_len(), 2);
        assert_eq!(inner.evidence_browse_indexes.chunk_len(), 2);

        let removed = inner.remove_document_with_chunks(alpha_document.id);
        let (after_removal, _) = search_evidence_snapshot(&inner, &browse_request(10, 10));
        assert_eq!(removed.map(|document| document.id), Some(alpha_document.id));
        assert_eq!(
            after_removal
                .documents
                .iter()
                .map(|document| document.id)
                .collect::<Vec<_>>(),
            vec![beta_document.id]
        );
        assert_eq!(
            after_removal
                .chunks
                .iter()
                .map(|chunk| chunk.id)
                .collect::<Vec<_>>(),
            vec![beta_chunk.id]
        );
    }

    #[test]
    fn demo_merge_rebuilds_document_and_chunk_ordering() {
        let source = fixture_source();
        let mut inner = MemoryStoreInner::default();
        inner.sources.insert(source.id, source.clone());
        let initial_document = fixture_document(source.id, 1, "zeta/demo.md");
        let initial_chunk = fixture_chunk(source.id, initial_document.id, 100, 0);
        inner.replace_document_with_chunks(initial_document.clone(), vec![initial_chunk]);

        let updated_document = Document {
            path: "alpha/demo.md".to_owned(),
            ..initial_document
        };
        let updated_chunk = fixture_chunk(source.id, updated_document.id, 100, 2);
        let added_chunk = fixture_chunk(source.id, updated_document.id, 101, 1);
        inner.merge_document_with_chunks(
            updated_document.clone(),
            vec![updated_chunk.clone(), added_chunk.clone()],
        );

        let (result, _) = search_evidence_snapshot(&inner, &browse_request(10, 10));
        assert_eq!(result.documents[0].path, updated_document.path);
        assert_eq!(
            result
                .chunks
                .iter()
                .map(|chunk| chunk.id)
                .collect::<Vec<_>>(),
            vec![added_chunk.id, updated_chunk.id]
        );
        assert_eq!(inner.evidence_browse_indexes.document_len(), 1);
        assert_eq!(inner.evidence_browse_indexes.chunk_len(), 2);
    }

    fn large_evidence_snapshot() -> MemoryStoreInner {
        let source = fixture_source();
        let mut inner = MemoryStoreInner::default();
        inner.sources.insert(source.id, source.clone());

        for document_index in 0..10_000 {
            let document = fixture_document(
                source.id,
                document_index,
                &format!("documents/{document_index:05}.md"),
            );
            let chunks = (0..6)
                .map(|ordinal| {
                    fixture_chunk(
                        source.id,
                        document.id,
                        fixture_chunk_number(document_index, ordinal),
                        ordinal,
                    )
                })
                .collect();
            inner.replace_document_with_chunks(document, chunks);
        }
        inner
    }

    fn browse_request(document_limit: u32, chunk_limit: u32) -> EvalLabEvidenceSearchRequest {
        EvalLabEvidenceSearchRequest {
            query: EvalLabEvidenceSearchQuery::Browse,
            excluded_document_ids: Vec::new(),
            excluded_chunk_ids: Vec::new(),
            document_limit,
            chunk_limit,
        }
    }

    fn fixture_source() -> Source {
        Source {
            id: SourceId(Uuid::from_u128(1)),
            project_id: ProjectId(Uuid::from_u128(2)),
            name: "Memory evidence fixture".to_owned(),
            kind: SourceKind::FileSet {
                root_hint: "memory-evidence".to_owned(),
            },
            sync_policy: SourceSyncPolicy::Manual,
            chunking: ChunkingConfig::default(),
        }
    }

    fn fixture_document(source_id: SourceId, index: usize, path: &str) -> Document {
        Document {
            id: fixture_document_id(index),
            source_id,
            path: path.to_owned(),
            mime_type: Some("text/markdown".to_owned()),
            checksum: format!("document-{index}"),
            byte_size: 128,
            profile: DocumentProfile::TechnicalDocs,
            extraction_quality: ExtractionQuality::High,
            warnings: Vec::new(),
        }
    }

    fn fixture_chunk(
        source_id: SourceId,
        document_id: DocumentId,
        chunk_number: usize,
        ordinal: u32,
    ) -> Chunk {
        let text = format!("bounded evidence chunk {chunk_number}");
        Chunk {
            id: ChunkId(Uuid::from_u128(1_000_000 + chunk_number as u128)),
            source_id,
            document_id,
            ordinal,
            token_count: 4,
            byte_range: ByteRange {
                start: 0,
                end: text.len() as u64,
            },
            checksum: format!("chunk-{chunk_number}"),
            text,
            strategy: ChunkingStrategy::Structured,
            section_title: Some("Bounded evidence".to_owned()),
            split_reason: ChunkSplitReason::DocumentEnd,
            quality_flags: vec![ChunkQualityFlag::GoodEvidenceCandidate],
            is_duplicate: false,
            text_density: 1.0,
            evidence_score_hint: 0.9,
        }
    }

    fn fixture_document_id(index: usize) -> DocumentId {
        DocumentId(Uuid::from_u128(100_000 + index as u128))
    }

    fn fixture_chunk_id(document_index: usize, ordinal: u32) -> ChunkId {
        ChunkId(Uuid::from_u128(
            1_000_000 + fixture_chunk_number(document_index, ordinal) as u128,
        ))
    }

    fn fixture_chunk_number(document_index: usize, ordinal: u32) -> usize {
        document_index * 6 + ordinal as usize
    }
}
