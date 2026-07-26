use std::collections::{BTreeSet, HashSet};

use rag_debugger_core::{ChunkId, DocumentId};
use uuid::Uuid;

type DocumentBrowseKey = (String, Uuid);
type ChunkBrowseKey = (String, Uuid, u32, Uuid);

#[derive(Debug, Default)]
pub(super) struct EvidenceBrowseIndexes {
    documents: BTreeSet<DocumentBrowseKey>,
    chunks: BTreeSet<ChunkBrowseKey>,
}

impl EvidenceBrowseIndexes {
    pub(super) fn insert_document(&mut self, path: &str, document_id: DocumentId) {
        self.documents
            .insert((normalized_path(path), document_id.0));
    }

    pub(super) fn remove_document(&mut self, path: &str, document_id: DocumentId) {
        self.documents
            .remove(&(normalized_path(path), document_id.0));
    }

    pub(super) fn insert_chunk(
        &mut self,
        document_path: &str,
        document_id: DocumentId,
        ordinal: u32,
        chunk_id: ChunkId,
    ) {
        self.chunks.insert((
            normalized_path(document_path),
            document_id.0,
            ordinal,
            chunk_id.0,
        ));
    }

    pub(super) fn remove_chunk(
        &mut self,
        document_path: &str,
        document_id: DocumentId,
        ordinal: u32,
        chunk_id: ChunkId,
    ) {
        self.chunks.remove(&(
            normalized_path(document_path),
            document_id.0,
            ordinal,
            chunk_id.0,
        ));
    }

    pub(super) fn browse_documents(
        &self,
        excluded: &HashSet<DocumentId>,
        limit: usize,
        mut is_eligible: impl FnMut(DocumentId) -> bool,
    ) -> BrowseSelection<DocumentId> {
        collect_eligible(
            self.documents.iter().map(|(_, id)| DocumentId(*id)),
            limit,
            &mut |id| !excluded.contains(&id) && is_eligible(id),
        )
    }

    pub(super) fn browse_chunks(
        &self,
        excluded: &HashSet<ChunkId>,
        limit: usize,
        mut is_eligible: impl FnMut(ChunkId) -> bool,
    ) -> BrowseSelection<ChunkId> {
        collect_eligible(
            self.chunks.iter().map(|(_, _, _, id)| ChunkId(*id)),
            limit,
            &mut |id| !excluded.contains(&id) && is_eligible(id),
        )
    }

    #[cfg(test)]
    pub(super) fn document_len(&self) -> usize {
        self.documents.len()
    }

    #[cfg(test)]
    pub(super) fn chunk_len(&self) -> usize {
        self.chunks.len()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct BrowseSelection<T> {
    pub(super) ids: Vec<T>,
    pub(super) examined: usize,
}

impl<T> Default for BrowseSelection<T> {
    fn default() -> Self {
        Self {
            ids: Vec::new(),
            examined: 0,
        }
    }
}

fn collect_eligible<T>(
    candidates: impl Iterator<Item = T>,
    limit: usize,
    is_eligible: &mut impl FnMut(T) -> bool,
) -> BrowseSelection<T>
where
    T: Copy,
{
    if limit == 0 {
        return BrowseSelection::default();
    }

    let mut selection = BrowseSelection {
        ids: Vec::with_capacity(limit),
        examined: 0,
    };
    for candidate in candidates {
        selection.examined += 1;
        if is_eligible(candidate) {
            selection.ids.push(candidate);
            if selection.ids.len() == limit {
                break;
            }
        }
    }
    selection
}

fn normalized_path(path: &str) -> String {
    path.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browse_stops_after_collecting_the_requested_eligible_entries() {
        let mut indexes = EvidenceBrowseIndexes::default();
        for value in 1_u128..=10_000 {
            indexes.insert_document(
                &format!("documents/{value:05}.md"),
                DocumentId(Uuid::from_u128(value)),
            );
        }

        let selection = indexes.browse_documents(&HashSet::new(), 1, |_| true);

        assert_eq!(selection.examined, 1);
        assert_eq!(selection.ids, vec![DocumentId(Uuid::from_u128(1))]);
    }

    #[test]
    fn browse_continues_only_until_an_eligible_non_excluded_entry_is_found() {
        let mut indexes = EvidenceBrowseIndexes::default();
        for value in 1_u128..=10 {
            indexes.insert_document(
                &format!("documents/{value:05}.md"),
                DocumentId(Uuid::from_u128(value)),
            );
        }
        let excluded = [DocumentId(Uuid::from_u128(1))].into_iter().collect();

        let selection =
            indexes.browse_documents(&excluded, 1, |id| id != DocumentId(Uuid::from_u128(2)));

        assert_eq!(selection.examined, 3);
        assert_eq!(selection.ids, vec![DocumentId(Uuid::from_u128(3))]);
    }
}
