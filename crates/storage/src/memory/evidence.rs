use std::collections::HashSet;

use async_trait::async_trait;
use rag_debugger_core::{
    Chunk, ChunkId, Document, DocumentId, EvalLabEvidenceChunk, EvalLabEvidenceDocument,
    EvalLabEvidenceSearchRequest, EvalLabEvidenceSearchResult, Source,
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
        let query = normalized_evidence_query(request.query.as_deref());

        let mut documents = inner
            .documents
            .values()
            .filter(|document| !excluded_documents.contains(&document.id))
            .filter_map(|document| {
                let source = inner.sources.get(&document.source_id)?;
                let priority =
                    evidence_document_match_priority(document, source, query.as_deref())?;
                let evidence = evidence_document(&inner, document)?;
                Some((priority, evidence))
            })
            .collect::<Vec<_>>();
        documents.sort_by(|(left_priority, left), (right_priority, right)| {
            left_priority
                .cmp(right_priority)
                .then_with(|| left.path.to_lowercase().cmp(&right.path.to_lowercase()))
                .then_with(|| {
                    left.source_name
                        .to_lowercase()
                        .cmp(&right.source_name.to_lowercase())
                })
                .then_with(|| left.id.0.cmp(&right.id.0))
        });

        let mut chunks = if request.chunk_limit == 0 {
            Vec::new()
        } else {
            inner
                .chunks_by_id
                .values()
                .filter(|chunk| !excluded_chunks.contains(&chunk.id))
                .filter_map(|chunk| {
                    let document = inner.documents.get(&chunk.document_id)?;
                    let source = inner.sources.get(&chunk.source_id)?;
                    let priority =
                        evidence_chunk_match_priority(chunk, document, source, query.as_deref())?;
                    let evidence = evidence_chunk(&inner, chunk)?;
                    Some((priority, evidence))
                })
                .collect::<Vec<_>>()
        };
        chunks.sort_by(|(left_priority, left), (right_priority, right)| {
            left_priority
                .cmp(right_priority)
                .then_with(|| {
                    left.document_path
                        .to_lowercase()
                        .cmp(&right.document_path.to_lowercase())
                })
                .then_with(|| left.ordinal.cmp(&right.ordinal))
                .then_with(|| left.id.0.cmp(&right.id.0))
        });

        Ok(EvalLabEvidenceSearchResult {
            documents: documents
                .into_iter()
                .map(|(_, document)| document)
                .take(request.document_limit as usize)
                .collect(),
            chunks: chunks
                .into_iter()
                .map(|(_, chunk)| chunk)
                .take(request.chunk_limit as usize)
                .collect(),
        })
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

fn normalized_evidence_query(query: Option<&str>) -> Option<String> {
    query
        .map(str::trim)
        .filter(|query| !query.is_empty())
        .map(str::to_lowercase)
}

fn evidence_document_match_priority(
    document: &Document,
    source: &Source,
    query: Option<&str>,
) -> Option<u8> {
    let Some(query) = query else {
        return Some(0);
    };
    let query_id = Uuid::parse_str(query).ok();
    if query_id.is_some_and(|query_id| document.id.0 == query_id) {
        Some(0)
    } else if document.path.to_lowercase().contains(query) {
        Some(1)
    } else if source.name.to_lowercase().contains(query) {
        Some(2)
    } else if document.id.0.to_string().contains(query) {
        Some(3)
    } else {
        None
    }
}

fn evidence_chunk_match_priority(
    chunk: &Chunk,
    document: &Document,
    source: &Source,
    query: Option<&str>,
) -> Option<u8> {
    let Some(query) = query else {
        return Some(0);
    };
    let query_id = Uuid::parse_str(query).ok();
    if query_id.is_some_and(|query_id| chunk.id.0 == query_id) {
        Some(0)
    } else if query_id.is_some_and(|query_id| document.id.0 == query_id) {
        Some(1)
    } else if document.path.to_lowercase().contains(query) {
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
    } else if chunk.id.0.to_string().contains(query) || document.id.0.to_string().contains(query) {
        Some(6)
    } else {
        None
    }
}

fn deduplicated<T>(ids: &[T]) -> impl Iterator<Item = &T>
where
    T: Eq + std::hash::Hash,
{
    let mut seen = HashSet::new();
    ids.iter().filter(move |id| seen.insert(*id))
}
