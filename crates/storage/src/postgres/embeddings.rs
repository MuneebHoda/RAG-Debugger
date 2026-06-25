use rag_debugger_core::*;
use sqlx::Row;

use super::{codec::*, PostgresStore};
use crate::StorageError;

impl PostgresStore {
    pub(super) async fn embedding_status(
        &self,
        request: &EmbeddingIndexRequest,
        model: &EmbeddingModelInfo,
    ) -> Result<EmbeddingStatus, StorageError> {
        let source_ids = source_filter_ids(&request.source_ids);
        let document_ids = document_filter_ids(&request.document_ids);
        let row = sqlx::query(
            "SELECT
                COUNT(*)::INT AS total_chunks,
                (COUNT(*) FILTER (
                    WHERE e.chunk_id IS NOT NULL
                      AND e.model_provider = $5
                      AND e.model_name = $6
                      AND e.dimension = $7
                      AND e.chunk_checksum = c.checksum
                ))::INT AS indexed_chunks,
                (COUNT(*) FILTER (WHERE e.chunk_id IS NULL))::INT AS missing_chunks,
                (COUNT(*) FILTER (
                    WHERE e.chunk_id IS NOT NULL
                      AND (
                        e.model_provider <> $5
                        OR e.model_name <> $6
                        OR e.dimension <> $7
                        OR e.chunk_checksum <> c.checksum
                      )
                ))::INT AS stale_chunks,
                MAX(e.indexed_at) FILTER (
                    WHERE e.chunk_id IS NOT NULL
                      AND e.model_provider = $5
                      AND e.model_name = $6
                      AND e.dimension = $7
                      AND e.chunk_checksum = c.checksum
                ) AS last_indexed_at
             FROM chunks c
             LEFT JOIN chunk_embeddings e ON e.chunk_id = c.id
             WHERE ($1 OR c.source_id = ANY($2))
               AND ($3 OR c.document_id = ANY($4))",
        )
        .bind(source_ids.is_empty())
        .bind(source_ids)
        .bind(document_ids.is_empty())
        .bind(document_ids)
        .bind(&model.provider)
        .bind(&model.model_name)
        .bind(model.dimension as i32)
        .fetch_one(&self.pool)
        .await?;

        Ok(EmbeddingStatus {
            model: model.clone(),
            total_chunks: as_u32(row.try_get("total_chunks")?, "total_chunks")?,
            indexed_chunks: as_u32(row.try_get("indexed_chunks")?, "indexed_chunks")?,
            missing_chunks: as_u32(row.try_get("missing_chunks")?, "missing_chunks")?,
            stale_chunks: as_u32(row.try_get("stale_chunks")?, "stale_chunks")?,
            last_indexed_at: row.try_get("last_indexed_at")?,
        })
    }

    pub(super) async fn list_embedding_candidates(
        &self,
        request: &EmbeddingIndexRequest,
    ) -> Result<Vec<EmbeddingIndexCandidate>, StorageError> {
        let source_ids = source_filter_ids(&request.source_ids);
        let document_ids = document_filter_ids(&request.document_ids);
        let rows = sqlx::query(
            "SELECT c.id, c.source_id, c.document_id, c.text, c.checksum, c.strategy
             FROM chunks c
             WHERE ($1 OR c.source_id = ANY($2))
               AND ($3 OR c.document_id = ANY($4))
             ORDER BY c.document_id ASC, c.ordinal ASC",
        )
        .bind(source_ids.is_empty())
        .bind(source_ids)
        .bind(document_ids.is_empty())
        .bind(document_ids)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|row| {
                Ok(EmbeddingIndexCandidate {
                    chunk_id: ChunkId(row.try_get("id")?),
                    source_id: SourceId(row.try_get("source_id")?),
                    document_id: DocumentId(row.try_get("document_id")?),
                    text: row.try_get("text")?,
                    checksum: row.try_get("checksum")?,
                    chunking_strategy: chunking_strategy_from_str(
                        row.try_get::<String, _>("strategy")?.as_str(),
                    )?,
                })
            })
            .collect()
    }

    pub(super) async fn upsert_chunk_embeddings(
        &self,
        embeddings: Vec<ChunkEmbedding>,
    ) -> Result<(), StorageError> {
        let mut transaction = self.pool.begin().await?;

        for embedding in embeddings {
            sqlx::query(
                "INSERT INTO chunk_embeddings (
                    chunk_id, model_provider, model_name, dimension, vector, chunk_checksum, indexed_at
                 )
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (chunk_id) DO UPDATE SET
                    model_provider = EXCLUDED.model_provider,
                    model_name = EXCLUDED.model_name,
                    dimension = EXCLUDED.dimension,
                    vector = EXCLUDED.vector,
                    chunk_checksum = EXCLUDED.chunk_checksum,
                    indexed_at = EXCLUDED.indexed_at",
            )
            .bind(embedding.chunk_id.0)
            .bind(&embedding.model.provider)
            .bind(&embedding.model.model_name)
            .bind(embedding.model.dimension as i32)
            .bind(&embedding.vector)
            .bind(&embedding.chunk_checksum)
            .bind(embedding.indexed_at)
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;
        Ok(())
    }
}
