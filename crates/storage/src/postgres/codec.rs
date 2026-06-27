use rag_debugger_core::*;
use sqlx::{types::Json, PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::StorageError;

pub(super) fn project_from_row(row: &sqlx::postgres::PgRow) -> Result<Project, StorageError> {
    Ok(Project {
        id: ProjectId(row.try_get("id")?),
        name: row.try_get("name")?,
        privacy_mode: privacy_mode_from_str(row.try_get::<String, _>("privacy_mode")?.as_str())?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

pub(super) fn source_from_row(row: &sqlx::postgres::PgRow) -> Result<Source, StorageError> {
    let source_kind = row.try_get::<String, _>("source_kind")?;
    let sync_policy = row.try_get::<String, _>("sync_policy")?;
    let target_tokens = row.try_get::<i32, _>("target_tokens")?;
    let overlap_tokens = row.try_get::<i32, _>("overlap_tokens")?;
    let chunking_strategy = row.try_get::<String, _>("chunking_strategy")?;

    Ok(Source {
        id: SourceId(row.try_get("id")?),
        project_id: ProjectId(row.try_get("project_id")?),
        name: row.try_get("name")?,
        kind: source_kind_from_columns(
            &source_kind,
            row.try_get("root_hint")?,
            row.try_get("github_owner")?,
            row.try_get("github_repo")?,
        )?,
        sync_policy: sync_policy_from_columns(&sync_policy, row.try_get("sync_cron")?)?,
        chunking: ChunkingConfig {
            target_tokens: as_u32(target_tokens, "target_tokens")?,
            overlap_tokens: as_u32(overlap_tokens, "overlap_tokens")?,
            strategy: chunking_strategy_from_str(&chunking_strategy)?,
        },
    })
}

pub(super) fn document_from_row(row: &sqlx::postgres::PgRow) -> Result<Document, StorageError> {
    let byte_size = row.try_get::<i64, _>("byte_size")?;
    let warnings = row
        .try_get::<Vec<String>, _>("warnings")
        .unwrap_or_default();
    Ok(Document {
        id: DocumentId(row.try_get("id")?),
        source_id: SourceId(row.try_get("source_id")?),
        path: row.try_get("path")?,
        mime_type: row.try_get("mime_type")?,
        checksum: row.try_get("checksum")?,
        byte_size: as_u64(byte_size, "byte_size")?,
        profile: document_profile_from_str(
            row.try_get::<String, _>("document_profile")
                .unwrap_or_else(|_| "general".to_owned())
                .as_str(),
        )?,
        extraction_quality: extraction_quality_from_str(
            row.try_get::<String, _>("extraction_quality")
                .unwrap_or_else(|_| "unknown".to_owned())
                .as_str(),
        )?,
        warnings: document_warnings_from_text(warnings),
    })
}

pub(super) fn chunk_from_row(row: &sqlx::postgres::PgRow) -> Result<Chunk, StorageError> {
    let ordinal = row.try_get::<i32, _>("ordinal")?;
    let token_count = row.try_get::<i32, _>("token_count")?;
    let byte_start = row.try_get::<i64, _>("byte_start")?;
    let byte_end = row.try_get::<i64, _>("byte_end")?;
    Ok(Chunk {
        id: ChunkId(row.try_get("id")?),
        source_id: SourceId(row.try_get("source_id")?),
        document_id: DocumentId(row.try_get("document_id")?),
        ordinal: as_u32(ordinal, "ordinal")?,
        text: row.try_get("text")?,
        token_count: as_u32(token_count, "token_count")?,
        byte_range: ByteRange {
            start: as_u64(byte_start, "byte_start")?,
            end: as_u64(byte_end, "byte_end")?,
        },
        checksum: row.try_get("checksum")?,
        strategy: chunking_strategy_from_str(row.try_get::<String, _>("strategy")?.as_str())?,
        section_title: row.try_get("section_title")?,
        split_reason: chunk_split_reason_from_str(
            row.try_get::<String, _>("split_reason")?.as_str(),
        )?,
        quality_flags: chunk_quality_flags_from_text(
            row.try_get::<Vec<String>, _>("quality_flags")
                .unwrap_or_default(),
        )?,
        is_duplicate: row.try_get("is_duplicate").unwrap_or(false),
        text_density: row.try_get("text_density").unwrap_or(0.0),
        evidence_score_hint: row.try_get("evidence_score_hint").unwrap_or(0.0),
    })
}

pub(super) fn searchable_chunk_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<SearchableChunk, StorageError> {
    let source_kind = row.try_get::<String, _>("source_kind")?;
    let sync_policy = row.try_get::<String, _>("sync_policy")?;
    let target_tokens = row.try_get::<i32, _>("target_tokens")?;
    let overlap_tokens = row.try_get::<i32, _>("overlap_tokens")?;
    let chunking_strategy = row.try_get::<String, _>("chunking_strategy")?;
    let byte_size = row.try_get::<i64, _>("byte_size")?;
    let ordinal = row.try_get::<i32, _>("ordinal")?;
    let token_count = row.try_get::<i32, _>("token_count")?;
    let byte_start = row.try_get::<i64, _>("byte_start")?;
    let byte_end = row.try_get::<i64, _>("byte_end")?;
    let document_warnings = row
        .try_get::<Vec<String>, _>("warnings")
        .unwrap_or_default();

    let source = Source {
        id: SourceId(row.try_get("source_id")?),
        project_id: ProjectId(row.try_get("project_id")?),
        name: row.try_get("source_name")?,
        kind: source_kind_from_columns(
            &source_kind,
            row.try_get("root_hint")?,
            row.try_get("github_owner")?,
            row.try_get("github_repo")?,
        )?,
        sync_policy: sync_policy_from_columns(&sync_policy, row.try_get("sync_cron")?)?,
        chunking: ChunkingConfig {
            target_tokens: as_u32(target_tokens, "target_tokens")?,
            overlap_tokens: as_u32(overlap_tokens, "overlap_tokens")?,
            strategy: chunking_strategy_from_str(&chunking_strategy)?,
        },
    };
    let document = Document {
        id: DocumentId(row.try_get("document_id")?),
        source_id: SourceId(row.try_get("document_source_id")?),
        path: row.try_get("document_path")?,
        mime_type: row.try_get("mime_type")?,
        checksum: row.try_get("document_checksum")?,
        byte_size: as_u64(byte_size, "byte_size")?,
        profile: document_profile_from_str(
            row.try_get::<String, _>("document_profile")
                .unwrap_or_else(|_| "general".to_owned())
                .as_str(),
        )?,
        extraction_quality: extraction_quality_from_str(
            row.try_get::<String, _>("extraction_quality")
                .unwrap_or_else(|_| "unknown".to_owned())
                .as_str(),
        )?,
        warnings: document_warnings_from_text(document_warnings),
    };
    let chunk = Chunk {
        id: ChunkId(row.try_get("chunk_id")?),
        source_id: SourceId(row.try_get("chunk_source_id")?),
        document_id: DocumentId(row.try_get("chunk_document_id")?),
        ordinal: as_u32(ordinal, "ordinal")?,
        text: row.try_get("text")?,
        token_count: as_u32(token_count, "token_count")?,
        byte_range: ByteRange {
            start: as_u64(byte_start, "byte_start")?,
            end: as_u64(byte_end, "byte_end")?,
        },
        checksum: row.try_get("chunk_checksum")?,
        strategy: chunking_strategy_from_str(row.try_get::<String, _>("strategy")?.as_str())?,
        section_title: row.try_get("section_title")?,
        split_reason: chunk_split_reason_from_str(
            row.try_get::<String, _>("split_reason")?.as_str(),
        )?,
        quality_flags: chunk_quality_flags_from_text(
            row.try_get::<Vec<String>, _>("quality_flags")
                .unwrap_or_default(),
        )?,
        is_duplicate: row.try_get("is_duplicate").unwrap_or(false),
        text_density: row.try_get("text_density").unwrap_or(0.0),
        evidence_score_hint: row.try_get("evidence_score_hint").unwrap_or(0.0),
    };
    let embedding = chunk_embedding_from_row(row)?;

    Ok(SearchableChunk {
        source,
        document,
        chunk,
        embedding,
    })
}

pub(super) fn chunk_embedding_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<Option<ChunkEmbedding>, StorageError> {
    let Some(model_provider) = row.try_get::<Option<String>, _>("embedding_model_provider")? else {
        return Ok(None);
    };
    let model_name = row
        .try_get::<Option<String>, _>("embedding_model_name")?
        .ok_or_else(|| StorageError::InvalidData("embedding model name is missing".to_owned()))?;
    let dimension = row
        .try_get::<Option<i32>, _>("embedding_dimension")?
        .ok_or_else(|| StorageError::InvalidData("embedding dimension is missing".to_owned()))?;
    let vector = row
        .try_get::<Option<Vec<f32>>, _>("embedding_vector")?
        .ok_or_else(|| StorageError::InvalidData("embedding vector is missing".to_owned()))?;
    let chunk_checksum = row
        .try_get::<Option<String>, _>("embedding_chunk_checksum")?
        .ok_or_else(|| StorageError::InvalidData("embedding checksum is missing".to_owned()))?;
    let indexed_at = row
        .try_get::<Option<OffsetDateTime>, _>("embedding_indexed_at")?
        .ok_or_else(|| StorageError::InvalidData("embedding indexed_at is missing".to_owned()))?;

    Ok(Some(ChunkEmbedding {
        chunk_id: ChunkId(row.try_get("chunk_id")?),
        chunk_checksum,
        model: EmbeddingModelInfo {
            provider: model_provider,
            model_name,
            dimension: as_u32(dimension, "embedding_dimension")?,
        },
        vector,
        indexed_at,
    }))
}

pub(super) fn retrieval_eval_case_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<RetrievalEvalCase, StorageError> {
    let top_k = row.try_get::<i32, _>("top_k")?;
    let expected_chunk_ids = row
        .try_get::<Vec<Uuid>, _>("expected_chunk_ids")?
        .into_iter()
        .map(ChunkId)
        .collect();
    let expected_document_ids = row
        .try_get::<Vec<Uuid>, _>("expected_document_ids")?
        .into_iter()
        .map(DocumentId)
        .collect();

    Ok(RetrievalEvalCase {
        id: RetrievalEvalCaseId(row.try_get("id")?),
        name: row.try_get("name")?,
        query: row.try_get("query")?,
        top_k: as_u32(top_k, "top_k")?,
        expected_chunk_ids,
        expected_document_ids,
        notes: row.try_get("notes")?,
        created_at: row.try_get("created_at")?,
    })
}

pub(super) fn retrieval_eval_result_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<RetrievalEvalResult, StorageError> {
    let expected_chunk_ids = row
        .try_get::<Vec<Uuid>, _>("expected_chunk_ids")?
        .into_iter()
        .map(ChunkId)
        .collect();
    let expected_document_ids = row
        .try_get::<Vec<Uuid>, _>("expected_document_ids")?
        .into_iter()
        .map(DocumentId)
        .collect();
    let retrieved_chunk_ids = row
        .try_get::<Vec<Uuid>, _>("retrieved_chunk_ids")?
        .into_iter()
        .map(ChunkId)
        .collect();

    Ok(RetrievalEvalResult {
        case_id: RetrievalEvalCaseId(row.try_get("case_id")?),
        query: row.try_get("query")?,
        top_k: as_u32(row.try_get("top_k")?, "top_k")?,
        recall_at_k: row.try_get("recall_at_k")?,
        precision_at_k: row.try_get("precision_at_k")?,
        top_hit_rank: row
            .try_get::<Option<i32>, _>("top_hit_rank")?
            .map(|rank| as_u32(rank, "top_hit_rank"))
            .transpose()?,
        passed: row.try_get("passed")?,
        expected_chunk_ids,
        expected_document_ids,
        retrieved_chunk_ids,
        latency_ms: as_u64(row.try_get("latency_ms")?, "latency_ms")?,
    })
}

pub(super) fn eval_dataset_summary_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<RetrievalEvalDatasetSummary, StorageError> {
    let case_count = row.try_get::<i32, _>("case_count")?;
    let latest_experiment = row
        .try_get::<Option<Json<RetrievalEvalExperiment>>, _>("latest_experiment_json")?
        .map(|json| json.0);
    let best_result = latest_experiment.as_ref().and_then(|experiment| {
        experiment.mode_results.iter().max_by(|left, right| {
            left.average_recall_at_k
                .partial_cmp(&right.average_recall_at_k)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    Ok(RetrievalEvalDatasetSummary {
        id: RetrievalEvalDatasetId(row.try_get("id")?),
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        case_count: as_u32(case_count, "case_count")?,
        latest_experiment_id: latest_experiment.as_ref().map(|experiment| experiment.id),
        latest_gate: latest_experiment
            .as_ref()
            .map(|experiment| experiment.gate.clone()),
        latest_average_recall_at_k: best_result.map(|result| result.average_recall_at_k),
        latest_average_precision_at_k: best_result.map(|result| result.average_precision_at_k),
        updated_at: row.try_get("updated_at")?,
    })
}

pub(super) fn eval_experiment_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<RetrievalEvalExperiment, StorageError> {
    Ok(row
        .try_get::<Json<RetrievalEvalExperiment>, _>("experiment_json")?
        .0)
}

pub(super) fn default_eval_dataset_id() -> RetrievalEvalDatasetId {
    RetrievalEvalDatasetId(Uuid::from_u128(0x018f_7a2a_6e2e_7000_a000_0000_0000_e001))
}

pub(super) async fn ensure_default_eval_dataset(pool: &PgPool) -> Result<(), StorageError> {
    let now = OffsetDateTime::now_utc();
    sqlx::query(
        "INSERT INTO retrieval_eval_datasets (id, name, description, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(default_eval_dataset_id().0)
    .bind("Default retrieval dataset")
    .bind("Backfilled and manually saved retrieval eval cases.")
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    sqlx::query(
        "UPDATE retrieval_eval_cases
         SET dataset_id = $1
         WHERE dataset_id IS NULL",
    )
    .bind(default_eval_dataset_id().0)
    .execute(pool)
    .await?;

    Ok(())
}

pub(super) fn retrieval_response_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<RetrievalQueryResponse, StorageError> {
    let response = row
        .try_get::<Option<Json<RetrievalQueryResponse>>, _>("response_json")?
        .ok_or_else(|| {
            StorageError::InvalidData(
                "retrieval response JSON was not stored for this run".to_owned(),
            )
        })?;
    Ok(response.0)
}

pub(super) fn trace_from_row(row: &sqlx::postgres::PgRow) -> Result<Trace, StorageError> {
    let trace = row.try_get::<Json<Trace>, _>("trace_json")?;
    Ok(trace.0)
}

pub(super) fn trace_summary_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<TraceSummary, StorageError> {
    Ok(TraceSummary {
        id: TraceId(row.try_get("id")?),
        query: row.try_get("query")?,
        retrieval_mode: retrieval_mode_from_str(
            row.try_get::<String, _>("retrieval_mode")?.as_str(),
        )?,
        latency_ms: as_u64(row.try_get("latency_ms")?, "latency_ms")?,
        evidence_strength: evidence_strength_from_str(
            row.try_get::<String, _>("evidence_strength")?.as_str(),
        )?,
        failure_labels: failure_labels_from_text(
            row.try_get::<Vec<String>, _>("failure_labels")
                .unwrap_or_default(),
        )?,
        span_count: as_u32(row.try_get("span_count")?, "span_count")?,
        rerun_count: as_u32(row.try_get("rerun_count")?, "rerun_count")?,
        created_at: row.try_get("created_at")?,
    })
}

pub(super) fn ingestion_run_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<IngestionRun, StorageError> {
    Ok(IngestionRun {
        id: IngestionRunId(row.try_get("id")?),
        source_id: SourceId(row.try_get("source_id")?),
        status: ingestion_status_from_str(row.try_get::<String, _>("status")?.as_str())?,
        totals: IngestionTotals {
            files_received: as_u32(row.try_get("files_received")?, "files_received")?,
            documents_created: as_u32(row.try_get("documents_created")?, "documents_created")?,
            chunks_created: as_u32(row.try_get("chunks_created")?, "chunks_created")?,
            failed_files: as_u32(row.try_get("failed_files")?, "failed_files")?,
        },
        started_at: row.try_get("started_at")?,
        completed_at: row.try_get("completed_at")?,
    })
}

pub(super) fn source_kind_columns(
    kind: &SourceKind,
) -> (&'static str, Option<String>, Option<String>, Option<String>) {
    match kind {
        SourceKind::FileSet { root_hint } => ("file_set", Some(root_hint.clone()), None, None),
        SourceKind::GitHubRepository { owner, repo } => (
            "github_repository",
            None,
            Some(owner.clone()),
            Some(repo.clone()),
        ),
    }
}

pub(super) fn source_kind_from_columns(
    kind: &str,
    root_hint: Option<String>,
    github_owner: Option<String>,
    github_repo: Option<String>,
) -> Result<SourceKind, StorageError> {
    match kind {
        "file_set" => Ok(SourceKind::FileSet {
            root_hint: root_hint.unwrap_or_default(),
        }),
        "github_repository" => Ok(SourceKind::GitHubRepository {
            owner: github_owner.unwrap_or_default(),
            repo: github_repo.unwrap_or_default(),
        }),
        _ => Err(StorageError::InvalidData(format!(
            "unknown source kind: {kind}"
        ))),
    }
}

pub(super) fn sync_policy_columns(policy: &SourceSyncPolicy) -> (&'static str, Option<String>) {
    match policy {
        SourceSyncPolicy::Manual => ("manual", None),
        SourceSyncPolicy::OnDemand => ("on_demand", None),
        SourceSyncPolicy::Scheduled { cron } => ("scheduled", Some(cron.clone())),
    }
}

pub(super) fn sync_policy_from_columns(
    policy: &str,
    cron: Option<String>,
) -> Result<SourceSyncPolicy, StorageError> {
    match policy {
        "manual" => Ok(SourceSyncPolicy::Manual),
        "on_demand" => Ok(SourceSyncPolicy::OnDemand),
        "scheduled" => Ok(SourceSyncPolicy::Scheduled {
            cron: cron.unwrap_or_default(),
        }),
        _ => Err(StorageError::InvalidData(format!(
            "unknown sync policy: {policy}"
        ))),
    }
}

pub(super) fn privacy_mode_to_str(mode: PrivacyMode) -> &'static str {
    match mode {
        PrivacyMode::LocalOnly => "local_only",
        PrivacyMode::RedactedCloudSync => "redacted_cloud_sync",
        PrivacyMode::ExplicitSnippetSync => "explicit_snippet_sync",
    }
}

pub(super) fn privacy_mode_from_str(mode: &str) -> Result<PrivacyMode, StorageError> {
    match mode {
        "local_only" => Ok(PrivacyMode::LocalOnly),
        "redacted_cloud_sync" => Ok(PrivacyMode::RedactedCloudSync),
        "explicit_snippet_sync" => Ok(PrivacyMode::ExplicitSnippetSync),
        _ => Err(StorageError::InvalidData(format!(
            "unknown privacy mode: {mode}"
        ))),
    }
}

pub(super) fn ingestion_status_to_str(status: IngestionRunStatus) -> &'static str {
    match status {
        IngestionRunStatus::Running => "running",
        IngestionRunStatus::Completed => "completed",
        IngestionRunStatus::Partial => "partial",
        IngestionRunStatus::Failed => "failed",
    }
}

pub(super) fn ingestion_status_from_str(status: &str) -> Result<IngestionRunStatus, StorageError> {
    match status {
        "running" => Ok(IngestionRunStatus::Running),
        "completed" => Ok(IngestionRunStatus::Completed),
        "partial" => Ok(IngestionRunStatus::Partial),
        "failed" => Ok(IngestionRunStatus::Failed),
        _ => Err(StorageError::InvalidData(format!(
            "unknown ingestion status: {status}"
        ))),
    }
}

pub(super) fn chunking_strategy_to_str(strategy: ChunkingStrategy) -> &'static str {
    match strategy.normalized() {
        ChunkingStrategy::Structured | ChunkingStrategy::SmartSections => "structured",
        ChunkingStrategy::Whitespace => "whitespace",
    }
}

pub(super) fn chunking_strategy_from_str(strategy: &str) -> Result<ChunkingStrategy, StorageError> {
    match strategy {
        "structured" | "smart_sections" => Ok(ChunkingStrategy::Structured),
        "whitespace" => Ok(ChunkingStrategy::Whitespace),
        _ => Err(StorageError::InvalidData(format!(
            "unknown chunking strategy: {strategy}"
        ))),
    }
}

pub(super) fn document_profile_to_str(profile: DocumentProfile) -> &'static str {
    match profile {
        DocumentProfile::General => "general",
        DocumentProfile::TechnicalDocs => "technical_docs",
        DocumentProfile::PolicyOrLegal => "policy_or_legal",
        DocumentProfile::SupportKb => "support_kb",
        DocumentProfile::ResearchPaper => "research_paper",
        DocumentProfile::CodeDocs => "code_docs",
        DocumentProfile::Resume => "resume",
    }
}

pub(super) fn document_profile_from_str(profile: &str) -> Result<DocumentProfile, StorageError> {
    match profile {
        "general" => Ok(DocumentProfile::General),
        "technical_docs" => Ok(DocumentProfile::TechnicalDocs),
        "policy_or_legal" => Ok(DocumentProfile::PolicyOrLegal),
        "support_kb" => Ok(DocumentProfile::SupportKb),
        "research_paper" => Ok(DocumentProfile::ResearchPaper),
        "code_docs" => Ok(DocumentProfile::CodeDocs),
        "resume" => Ok(DocumentProfile::Resume),
        _ => Err(StorageError::InvalidData(format!(
            "unknown document profile: {profile}"
        ))),
    }
}

pub(super) fn extraction_quality_to_str(quality: ExtractionQuality) -> &'static str {
    match quality {
        ExtractionQuality::High => "high",
        ExtractionQuality::Medium => "medium",
        ExtractionQuality::Low => "low",
        ExtractionQuality::Unknown => "unknown",
    }
}

pub(super) fn extraction_quality_from_str(
    quality: &str,
) -> Result<ExtractionQuality, StorageError> {
    match quality {
        "high" => Ok(ExtractionQuality::High),
        "medium" => Ok(ExtractionQuality::Medium),
        "low" => Ok(ExtractionQuality::Low),
        "unknown" => Ok(ExtractionQuality::Unknown),
        _ => Err(StorageError::InvalidData(format!(
            "unknown extraction quality: {quality}"
        ))),
    }
}

pub(super) fn chunk_quality_flag_to_str(flag: ChunkQualityFlag) -> &'static str {
    match flag {
        ChunkQualityFlag::HeadingOnly => "heading_only",
        ChunkQualityFlag::TooShort => "too_short",
        ChunkQualityFlag::TooLong => "too_long",
        ChunkQualityFlag::Duplicate => "duplicate",
        ChunkQualityFlag::LowTextDensity => "low_text_density",
        ChunkQualityFlag::ExtractionWarning => "extraction_warning",
        ChunkQualityFlag::GoodEvidenceCandidate => "good_evidence_candidate",
    }
}

pub(super) fn chunk_quality_flag_from_str(flag: &str) -> Result<ChunkQualityFlag, StorageError> {
    match flag {
        "heading_only" => Ok(ChunkQualityFlag::HeadingOnly),
        "too_short" => Ok(ChunkQualityFlag::TooShort),
        "too_long" => Ok(ChunkQualityFlag::TooLong),
        "duplicate" => Ok(ChunkQualityFlag::Duplicate),
        "low_text_density" => Ok(ChunkQualityFlag::LowTextDensity),
        "extraction_warning" => Ok(ChunkQualityFlag::ExtractionWarning),
        "good_evidence_candidate" => Ok(ChunkQualityFlag::GoodEvidenceCandidate),
        _ => Err(StorageError::InvalidData(format!(
            "unknown chunk quality flag: {flag}"
        ))),
    }
}

pub(super) fn chunk_quality_flags_to_text(flags: &[ChunkQualityFlag]) -> Vec<String> {
    flags
        .iter()
        .map(|flag| chunk_quality_flag_to_str(*flag).to_owned())
        .collect()
}

pub(super) fn chunk_quality_flags_from_text(
    flags: Vec<String>,
) -> Result<Vec<ChunkQualityFlag>, StorageError> {
    flags
        .iter()
        .map(|flag| chunk_quality_flag_from_str(flag))
        .collect()
}

pub(super) fn document_warnings_to_text(warnings: &[DocumentWarning]) -> Vec<String> {
    warnings
        .iter()
        .map(|warning| format!("{}:{}", warning.code, warning.message))
        .collect()
}

pub(super) fn document_warnings_from_text(warnings: Vec<String>) -> Vec<DocumentWarning> {
    warnings
        .into_iter()
        .map(|warning| {
            let (code, message) = warning
                .split_once(':')
                .map(|(code, message)| (code.to_owned(), message.to_owned()))
                .unwrap_or_else(|| ("warning".to_owned(), warning));
            DocumentWarning { code, message }
        })
        .collect()
}

pub(super) fn chunk_split_reason_to_str(reason: ChunkSplitReason) -> &'static str {
    match reason {
        ChunkSplitReason::SectionBoundary => "section_boundary",
        ChunkSplitReason::TokenLimit => "token_limit",
        ChunkSplitReason::DocumentEnd => "document_end",
        ChunkSplitReason::FallbackWhitespace => "fallback_whitespace",
    }
}

pub(super) fn chunk_split_reason_from_str(reason: &str) -> Result<ChunkSplitReason, StorageError> {
    match reason {
        "section_boundary" => Ok(ChunkSplitReason::SectionBoundary),
        "token_limit" => Ok(ChunkSplitReason::TokenLimit),
        "document_end" => Ok(ChunkSplitReason::DocumentEnd),
        "fallback_whitespace" => Ok(ChunkSplitReason::FallbackWhitespace),
        _ => Err(StorageError::InvalidData(format!(
            "unknown chunk split reason: {reason}"
        ))),
    }
}

pub(super) fn answer_status_to_str(status: ExtractiveAnswerStatus) -> &'static str {
    match status {
        ExtractiveAnswerStatus::Answered => "answered",
        ExtractiveAnswerStatus::InsufficientEvidence => "insufficient_evidence",
    }
}

pub(super) fn retrieval_mode_to_str(mode: RetrievalMode) -> &'static str {
    match mode {
        RetrievalMode::Lexical => "lexical",
        RetrievalMode::Vector => "vector",
        RetrievalMode::Hybrid => "hybrid",
    }
}

pub(super) fn retrieval_mode_from_str(mode: &str) -> Result<RetrievalMode, StorageError> {
    match mode {
        "lexical" => Ok(RetrievalMode::Lexical),
        "vector" => Ok(RetrievalMode::Vector),
        "hybrid" => Ok(RetrievalMode::Hybrid),
        _ => Err(StorageError::InvalidData(format!(
            "unknown retrieval mode: {mode}"
        ))),
    }
}

pub(super) fn eval_gate_status_to_str(status: RetrievalEvalGateStatus) -> &'static str {
    match status {
        RetrievalEvalGateStatus::Passed => "passed",
        RetrievalEvalGateStatus::Failed => "failed",
    }
}

pub(super) fn workspace_role_to_str(role: WorkspaceRole) -> &'static str {
    match role {
        WorkspaceRole::Owner => "owner",
        WorkspaceRole::Admin => "admin",
        WorkspaceRole::Member => "member",
        WorkspaceRole::Viewer => "viewer",
    }
}

pub(super) fn workspace_role_from_str(role: &str) -> Result<WorkspaceRole, StorageError> {
    match role {
        "owner" => Ok(WorkspaceRole::Owner),
        "admin" => Ok(WorkspaceRole::Admin),
        "member" => Ok(WorkspaceRole::Member),
        "viewer" => Ok(WorkspaceRole::Viewer),
        other => Err(StorageError::InvalidData(format!(
            "unknown workspace role: {other}"
        ))),
    }
}

pub(super) fn api_key_scopes_to_text(scopes: &[ApiKeyScope]) -> Vec<String> {
    scopes
        .iter()
        .map(|scope| match scope {
            ApiKeyScope::CiEvalRuns => "ci_eval_runs".to_owned(),
        })
        .collect()
}

pub(super) fn api_key_scopes_from_text(
    scopes: Vec<String>,
) -> Result<Vec<ApiKeyScope>, StorageError> {
    scopes
        .into_iter()
        .map(|scope| match scope.as_str() {
            "ci_eval_runs" => Ok(ApiKeyScope::CiEvalRuns),
            other => Err(StorageError::InvalidData(format!(
                "unknown api key scope: {other}"
            ))),
        })
        .collect()
}

pub(super) fn ci_eval_run_status_to_str(status: CiEvalRunStatus) -> &'static str {
    match status {
        CiEvalRunStatus::Passed => "passed",
        CiEvalRunStatus::Failed => "failed",
    }
}

pub(super) fn trace_status_to_str(status: TraceStatus) -> &'static str {
    match status {
        TraceStatus::Completed => "completed",
        TraceStatus::Warning => "warning",
        TraceStatus::Failed => "failed",
    }
}

pub(super) fn evidence_strength_to_str(strength: EvidenceStrength) -> &'static str {
    match strength {
        EvidenceStrength::Strong => "strong",
        EvidenceStrength::Medium => "medium",
        EvidenceStrength::Weak => "weak",
    }
}

pub(super) fn evidence_strength_from_str(strength: &str) -> Result<EvidenceStrength, StorageError> {
    match strength {
        "strong" => Ok(EvidenceStrength::Strong),
        "medium" => Ok(EvidenceStrength::Medium),
        "weak" => Ok(EvidenceStrength::Weak),
        _ => Err(StorageError::InvalidData(format!(
            "unknown evidence strength: {strength}"
        ))),
    }
}

pub(super) fn failure_label_to_str(label: &FailureLabel) -> &'static str {
    match label {
        FailureLabel::MissingDocument => "missing_document",
        FailureLabel::BadChunking => "bad_chunking",
        FailureLabel::BadEmbedding => "bad_embedding",
        FailureLabel::BadRanking => "bad_ranking",
        FailureLabel::BadPrompt => "bad_prompt",
        FailureLabel::UnsupportedQuestion => "unsupported_question",
        FailureLabel::HallucinatedAnswer => "hallucinated_answer",
        FailureLabel::WeakEvidence => "weak_evidence",
        FailureLabel::MissingEmbeddingIndex => "missing_embedding_index",
        FailureLabel::DuplicateEvidence => "duplicate_evidence",
        FailureLabel::HeadingOnlyEvidence => "heading_only_evidence",
    }
}

pub(super) fn failure_label_from_str(label: &str) -> Result<FailureLabel, StorageError> {
    match label {
        "missing_document" => Ok(FailureLabel::MissingDocument),
        "bad_chunking" => Ok(FailureLabel::BadChunking),
        "bad_embedding" => Ok(FailureLabel::BadEmbedding),
        "bad_ranking" => Ok(FailureLabel::BadRanking),
        "bad_prompt" => Ok(FailureLabel::BadPrompt),
        "unsupported_question" => Ok(FailureLabel::UnsupportedQuestion),
        "hallucinated_answer" => Ok(FailureLabel::HallucinatedAnswer),
        "weak_evidence" => Ok(FailureLabel::WeakEvidence),
        "missing_embedding_index" => Ok(FailureLabel::MissingEmbeddingIndex),
        "duplicate_evidence" => Ok(FailureLabel::DuplicateEvidence),
        "heading_only_evidence" => Ok(FailureLabel::HeadingOnlyEvidence),
        _ => Err(StorageError::InvalidData(format!(
            "unknown failure label: {label}"
        ))),
    }
}

pub(super) fn failure_labels_to_text(labels: &[FailureLabel]) -> Vec<String> {
    labels
        .iter()
        .map(|label| failure_label_to_str(label).to_owned())
        .collect()
}

pub(super) fn failure_labels_from_text(
    labels: Vec<String>,
) -> Result<Vec<FailureLabel>, StorageError> {
    labels
        .iter()
        .map(|label| failure_label_from_str(label))
        .collect()
}

pub(super) fn source_filter_ids(source_ids: &[SourceId]) -> Vec<Uuid> {
    source_ids.iter().map(|source_id| source_id.0).collect()
}

pub(super) fn document_filter_ids(document_ids: &[DocumentId]) -> Vec<Uuid> {
    document_ids
        .iter()
        .map(|document_id| document_id.0)
        .collect()
}

pub(super) fn matched_terms_to_text(terms: &[RetrievalMatchedTerm]) -> String {
    terms
        .iter()
        .map(|term| format!("{}:{}", term.term, term.count))
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn as_u32(value: i32, field: &str) -> Result<u32, StorageError> {
    value
        .try_into()
        .map_err(|_| StorageError::InvalidData(format!("{field} cannot be negative")))
}

pub(super) fn as_u64(value: i64, field: &str) -> Result<u64, StorageError> {
    value
        .try_into()
        .map_err(|_| StorageError::InvalidData(format!("{field} cannot be negative")))
}
