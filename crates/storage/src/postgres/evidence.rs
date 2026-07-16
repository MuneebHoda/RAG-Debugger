use rag_debugger_core::{
    EvalLabEvidenceChunk, EvalLabEvidenceDocument, EvalLabEvidenceSearchRequest,
    EvalLabEvidenceSearchResult, EVAL_LAB_EVIDENCE_PREVIEW_CHAR_LIMIT,
};
use sqlx::Row;
use uuid::Uuid;

use super::{codec::*, PostgresStore};
use crate::StorageError;

impl PostgresStore {
    pub(super) async fn resolve_evidence_documents(
        &self,
        document_ids: &[rag_debugger_core::DocumentId],
    ) -> Result<Vec<EvalLabEvidenceDocument>, StorageError> {
        if document_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ids = document_ids.iter().map(|id| id.0).collect::<Vec<_>>();
        let rows = sqlx::query(
            "WITH requested AS (
                SELECT id, MIN(position) AS position
                FROM unnest($1::uuid[]) WITH ORDINALITY AS input(id, position)
                GROUP BY id
             )
             SELECT d.id, d.source_id, s.name AS source_name, d.path,
                    d.document_profile, d.extraction_quality, d.warnings,
                    (SELECT COUNT(*) FROM chunks c WHERE c.document_id = d.id) AS chunk_count
             FROM requested r
             INNER JOIN documents d ON d.id = r.id
             INNER JOIN sources s ON s.id = d.source_id
             ORDER BY r.position",
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(evidence_document_from_row).collect()
    }

    pub(super) async fn resolve_evidence_chunks(
        &self,
        chunk_ids: &[rag_debugger_core::ChunkId],
    ) -> Result<Vec<EvalLabEvidenceChunk>, StorageError> {
        if chunk_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ids = chunk_ids.iter().map(|id| id.0).collect::<Vec<_>>();
        let rows = sqlx::query(
            "WITH requested AS (
                SELECT id, MIN(position) AS position
                FROM unnest($1::uuid[]) WITH ORDINALITY AS input(id, position)
                GROUP BY id
             )
             SELECT c.id, c.document_id, c.source_id, s.name AS source_name,
                    d.path AS document_path, c.ordinal,
                    LEFT(c.text, $2) AS text_preview,
                    char_length(c.text) > $2 AS preview_truncated,
                    c.token_count, c.checksum, c.section_title, c.quality_flags,
                    c.is_duplicate, c.text_density, c.evidence_score_hint
             FROM requested r
             INNER JOIN chunks c ON c.id = r.id
             INNER JOIN documents d ON d.id = c.document_id
             INNER JOIN sources s ON s.id = c.source_id
             ORDER BY r.position",
        )
        .bind(ids)
        .bind(EVAL_LAB_EVIDENCE_PREVIEW_CHAR_LIMIT as i32)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(evidence_chunk_from_row).collect()
    }

    pub(super) async fn search_evidence(
        &self,
        request: &EvalLabEvidenceSearchRequest,
    ) -> Result<EvalLabEvidenceSearchResult, StorageError> {
        let needle = request
            .query
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .to_lowercase();
        let excluded_document_ids = request
            .excluded_document_ids
            .iter()
            .map(|id| id.0)
            .collect::<Vec<_>>();
        let excluded_chunk_ids = request
            .excluded_chunk_ids
            .iter()
            .map(|id| id.0)
            .collect::<Vec<_>>();
        let query_id = Uuid::parse_str(&needle).ok();

        let documents = if request.document_limit == 0 {
            Vec::new()
        } else {
            let rows = sqlx::query(
                "SELECT d.id, d.source_id, s.name AS source_name, d.path,
                        d.document_profile, d.extraction_quality, d.warnings,
                        (SELECT COUNT(*) FROM chunks c WHERE c.document_id = d.id) AS chunk_count
                 FROM documents d
                 INNER JOIN sources s ON s.id = d.source_id
                 WHERE NOT (d.id = ANY($2::uuid[]))
                   AND ($1 = ''
                        OR ($4::uuid IS NOT NULL AND d.id = $4)
                        OR lower(d.id::text) LIKE '%' || $1 || '%'
                        OR lower(d.path) LIKE '%' || $1 || '%'
                        OR lower(s.name) LIKE '%' || $1 || '%')
                 ORDER BY
                    CASE
                      WHEN $4::uuid IS NOT NULL AND d.id = $4 THEN 0
                      WHEN $1 <> '' AND lower(d.path) LIKE '%' || $1 || '%' THEN 1
                      WHEN $1 <> '' AND lower(s.name) LIKE '%' || $1 || '%' THEN 2
                      WHEN $1 <> '' AND lower(d.id::text) LIKE '%' || $1 || '%' THEN 3
                      ELSE 4
                    END,
                    lower(d.path), lower(s.name), d.id
                 LIMIT $3",
            )
            .bind(&needle)
            .bind(excluded_document_ids)
            .bind(request.document_limit as i64)
            .bind(query_id)
            .fetch_all(&self.pool)
            .await?;
            rows.iter()
                .map(evidence_document_from_row)
                .collect::<Result<Vec<_>, _>>()?
        };

        let chunks = if request.chunk_limit == 0 {
            Vec::new()
        } else {
            let rows = sqlx::query(
                "SELECT c.id, c.document_id, c.source_id, s.name AS source_name,
                        d.path AS document_path, c.ordinal,
                        LEFT(c.text, $3) AS text_preview,
                        char_length(c.text) > $3 AS preview_truncated,
                        c.token_count, c.checksum, c.section_title, c.quality_flags,
                        c.is_duplicate, c.text_density, c.evidence_score_hint
                 FROM chunks c
                 INNER JOIN documents d ON d.id = c.document_id
                 INNER JOIN sources s ON s.id = c.source_id
                 WHERE NOT (c.id = ANY($2::uuid[]))
                   AND ($1 = ''
                        OR ($5::uuid IS NOT NULL AND (c.id = $5 OR d.id = $5))
                        OR lower(c.id::text) LIKE '%' || $1 || '%'
                        OR lower(d.id::text) LIKE '%' || $1 || '%'
                        OR lower(d.path) LIKE '%' || $1 || '%'
                        OR lower(COALESCE(c.section_title, '')) LIKE '%' || $1 || '%'
                        OR lower(s.name) LIKE '%' || $1 || '%'
                        OR lower(c.text) LIKE '%' || $1 || '%')
                 ORDER BY
                    CASE
                      WHEN $5::uuid IS NOT NULL AND c.id = $5 THEN 0
                      WHEN $5::uuid IS NOT NULL AND d.id = $5 THEN 1
                      WHEN $1 <> '' AND lower(d.path) LIKE '%' || $1 || '%' THEN 2
                      WHEN $1 <> '' AND lower(COALESCE(c.section_title, '')) LIKE '%' || $1 || '%' THEN 3
                      WHEN $1 <> '' AND lower(s.name) LIKE '%' || $1 || '%' THEN 4
                      WHEN $1 <> '' AND lower(c.text) LIKE '%' || $1 || '%' THEN 5
                      WHEN $1 <> '' AND (lower(c.id::text) LIKE '%' || $1 || '%'
                                           OR lower(d.id::text) LIKE '%' || $1 || '%') THEN 6
                      ELSE 7
                    END,
                    lower(d.path), c.ordinal, c.id
                 LIMIT $4",
            )
            .bind(&needle)
            .bind(excluded_chunk_ids)
            .bind(EVAL_LAB_EVIDENCE_PREVIEW_CHAR_LIMIT as i32)
            .bind(request.chunk_limit as i64)
            .bind(query_id)
            .fetch_all(&self.pool)
            .await?;
            rows.iter()
                .map(evidence_chunk_from_row)
                .collect::<Result<Vec<_>, _>>()?
        };

        Ok(EvalLabEvidenceSearchResult { documents, chunks })
    }
}

fn evidence_document_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<EvalLabEvidenceDocument, StorageError> {
    let chunk_count = row.try_get::<i64, _>("chunk_count")?;
    Ok(EvalLabEvidenceDocument {
        id: rag_debugger_core::DocumentId(row.try_get("id")?),
        source_id: rag_debugger_core::SourceId(row.try_get("source_id")?),
        source_name: row.try_get("source_name")?,
        path: row.try_get("path")?,
        profile: document_profile_from_str(row.try_get::<String, _>("document_profile")?.as_str())?,
        extraction_quality: extraction_quality_from_str(
            row.try_get::<String, _>("extraction_quality")?.as_str(),
        )?,
        warnings: document_warnings_from_text(
            row.try_get::<Vec<String>, _>("warnings")
                .unwrap_or_default(),
        ),
        chunk_count: u32::try_from(chunk_count)
            .map_err(|_| StorageError::InvalidData("chunk_count is out of range".to_owned()))?,
    })
}

fn evidence_chunk_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<EvalLabEvidenceChunk, StorageError> {
    Ok(EvalLabEvidenceChunk {
        id: rag_debugger_core::ChunkId(row.try_get("id")?),
        document_id: rag_debugger_core::DocumentId(row.try_get("document_id")?),
        source_id: rag_debugger_core::SourceId(row.try_get("source_id")?),
        source_name: row.try_get("source_name")?,
        document_path: row.try_get("document_path")?,
        ordinal: as_u32(row.try_get("ordinal")?, "ordinal")?,
        text_preview: row.try_get("text_preview")?,
        preview_truncated: row.try_get("preview_truncated")?,
        token_count: as_u32(row.try_get("token_count")?, "token_count")?,
        checksum: row.try_get("checksum")?,
        section_title: row.try_get("section_title")?,
        quality_flags: chunk_quality_flags_from_text(
            row.try_get::<Vec<String>, _>("quality_flags")
                .unwrap_or_default(),
        )?,
        is_duplicate: row.try_get("is_duplicate")?,
        text_density: row.try_get("text_density")?,
        evidence_score_hint: row.try_get("evidence_score_hint")?,
    })
}
