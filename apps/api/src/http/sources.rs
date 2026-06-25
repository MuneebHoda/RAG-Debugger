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
use rag_debugger_rag::{
    chunking::{Chunker, SmartSectionChunker, WhitespaceChunker},
    intelligence::{analyze_document, annotate_chunk_quality},
    ExtractionError, TextExtractor,
};
use rag_debugger_storage::repository::IngestionRepository;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

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

    let extractor = TextExtractor;
    let mut totals = IngestionTotals {
        files_received,
        failed_files: results.len() as u32,
        ..IngestionTotals::default()
    };

    for uploaded_file in uploaded_files {
        match process_file(
            repository.clone(),
            &extractor,
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
    extractor: &TextExtractor,
    source_id: SourceId,
    chunking: ChunkingConfig,
    preview_chunk_limit: usize,
    uploaded_file: UploadedFile,
) -> Result<DocumentIngestResult, FileFailure> {
    let extracted = extractor
        .extract(
            &uploaded_file.file_name,
            uploaded_file.content_type.as_deref(),
            &uploaded_file.bytes,
        )
        .map_err(|error| extraction_failure(&uploaded_file.file_name, error))?;

    if extracted.text.trim().is_empty() {
        return Err(FileFailure {
            file_name: uploaded_file.file_name,
            error_code: "empty_extracted_text",
            message: "no readable text was extracted from the file".to_owned(),
        });
    }

    let intelligence = analyze_document(&uploaded_file.file_name, &extracted.text);
    let document = Document {
        id: DocumentId(Uuid::now_v7()),
        source_id,
        path: uploaded_file.file_name.clone(),
        mime_type: extracted.mime_type,
        checksum: checksum_bytes(&uploaded_file.bytes),
        byte_size: uploaded_file.bytes.len() as u64,
        profile: intelligence.profile,
        extraction_quality: intelligence.extraction_quality,
        warnings: intelligence.warnings,
    };
    let mut chunks =
        chunk_document(&document, &extracted.text, chunking).map_err(|error| FileFailure {
            file_name: uploaded_file.file_name.clone(),
            error_code: "chunking_failed",
            message: error.to_string(),
        })?;
    annotate_chunk_quality(&mut chunks);

    if chunks.is_empty() {
        return Err(FileFailure {
            file_name: uploaded_file.file_name,
            error_code: "empty_chunks",
            message: "text extraction produced no chunks".to_owned(),
        });
    }

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

fn chunk_document(
    document: &Document,
    text: &str,
    chunking: ChunkingConfig,
) -> Result<Vec<rag_debugger_core::Chunk>, rag_debugger_rag::RagError> {
    match chunking.strategy.normalized() {
        ChunkingStrategy::Structured | ChunkingStrategy::SmartSections => {
            SmartSectionChunker.chunk(document, text, chunking)
        }
        ChunkingStrategy::Whitespace => WhitespaceChunker.chunk(document, text, chunking),
    }
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

fn extraction_failure(file_name: &str, error: ExtractionError) -> FileFailure {
    let (error_code, message) = match error {
        ExtractionError::UnsupportedFileType => (
            "unsupported_file_type",
            "supported files are .txt, .md, .markdown, .html, .htm, and .pdf".to_owned(),
        ),
        ExtractionError::InvalidUtf8 => (
            "invalid_utf8",
            "text and markdown files must be valid UTF-8".to_owned(),
        ),
        ExtractionError::Html(message) => ("html_extraction_failed", message),
        ExtractionError::Pdf(message) => ("pdf_extraction_failed", message),
    };

    FileFailure {
        file_name: file_name.to_owned(),
        error_code,
        message,
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

fn checksum_bytes(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
