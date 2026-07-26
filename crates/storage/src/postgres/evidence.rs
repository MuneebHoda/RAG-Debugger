use rag_debugger_core::{
    EvalLabEvidenceChunk, EvalLabEvidenceDocument, EvalLabEvidenceSearchQuery,
    EvalLabEvidenceSearchRequest, EvalLabEvidenceSearchResult,
    EVAL_LAB_EVIDENCE_PREVIEW_CHAR_LIMIT,
};
use sqlx::Row;

use super::{codec::*, PostgresStore};
use crate::StorageError;

impl PostgresStore {
    pub(super) async fn resolve_evidence_documents(
        &self,
        workspace_id: rag_debugger_core::WorkspaceId,
        document_ids: &[rag_debugger_core::DocumentId],
    ) -> Result<Vec<EvalLabEvidenceDocument>, StorageError> {
        if document_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ids = document_ids.iter().map(|id| id.0).collect::<Vec<_>>();
        let rows = sqlx::query(
            "WITH requested AS (
                SELECT id, MIN(position) AS position
                FROM unnest($2::uuid[]) WITH ORDINALITY AS input(id, position)
                GROUP BY id
             )
             SELECT d.id, d.source_id, s.name AS source_name, d.path,
                    d.document_profile, d.extraction_quality, d.warnings,
                    (SELECT COUNT(*) FROM chunks c WHERE c.document_id = d.id) AS chunk_count
             FROM requested r
             INNER JOIN documents d ON d.id = r.id
             INNER JOIN sources s ON s.id = d.source_id
             INNER JOIN projects p ON p.id = s.project_id
             WHERE p.workspace_id = $1
             ORDER BY r.position",
        )
        .bind(workspace_id.0)
        .bind(ids)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(evidence_document_from_row).collect()
    }

    pub(super) async fn resolve_evidence_chunks(
        &self,
        workspace_id: rag_debugger_core::WorkspaceId,
        chunk_ids: &[rag_debugger_core::ChunkId],
    ) -> Result<Vec<EvalLabEvidenceChunk>, StorageError> {
        if chunk_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ids = chunk_ids.iter().map(|id| id.0).collect::<Vec<_>>();
        let rows = sqlx::query(
            "WITH requested AS (
                SELECT id, MIN(position) AS position
                FROM unnest($2::uuid[]) WITH ORDINALITY AS input(id, position)
                GROUP BY id
             )
             SELECT c.id, c.document_id, c.source_id, s.name AS source_name,
                    d.path AS document_path, c.ordinal,
                    LEFT(c.text, $3) AS text_preview,
                    char_length(c.text) > $3 AS preview_truncated,
                    c.token_count, c.checksum, c.section_title, c.quality_flags,
                    c.is_duplicate, c.text_density, c.evidence_score_hint
             FROM requested r
             INNER JOIN chunks c ON c.id = r.id
             INNER JOIN documents d ON d.id = c.document_id
             INNER JOIN sources s ON s.id = c.source_id
             INNER JOIN projects p ON p.id = s.project_id
             WHERE p.workspace_id = $1
             ORDER BY r.position",
        )
        .bind(workspace_id.0)
        .bind(ids)
        .bind(EVAL_LAB_EVIDENCE_PREVIEW_CHAR_LIMIT as i32)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(evidence_chunk_from_row).collect()
    }

    pub(super) async fn search_evidence(
        &self,
        workspace_id: rag_debugger_core::WorkspaceId,
        request: &EvalLabEvidenceSearchRequest,
    ) -> Result<EvalLabEvidenceSearchResult, StorageError> {
        if request.document_limit == 0 && request.chunk_limit == 0 {
            return Ok(EvalLabEvidenceSearchResult::default());
        }
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
        let documents = if request.document_limit == 0 {
            Vec::new()
        } else {
            let rows = match &request.query {
                EvalLabEvidenceSearchQuery::Browse => {
                    sqlx::query(BROWSE_DOCUMENTS_SQL)
                        .bind(workspace_id.0)
                        .bind(&excluded_document_ids)
                        .bind(request.document_limit as i64)
                        .fetch_all(&self.pool)
                        .await?
                }
                EvalLabEvidenceSearchQuery::ExactId(id) => {
                    sqlx::query(EXACT_DOCUMENT_SQL)
                        .bind(workspace_id.0)
                        .bind(id)
                        .bind(&excluded_document_ids)
                        .bind(request.document_limit as i64)
                        .fetch_all(&self.pool)
                        .await?
                }
                EvalLabEvidenceSearchQuery::Text(query) => {
                    sqlx::query(TEXT_DOCUMENTS_SQL)
                        .bind(workspace_id.0)
                        .bind(query)
                        .bind(&excluded_document_ids)
                        .bind(request.document_limit as i64)
                        .fetch_all(&self.pool)
                        .await?
                }
            };
            rows.iter()
                .map(evidence_document_from_row)
                .collect::<Result<Vec<_>, _>>()?
        };

        let chunks = if request.chunk_limit == 0 {
            Vec::new()
        } else {
            let rows = match &request.query {
                EvalLabEvidenceSearchQuery::Browse => {
                    sqlx::query(BROWSE_CHUNKS_SQL)
                        .bind(workspace_id.0)
                        .bind(&excluded_chunk_ids)
                        .bind(request.chunk_limit as i64)
                        .bind(EVAL_LAB_EVIDENCE_PREVIEW_CHAR_LIMIT as i32)
                        .fetch_all(&self.pool)
                        .await?
                }
                EvalLabEvidenceSearchQuery::ExactId(id) => {
                    sqlx::query(EXACT_CHUNKS_SQL)
                        .bind(workspace_id.0)
                        .bind(id)
                        .bind(&excluded_chunk_ids)
                        .bind(request.chunk_limit as i64)
                        .bind(EVAL_LAB_EVIDENCE_PREVIEW_CHAR_LIMIT as i32)
                        .fetch_all(&self.pool)
                        .await?
                }
                EvalLabEvidenceSearchQuery::Text(query) => {
                    sqlx::query(TEXT_CHUNKS_SQL)
                        .bind(workspace_id.0)
                        .bind(query)
                        .bind(&excluded_chunk_ids)
                        .bind(request.chunk_limit as i64)
                        .bind(EVAL_LAB_EVIDENCE_PREVIEW_CHAR_LIMIT as i32)
                        .fetch_all(&self.pool)
                        .await?
                }
            };
            rows.iter()
                .map(evidence_chunk_from_row)
                .collect::<Result<Vec<_>, _>>()?
        };

        Ok(EvalLabEvidenceSearchResult { documents, chunks })
    }
}

const BROWSE_DOCUMENTS_SQL: &str = "SELECT d.id, d.source_id, s.name AS source_name, d.path,
        d.document_profile, d.extraction_quality, d.warnings,
        (SELECT COUNT(*) FROM chunks count_chunks WHERE count_chunks.document_id = d.id) AS chunk_count
    FROM documents d
    INNER JOIN sources s ON s.id = d.source_id
    INNER JOIN projects p ON p.id = s.project_id
    WHERE p.workspace_id = $1
      AND NOT (d.id = ANY($2::uuid[]))
    ORDER BY lower(d.path) COLLATE \"C\", d.id
    LIMIT $3";

const EXACT_DOCUMENT_SQL: &str = "SELECT d.id, d.source_id, s.name AS source_name, d.path,
        d.document_profile, d.extraction_quality, d.warnings,
        (SELECT COUNT(*) FROM chunks count_chunks WHERE count_chunks.document_id = d.id) AS chunk_count
    FROM documents d
    INNER JOIN sources s ON s.id = d.source_id
    INNER JOIN projects p ON p.id = s.project_id
    WHERE p.workspace_id = $1
      AND d.id = $2
      AND NOT (d.id = ANY($3::uuid[]))
    LIMIT $4";

const TEXT_DOCUMENTS_SQL: &str = "WITH path_matches AS (
        SELECT d.id, 1::smallint AS priority
        FROM documents d
        INNER JOIN sources s ON s.id = d.source_id
        INNER JOIN projects p ON p.id = s.project_id
        WHERE p.workspace_id = $1
          AND NOT (d.id = ANY($3::uuid[]))
          AND lower(d.path) LIKE '%' || $2 || '%'
        ORDER BY lower(d.path) COLLATE \"C\", d.id
        LIMIT $4
    ), source_matches AS (
        SELECT d.id, 2::smallint AS priority
        FROM sources s
        INNER JOIN projects p ON p.id = s.project_id
        INNER JOIN documents d ON d.source_id = s.id
        WHERE p.workspace_id = $1
          AND NOT (d.id = ANY($3::uuid[]))
          AND lower(s.name) LIKE '%' || $2 || '%'
        ORDER BY lower(s.name) COLLATE \"C\", lower(d.path) COLLATE \"C\", d.id
        LIMIT $4
    ), ranked AS (
        SELECT id, MIN(priority) AS priority
        FROM (
            SELECT * FROM path_matches
            UNION ALL
            SELECT * FROM source_matches
        ) matches
        GROUP BY id
    )
    SELECT d.id, d.source_id, s.name AS source_name, d.path,
           d.document_profile, d.extraction_quality, d.warnings,
           (SELECT COUNT(*) FROM chunks count_chunks WHERE count_chunks.document_id = d.id) AS chunk_count
    FROM ranked r
    INNER JOIN documents d ON d.id = r.id
    INNER JOIN sources s ON s.id = d.source_id
    ORDER BY r.priority, lower(d.path) COLLATE \"C\", lower(s.name) COLLATE \"C\", d.id
    LIMIT $4";

const BROWSE_CHUNKS_SQL: &str = "SELECT c.id, c.document_id, c.source_id, s.name AS source_name,
        d.path AS document_path, c.ordinal,
        LEFT(c.text, $4) AS text_preview,
        char_length(c.text) > $4 AS preview_truncated,
        c.token_count, c.checksum, c.section_title, c.quality_flags,
        c.is_duplicate, c.text_density, c.evidence_score_hint
    FROM documents d
    INNER JOIN sources s ON s.id = d.source_id
    INNER JOIN projects p ON p.id = s.project_id
    CROSS JOIN LATERAL (
        SELECT candidate.id, candidate.ordinal
        FROM chunks candidate
        WHERE candidate.document_id = d.id
          AND NOT (candidate.id = ANY($2::uuid[]))
        ORDER BY candidate.ordinal, candidate.id
        LIMIT $3
    ) ranked_chunk
    INNER JOIN chunks c ON c.id = ranked_chunk.id
    WHERE p.workspace_id = $1
    ORDER BY lower(d.path) COLLATE \"C\", d.id, ranked_chunk.ordinal, c.id
    LIMIT $3";

const EXACT_CHUNKS_SQL: &str = "WITH matches AS (
        SELECT c.id, 0::smallint AS priority
        FROM chunks c
        INNER JOIN sources s ON s.id = c.source_id
        INNER JOIN projects p ON p.id = s.project_id
        WHERE p.workspace_id = $1
          AND c.id = $2
          AND NOT (c.id = ANY($3::uuid[]))
        UNION ALL
        SELECT c.id, 1::smallint AS priority
        FROM chunks c
        INNER JOIN sources s ON s.id = c.source_id
        INNER JOIN projects p ON p.id = s.project_id
        WHERE p.workspace_id = $1
          AND c.document_id = $2
          AND NOT (c.id = ANY($3::uuid[]))
    ), ranked AS (
        SELECT id, MIN(priority) AS priority FROM matches GROUP BY id
    )
    SELECT c.id, c.document_id, c.source_id, s.name AS source_name,
        d.path AS document_path, c.ordinal,
        LEFT(c.text, $5) AS text_preview,
        char_length(c.text) > $5 AS preview_truncated,
        c.token_count, c.checksum, c.section_title, c.quality_flags,
        c.is_duplicate, c.text_density, c.evidence_score_hint
    FROM ranked r
    INNER JOIN chunks c ON c.id = r.id
    INNER JOIN documents d ON d.id = c.document_id
    INNER JOIN sources s ON s.id = c.source_id
    ORDER BY r.priority, lower(d.path) COLLATE \"C\", d.id, c.ordinal, c.id
    LIMIT $4";

const TEXT_CHUNKS_SQL: &str = "WITH path_matches AS (
        SELECT c.id, 2::smallint AS priority
        FROM documents d
        INNER JOIN sources s ON s.id = d.source_id
        INNER JOIN projects p ON p.id = s.project_id
        INNER JOIN chunks c ON c.document_id = d.id
        WHERE p.workspace_id = $1
          AND NOT (c.id = ANY($3::uuid[]))
          AND lower(d.path) LIKE '%' || $2 || '%'
        ORDER BY lower(d.path) COLLATE \"C\", d.id, c.ordinal, c.id
        LIMIT $4
    ), section_matches AS (
        SELECT c.id, 3::smallint AS priority
        FROM chunks c
        INNER JOIN documents d ON d.id = c.document_id
        INNER JOIN sources s ON s.id = c.source_id
        INNER JOIN projects p ON p.id = s.project_id
        WHERE p.workspace_id = $1
          AND NOT (c.id = ANY($3::uuid[]))
          AND lower(COALESCE(c.section_title, '')) LIKE '%' || $2 || '%'
        ORDER BY lower(d.path) COLLATE \"C\", d.id, c.ordinal, c.id
        LIMIT $4
    ), source_matches AS (
        SELECT c.id, 4::smallint AS priority
        FROM sources s
        INNER JOIN projects p ON p.id = s.project_id
        INNER JOIN documents d ON d.source_id = s.id
        INNER JOIN chunks c ON c.document_id = d.id
        WHERE p.workspace_id = $1
          AND NOT (c.id = ANY($3::uuid[]))
          AND lower(s.name) LIKE '%' || $2 || '%'
        ORDER BY lower(d.path) COLLATE \"C\", lower(s.name) COLLATE \"C\", d.id, c.ordinal, c.id
        LIMIT $4
    ), body_matches AS (
        SELECT c.id, 5::smallint AS priority
        FROM chunks c
        INNER JOIN documents d ON d.id = c.document_id
        INNER JOIN sources s ON s.id = c.source_id
        INNER JOIN projects p ON p.id = s.project_id
        WHERE p.workspace_id = $1
          AND NOT (c.id = ANY($3::uuid[]))
          AND lower(c.text) LIKE '%' || $2 || '%'
        ORDER BY lower(d.path) COLLATE \"C\", d.id, c.ordinal, c.id
        LIMIT $4
    ), ranked AS (
        SELECT id, MIN(priority) AS priority
        FROM (
            SELECT * FROM path_matches
            UNION ALL SELECT * FROM section_matches
            UNION ALL SELECT * FROM source_matches
            UNION ALL SELECT * FROM body_matches
        ) matches
        GROUP BY id
    )
    SELECT c.id, c.document_id, c.source_id, s.name AS source_name,
        d.path AS document_path, c.ordinal,
        LEFT(c.text, $5) AS text_preview,
        char_length(c.text) > $5 AS preview_truncated,
        c.token_count, c.checksum, c.section_title, c.quality_flags,
        c.is_duplicate, c.text_density, c.evidence_score_hint
    FROM ranked r
    INNER JOIN chunks c ON c.id = r.id
    INNER JOIN documents d ON d.id = c.document_id
    INNER JOIN sources s ON s.id = c.source_id
    ORDER BY r.priority, lower(d.path) COLLATE \"C\", lower(s.name) COLLATE \"C\", d.id, c.ordinal, c.id
    LIMIT $4";

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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use serde_json::Value;
    use sqlx::PgPool;
    use uuid::Uuid;

    use super::*;

    const PLAN_DOCUMENT_COUNT: i32 = 20_000;
    const PLAN_CHUNKS_PER_DOCUMENT: i32 = 3;

    #[tokio::test]
    #[ignore = "requires a migrated Postgres database"]
    async fn postgres_evidence_query_plans_are_index_compatible() {
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is required");
        let store = PostgresStore::connect(&database_url)
            .await
            .expect("connect Postgres store");
        let pool = store.pool();
        let marker = Uuid::now_v7().simple().to_string();
        let organization_id = Uuid::now_v7();
        let workspace_id = Uuid::now_v7();
        let project_id = Uuid::now_v7();
        let source_id = Uuid::now_v7();

        seed_query_plan_corpus(
            pool,
            organization_id,
            workspace_id,
            project_id,
            source_id,
            &marker,
        )
        .await;

        let exact_document_id: Uuid = sqlx::query_scalar(
            "SELECT id FROM documents WHERE source_id = $1 ORDER BY path LIMIT 1",
        )
        .bind(source_id)
        .fetch_one(pool)
        .await
        .expect("exact document fixture");
        let exact_chunk_id: Uuid = sqlx::query_scalar(
            "SELECT id FROM chunks WHERE document_id = $1 ORDER BY ordinal LIMIT 1",
        )
        .bind(exact_document_id)
        .fetch_one(pool)
        .await
        .expect("exact chunk fixture");

        let browse_documents = explain_browse_documents(pool, workspace_id).await;
        let browse_chunks = explain_browse_chunks(pool, workspace_id).await;
        let exact_document = explain_exact_document(pool, workspace_id, exact_document_id).await;
        let exact_chunks = explain_exact_chunks(pool, workspace_id, exact_chunk_id).await;
        let path_search = explain_text_documents(pool, workspace_id, "zqx").await;
        let section_search = explain_text_chunks(pool, workspace_id, "vwx").await;
        let body_search = explain_text_chunks(pool, workspace_id, "jkp").await;

        sqlx::query("DELETE FROM projects WHERE id = $1")
            .bind(project_id)
            .execute(pool)
            .await
            .expect("clean query plan corpus");
        sqlx::query("DELETE FROM organizations WHERE id = $1")
            .bind(organization_id)
            .execute(pool)
            .await
            .expect("clean query plan organization");

        assert_uses_index(&browse_documents, "idx_documents_evidence_browse");
        assert_uses_index(&browse_chunks, "idx_documents_evidence_browse");
        assert_uses_index(&browse_chunks, "idx_chunks_evidence_browse");
        assert_no_relation_scan(&browse_chunks, "chunks");
        assert_no_large_sort(
            &browse_chunks,
            (PLAN_DOCUMENT_COUNT * PLAN_CHUNKS_PER_DOCUMENT) as u64,
        );

        assert_uses_index(&exact_document, "documents_pkey");
        assert_uses_index(&exact_chunks, "chunks_pkey");
        assert_uses_index(&exact_chunks, "idx_chunks_evidence_browse");
        // PostgreSQL may prefer the workspace-constrained ownership path over
        // the selective trigram path when a workspace contains very few
        // sources. Both plans remain index-backed and avoid global scans.
        assert_uses_one_of_indexes(
            &path_search,
            &["idx_documents_path_trgm", "idx_documents_source_id"],
        );
        assert_uses_one_of_indexes(
            &section_search,
            &[
                "idx_chunks_section_title_trgm",
                "idx_chunks_evidence_browse",
            ],
        );
        assert_uses_one_of_indexes(
            &body_search,
            &["idx_chunks_text_trgm", "idx_chunks_evidence_browse"],
        );
    }

    async fn seed_query_plan_corpus(
        pool: &PgPool,
        organization_id: Uuid,
        workspace_id: Uuid,
        project_id: Uuid,
        source_id: Uuid,
        marker: &str,
    ) {
        sqlx::query(
            "INSERT INTO organizations (id, name, created_at)
             VALUES ($1, $2, NOW())",
        )
        .bind(organization_id)
        .bind(format!("Evidence plan organization {marker}"))
        .execute(pool)
        .await
        .expect("insert plan organization");
        sqlx::query(
            "INSERT INTO workspaces (id, organization_id, name, created_at)
             VALUES ($1, $2, $3, NOW())",
        )
        .bind(workspace_id)
        .bind(organization_id)
        .bind(format!("Evidence plan workspace {marker}"))
        .execute(pool)
        .await
        .expect("insert plan workspace");
        sqlx::query(
            "INSERT INTO projects (
                id, workspace_id, name, privacy_mode, created_at, updated_at
             )
             VALUES ($1, $2, $3, 'local_only', NOW(), NOW())",
        )
        .bind(project_id)
        .bind(workspace_id)
        .bind(format!("Evidence plan project {marker}"))
        .execute(pool)
        .await
        .expect("insert plan project");
        sqlx::query(
            "INSERT INTO sources (
                id, project_id, name, source_kind, root_hint, sync_policy,
                target_tokens, overlap_tokens, created_at
             ) VALUES ($1, $2, $3, 'file_set', 'query-plan', 'manual', 512, 64, NOW())",
        )
        .bind(source_id)
        .bind(project_id)
        .bind(format!("needle-source-{marker}"))
        .execute(pool)
        .await
        .expect("insert plan source");
        sqlx::query(
            "INSERT INTO documents (
                id, source_id, path, mime_type, checksum, byte_size, created_at
             )
             SELECT md5($2 || '-document-' || item::text)::uuid,
                    $1,
                    CASE item
                      WHEN 4998 THEN 'zqx/' || $2 || '/guide.md'
                      WHEN 4999 THEN 'section-target-' || $2 || '/guide.md'
                      WHEN 5000 THEN 'body-target-' || $2 || '/guide.md'
                      ELSE 'browse/' || lpad(item::text, 6, '0') || '/guide.md'
                    END,
                    'text/markdown', md5($2 || '-checksum-' || item::text), 512, NOW()
             FROM generate_series(1, $3) AS items(item)",
        )
        .bind(source_id)
        .bind(marker)
        .bind(PLAN_DOCUMENT_COUNT)
        .execute(pool)
        .await
        .expect("insert plan documents");
        sqlx::query(
            "INSERT INTO chunks (
                id, source_id, document_id, ordinal, text, token_count,
                byte_start, byte_end, checksum, created_at, section_title
             )
             SELECT md5($2 || '-chunk-' || d.id::text || '-' || ordinal::text)::uuid,
                    $1, d.id, ordinal,
                    CASE
                      WHEN d.path LIKE 'body-target-%' AND ordinal = 1
                        THEN 'jkp uniquely identifies indexed body evidence ' || $2
                      ELSE 'bounded evidence fixture ' || d.path || ' chunk ' || ordinal::text
                    END,
                    12, 0, 128,
                    md5($2 || '-chunk-checksum-' || d.id::text || '-' || ordinal::text), NOW(),
                    CASE
                      WHEN d.path LIKE 'section-target-%' AND ordinal = 1
                        THEN 'vwx ' || $2
                      ELSE 'Evidence section'
                    END
             FROM documents d
             CROSS JOIN generate_series(0, $3 - 1) AS ordinals(ordinal)
             WHERE d.source_id = $1",
        )
        .bind(source_id)
        .bind(marker)
        .bind(PLAN_CHUNKS_PER_DOCUMENT)
        .execute(pool)
        .await
        .expect("insert plan chunks");
        sqlx::query("ANALYZE sources, documents, chunks")
            .execute(pool)
            .await
            .expect("analyze plan corpus");
    }

    async fn explain_browse_documents(pool: &PgPool, workspace_id: Uuid) -> Value {
        let sql = format!("EXPLAIN (FORMAT JSON) {BROWSE_DOCUMENTS_SQL}");
        sqlx::query_scalar(&sql)
            .bind(workspace_id)
            .bind(Vec::<Uuid>::new())
            .bind(1_i64)
            .fetch_one(pool)
            .await
            .expect("explain document browse")
    }

    async fn explain_browse_chunks(pool: &PgPool, workspace_id: Uuid) -> Value {
        let sql = format!("EXPLAIN (FORMAT JSON) {BROWSE_CHUNKS_SQL}");
        sqlx::query_scalar(&sql)
            .bind(workspace_id)
            .bind(Vec::<Uuid>::new())
            .bind(1_i64)
            .bind(EVAL_LAB_EVIDENCE_PREVIEW_CHAR_LIMIT as i32)
            .fetch_one(pool)
            .await
            .expect("explain chunk browse")
    }

    async fn explain_exact_document(pool: &PgPool, workspace_id: Uuid, id: Uuid) -> Value {
        let sql = format!("EXPLAIN (FORMAT JSON) {EXACT_DOCUMENT_SQL}");
        sqlx::query_scalar(&sql)
            .bind(workspace_id)
            .bind(id)
            .bind(Vec::<Uuid>::new())
            .bind(1_i64)
            .fetch_one(pool)
            .await
            .expect("explain exact document")
    }

    async fn explain_exact_chunks(pool: &PgPool, workspace_id: Uuid, id: Uuid) -> Value {
        let sql = format!("EXPLAIN (FORMAT JSON) {EXACT_CHUNKS_SQL}");
        sqlx::query_scalar(&sql)
            .bind(workspace_id)
            .bind(id)
            .bind(Vec::<Uuid>::new())
            .bind(1_i64)
            .bind(EVAL_LAB_EVIDENCE_PREVIEW_CHAR_LIMIT as i32)
            .fetch_one(pool)
            .await
            .expect("explain exact chunks")
    }

    async fn explain_text_documents(pool: &PgPool, workspace_id: Uuid, query: &str) -> Value {
        let sql = format!("EXPLAIN (FORMAT JSON) {TEXT_DOCUMENTS_SQL}");
        sqlx::query_scalar(&sql)
            .bind(workspace_id)
            .bind(query)
            .bind(Vec::<Uuid>::new())
            .bind(1_i64)
            .fetch_one(pool)
            .await
            .expect("explain text documents")
    }

    async fn explain_text_chunks(pool: &PgPool, workspace_id: Uuid, query: &str) -> Value {
        let sql = format!("EXPLAIN (FORMAT JSON) {TEXT_CHUNKS_SQL}");
        sqlx::query_scalar(&sql)
            .bind(workspace_id)
            .bind(query)
            .bind(Vec::<Uuid>::new())
            .bind(1_i64)
            .bind(EVAL_LAB_EVIDENCE_PREVIEW_CHAR_LIMIT as i32)
            .fetch_one(pool)
            .await
            .expect("explain text chunks")
    }

    fn assert_uses_index(plan: &Value, expected: &str) {
        let indexes = plan_values(plan, "Index Name")
            .into_iter()
            .filter_map(Value::as_str)
            .collect::<HashSet<_>>();
        assert!(
            indexes.contains(expected),
            "expected {expected} in query plan indexes {indexes:?}: {plan}"
        );
    }

    fn assert_uses_one_of_indexes(plan: &Value, expected: &[&str]) {
        let indexes = plan_values(plan, "Index Name")
            .into_iter()
            .filter_map(Value::as_str)
            .collect::<HashSet<_>>();
        assert!(
            expected.iter().any(|candidate| indexes.contains(candidate)),
            "expected one of {expected:?} in query plan indexes {indexes:?}: {plan}"
        );
    }

    fn assert_no_relation_scan(plan: &Value, relation: &str) {
        let has_scan = plan_objects(plan).into_iter().any(|node| {
            node.get("Node Type").and_then(Value::as_str) == Some("Seq Scan")
                && node.get("Relation Name").and_then(Value::as_str) == Some(relation)
        });
        assert!(
            !has_scan,
            "unexpected sequential scan of {relation}: {plan}"
        );
    }

    fn assert_no_large_sort(plan: &Value, corpus_rows: u64) {
        let has_large_sort = plan_objects(plan).into_iter().any(|node| {
            node.get("Node Type")
                .and_then(Value::as_str)
                .is_some_and(|node_type| node_type.contains("Sort"))
                && node
                    .get("Plan Rows")
                    .and_then(Value::as_u64)
                    .is_some_and(|rows| rows >= corpus_rows)
        });
        assert!(!has_large_sort, "unexpected full-corpus sort: {plan}");
    }

    fn plan_values<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
        let mut matches = Vec::new();
        visit_plan(value, &mut |object| {
            if let Some(value) = object.get(key) {
                matches.push(value);
            }
        });
        matches
    }

    fn plan_objects(value: &Value) -> Vec<&serde_json::Map<String, Value>> {
        let mut objects = Vec::new();
        visit_plan(value, &mut |object| objects.push(object));
        objects
    }

    fn visit_plan<'a>(
        value: &'a Value,
        visitor: &mut impl FnMut(&'a serde_json::Map<String, Value>),
    ) {
        match value {
            Value::Array(values) => {
                for value in values {
                    visit_plan(value, visitor);
                }
            }
            Value::Object(object) => {
                visitor(object);
                for value in object.values() {
                    visit_plan(value, visitor);
                }
            }
            _ => {}
        }
    }
}
