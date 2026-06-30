use std::{path::Path, sync::Arc};

use axum::{
    extract::{Multipart, Path as AxumPath, State},
    http::StatusCode,
    Json,
};
use rag_debugger_core::{
    ChunkPreview, ChunkingConfig, ChunkingStrategy, Document, DocumentId, IngestionRun,
    IngestionRunId, IngestionRunStatus, IngestionTotals, ProductConfig, Source, SourceId,
    SourceKind, SourceSummary, SourceSyncPolicy,
};
use rag_debugger_storage::repository::IngestionRepository;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    error::ApiError,
    ingestion::{prepare_document, DocumentPreparationError, IngestionFile},
    state::AppState,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct IngestFilesResponse {
    pub source: Source,
    pub ingestion_run: IngestionRun,
    pub documents: Vec<DocumentIngestResult>,
    pub totals: IngestionTotals,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentIngestResult {
    pub file_name: String,
    pub status: DocumentIngestStatus,
    pub document: Option<Document>,
    pub chunk_count: u32,
    pub preview_chunks: Vec<ChunkPreview>,
    pub error_code: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DocumentIngestStatus {
    Success,
    Failure,
}

#[derive(Debug)]
struct UploadedFile {
    file_name: String,
    content_type: Option<String>,
    bytes: Vec<u8>,
}

#[derive(Debug)]
struct FileFailure {
    file_name: String,
    error_code: &'static str,
    message: String,
}

pub async fn ingest_files(
    State(state): State<AppState>,
    multipart: Multipart,
) -> Result<(StatusCode, Json<IngestFilesResponse>), ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let product_config = &state.config().product;
    let (uploaded_files, mut results, files_received, chunking) =
        read_multipart(multipart, product_config).await?;
    let project = repository.ensure_default_project().await?;
    let source = create_upload_source(
        repository.clone(),
        project.id,
        chunking,
        product_config.product.workspace_name.as_str(),
    )
    .await?;
    let started_at = OffsetDateTime::now_utc();
    let run = repository
        .create_ingestion_run(IngestionRun {
            id: IngestionRunId(Uuid::now_v7()),
            source_id: source.id,
            status: IngestionRunStatus::Running,
            totals: IngestionTotals {
                files_received,
                ..IngestionTotals::default()
            },
            started_at,
            completed_at: None,
        })
        .await?;

    let mut totals = IngestionTotals {
        files_received,
        failed_files: results.len() as u32,
        ..IngestionTotals::default()
    };

    for uploaded_file in uploaded_files {
        match process_file(
            repository.clone(),
            source.id,
            chunking,
            product_config.ingestion.preview_chunk_limit as usize,
            uploaded_file,
        )
        .await
        {
            Ok(result) => {
                totals.documents_created += 1;
                totals.chunks_created += result.chunk_count;
                results.push(result);
            }
            Err(failure) => {
                totals.failed_files += 1;
                results.push(failure_result(failure));
            }
        }
    }

    let status = match (totals.documents_created, totals.failed_files) {
        (0, _) => IngestionRunStatus::Failed,
        (_, 0) => IngestionRunStatus::Completed,
        _ => IngestionRunStatus::Partial,
    };
    let ingestion_run = repository
        .complete_ingestion_run(run.id, status, totals)
        .await?;

    let response = IngestFilesResponse {
        source,
        ingestion_run,
        documents: results,
        totals,
    };
    let http_status = if totals.documents_created == 0 {
        StatusCode::UNPROCESSABLE_ENTITY
    } else {
        StatusCode::CREATED
    };

    Ok((http_status, Json(response)))
}

pub async fn list_sources(
    State(state): State<AppState>,
) -> Result<Json<Vec<SourceSummary>>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    Ok(Json(repository.list_sources().await?))
}

pub async fn list_document_chunks(
    State(state): State<AppState>,
    AxumPath(document_id): AxumPath<Uuid>,
) -> Result<Json<Vec<ChunkPreview>>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let chunks = repository
        .list_document_chunks(DocumentId(document_id))
        .await?
        .into_iter()
        .map(ChunkPreview::from)
        .collect();
    Ok(Json(chunks))
}

async fn read_multipart(
    mut multipart: Multipart,
    product_config: &ProductConfig,
) -> Result<
    (
        Vec<UploadedFile>,
        Vec<DocumentIngestResult>,
        u32,
        ChunkingConfig,
    ),
    ApiError,
> {
    let mut uploaded_files = Vec::new();
    let mut results = Vec::new();
    let mut total_bytes = 0usize;
    let mut files_received = 0u32;
    let default_chunking = product_config.chunking;
    let mut target_tokens = default_chunking.target_tokens;
    let mut overlap_tokens = default_chunking.overlap_tokens;
    let mut strategy = default_chunking.strategy;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| ApiError::BadRequest(error.to_string()))?
    {
        let Some(name) = field.name().map(ToOwned::to_owned) else {
            continue;
        };

        if name == "target_tokens" {
            target_tokens = read_u32_field(field, "target_tokens").await?;
            continue;
        }

        if name == "overlap_tokens" {
            overlap_tokens = read_u32_field(field, "overlap_tokens").await?;
            continue;
        }

        if name == "chunking_strategy" {
            strategy = read_chunking_strategy_field(field).await?;
            continue;
        }

        if name != "files[]" && name != "files" {
            continue;
        }

        files_received += 1;
        let file_name = clean_file_name(field.file_name().unwrap_or("upload"));
        let content_type = field.content_type().map(ToOwned::to_owned);
        let bytes = field
            .bytes()
            .await
            .map_err(|error| ApiError::BadRequest(error.to_string()))?;

        total_bytes = total_bytes.saturating_add(bytes.len());
        if total_bytes > product_config.ingestion.max_request_bytes as usize {
            return Err(ApiError::BadRequest(format!(
                "multipart request exceeds {} bytes",
                product_config.ingestion.max_request_bytes
            )));
        }

        if files_received > product_config.ingestion.max_files_per_request {
            results.push(failure_result(FileFailure {
                file_name,
                error_code: "too_many_files",
                message: format!(
                    "maximum {} files are allowed per request",
                    product_config.ingestion.max_files_per_request
                ),
            }));
            continue;
        }

        if bytes.len() > product_config.ingestion.max_file_bytes as usize {
            results.push(failure_result(FileFailure {
                file_name,
                error_code: "file_too_large",
                message: format!(
                    "file exceeds {} bytes",
                    product_config.ingestion.max_file_bytes
                ),
            }));
            continue;
        }

        uploaded_files.push(UploadedFile {
            file_name,
            content_type,
            bytes: bytes.to_vec(),
        });
    }

    if files_received == 0 {
        return Err(ApiError::BadRequest(
            "multipart field files[] is required".to_owned(),
        ));
    }

    if target_tokens == 0 {
        return Err(ApiError::BadRequest(
            "target_tokens must be greater than zero".to_owned(),
        ));
    }

    if overlap_tokens >= target_tokens {
        return Err(ApiError::BadRequest(
            "overlap_tokens must be less than target_tokens".to_owned(),
        ));
    }

    Ok((
        uploaded_files,
        results,
        files_received,
        ChunkingConfig {
            target_tokens,
            overlap_tokens,
            strategy,
        },
    ))
}

async fn read_u32_field(
    field: axum::extract::multipart::Field<'_>,
    field_name: &str,
) -> Result<u32, ApiError> {
    let text = field
        .text()
        .await
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    text.parse::<u32>()
        .map_err(|_| ApiError::BadRequest(format!("{field_name} must be an unsigned integer")))
}

async fn read_chunking_strategy_field(
    field: axum::extract::multipart::Field<'_>,
) -> Result<ChunkingStrategy, ApiError> {
    let text = field
        .text()
        .await
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    parse_chunking_strategy(text.trim())
}

fn parse_chunking_strategy(strategy: &str) -> Result<ChunkingStrategy, ApiError> {
    match strategy {
        "structured" | "smart_sections" => Ok(ChunkingStrategy::Structured),
        "whitespace" => Ok(ChunkingStrategy::Whitespace),
        _ => Err(ApiError::BadRequest(
            "chunking_strategy must be structured or whitespace".to_owned(),
        )),
    }
}

async fn create_upload_source(
    repository: Arc<dyn IngestionRepository>,
    project_id: rag_debugger_core::ProjectId,
    chunking: ChunkingConfig,
    workspace_name: &str,
) -> Result<Source, ApiError> {
    let source = Source {
        id: SourceId(Uuid::now_v7()),
        project_id,
        name: format!("{workspace_name} upload {}", OffsetDateTime::now_utc()),
        kind: SourceKind::FileSet {
            root_hint: "browser-upload".to_owned(),
        },
        sync_policy: SourceSyncPolicy::Manual,
        chunking,
    };

    Ok(repository.create_source(source).await?)
}

async fn process_file(
    repository: Arc<dyn IngestionRepository>,
    source_id: SourceId,
    chunking: ChunkingConfig,
    preview_chunk_limit: usize,
    uploaded_file: UploadedFile,
) -> Result<DocumentIngestResult, FileFailure> {
    let prepared = prepare_document(
        source_id,
        DocumentId(Uuid::now_v7()),
        chunking,
        IngestionFile {
            file_name: &uploaded_file.file_name,
            content_type: uploaded_file.content_type.as_deref(),
            bytes: &uploaded_file.bytes,
        },
    )
    .map_err(|error| preparation_failure(&uploaded_file.file_name, error))?;
    let document = prepared.document;
    let chunks = prepared.chunks;

    let preview_chunks = chunks
        .iter()
        .take(preview_chunk_limit)
        .cloned()
        .map(ChunkPreview::from)
        .collect::<Vec<_>>();
    let chunk_count = chunks.len() as u32;
    let document = repository
        .insert_document_with_chunks(document, chunks)
        .await
        .map_err(|error| FileFailure {
            file_name: uploaded_file.file_name.clone(),
            error_code: "storage_failed",
            message: error.to_string(),
        })?;

    Ok(DocumentIngestResult {
        file_name: uploaded_file.file_name,
        status: DocumentIngestStatus::Success,
        document: Some(document),
        chunk_count,
        preview_chunks,
        error_code: None,
        message: None,
    })
}

fn failure_result(failure: FileFailure) -> DocumentIngestResult {
    DocumentIngestResult {
        file_name: failure.file_name,
        status: DocumentIngestStatus::Failure,
        document: None,
        chunk_count: 0,
        preview_chunks: Vec::new(),
        error_code: Some(failure.error_code.to_owned()),
        message: Some(failure.message),
    }
}

fn preparation_failure(file_name: &str, error: DocumentPreparationError) -> FileFailure {
    FileFailure {
        file_name: file_name.to_owned(),
        error_code: error.code,
        message: error.message,
    }
}

fn clean_file_name(file_name: &str) -> String {
    Path::new(file_name)
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .filter(|file_name| !file_name.trim().is_empty())
        .unwrap_or("upload")
        .to_owned()
}
