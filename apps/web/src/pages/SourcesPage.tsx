import {
  AlertCircle,
  CheckCircle2,
  FileText,
  Loader2,
  ShieldAlert,
  UploadCloud,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";

import {
  getProductConfig,
  ingestFiles,
  listDocumentChunks,
  listSources,
  type ChunkPreview,
  type ChunkingStrategy,
  type DocumentIngestResult,
  type DocumentSummary,
  type IngestFilesResponse,
  type ProductConfig,
  type SourceSummary,
} from "../lib/apiClient";

const DEFAULT_TARGET_TOKENS = 512;
const DEFAULT_OVERLAP_TOKENS = 64;
const DEFAULT_CHUNKING_STRATEGY: ChunkingStrategy = "structured";

const CHUNKING_STRATEGY_LABELS: Record<ChunkingStrategy, string> = {
  structured: "Structured document",
  smart_sections: "Smart sections",
  whitespace: "Whitespace",
};

const SPLIT_REASON_LABELS: Record<ChunkPreview["split_reason"], string> = {
  section_boundary: "Section boundary",
  token_limit: "Token limit",
  document_end: "Document end",
  fallback_whitespace: "Whitespace fallback",
};

const CHUNK_QUALITY_LABELS: Record<string, string> = {
  heading_only: "Heading only",
  too_short: "Too short",
  too_long: "Too long",
  duplicate: "Duplicate",
  low_text_density: "Low density",
  extraction_warning: "Extraction warning",
  good_evidence_candidate: "Evidence candidate",
};

export function SourcesPage() {
  const [files, setFiles] = useState<File[]>([]);
  const [targetTokens, setTargetTokens] = useState(DEFAULT_TARGET_TOKENS);
  const [overlapTokens, setOverlapTokens] = useState(DEFAULT_OVERLAP_TOKENS);
  const [chunkingStrategy, setChunkingStrategy] = useState<ChunkingStrategy>(
    DEFAULT_CHUNKING_STRATEGY,
  );
  const [productConfig, setProductConfig] = useState<ProductConfig | null>(
    null,
  );
  const [sources, setSources] = useState<SourceSummary[]>([]);
  const [selectedDocumentId, setSelectedDocumentId] = useState<string | null>(
    null,
  );
  const [chunks, setChunks] = useState<ChunkPreview[]>([]);
  const [uploadResponse, setUploadResponse] =
    useState<IngestFilesResponse | null>(null);
  const [isLoadingSources, setIsLoadingSources] = useState(true);
  const [isLoadingChunks, setIsLoadingChunks] = useState(false);
  const [isUploading, setIsUploading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const selectDocument = useCallback((documentId: string | null) => {
    setSelectedDocumentId(documentId);
    setChunks([]);

    if (!documentId) {
      setIsLoadingChunks(false);
      return;
    }

    setIsLoadingChunks(true);
    listDocumentChunks(documentId)
      .then(setChunks)
      .catch((cause: unknown) => setError(errorMessage(cause)))
      .finally(() => setIsLoadingChunks(false));
  }, []);

  const refreshSources = useCallback(
    async (signal?: AbortSignal, preferredDocumentId?: string | null) => {
      setIsLoadingSources(true);
      setError(null);

      try {
        const nextSources = await listSources(signal);
        setSources(nextSources);
        const nextSelectedDocumentId =
          preferredDocumentId && hasDocument(nextSources, preferredDocumentId)
            ? preferredDocumentId
            : (nextSources[0]?.documents[0]?.document.id ?? null);

        selectDocument(nextSelectedDocumentId);
      } catch (cause) {
        if (!signal?.aborted) {
          setError(errorMessage(cause));
        }
      } finally {
        if (!signal?.aborted) {
          setIsLoadingSources(false);
        }
      }
    },
    [selectDocument],
  );

  useEffect(() => {
    const controller = new AbortController();
    getProductConfig(controller.signal)
      .then((config) => {
        setProductConfig(config);
        if (config.chunking) {
          setTargetTokens(config.chunking.target_tokens);
          setOverlapTokens(config.chunking.overlap_tokens);
          setChunkingStrategy(config.chunking.strategy);
        }
      })
      .catch(() => undefined)
      .finally(() => {
        if (!controller.signal.aborted) {
          void refreshSources(controller.signal);
        }
      });
    return () => controller.abort();
  }, [refreshSources]);

  const documents = useMemo(
    () => sources.flatMap((source) => source.documents),
    [sources],
  );
  const selectedDocument = documents.find(
    (document) => document.document.id === selectedDocumentId,
  );

  async function handleUpload() {
    if (files.length === 0 || isUploading) {
      return;
    }

    setIsUploading(true);
    setError(null);

    try {
      const response = await ingestFiles(files, {
        targetTokens,
        overlapTokens,
        strategy: chunkingStrategy,
      });
      setUploadResponse(response);

      const firstDocumentId = response.documents.find(
        (document) => document.status === "success",
      )?.document?.id;

      await refreshSources(undefined, firstDocumentId ?? null);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setIsUploading(false);
    }
  }

  return (
    <section className="sources-page" aria-labelledby="sources-title">
      <header className="page-header">
        <div>
          <p className="eyebrow">Corpus ingestion</p>
          <h1 id="sources-title">Sources</h1>
          <p>
            Upload corpus documents, extract readable text, detect document
            profile, persist chunks, and inspect retrieval-ready evidence units.
          </p>
        </div>
      </header>

      {error ? (
        <div className="alert" role="alert">
          <AlertCircle aria-hidden="true" size={18} />
          <span>{error}</span>
        </div>
      ) : null}

      <section className="sources-layout">
        <div className="panel upload-panel">
          <div className="panel-heading">
            <h2>Upload Files</h2>
            <span className="status-pill">
              {productConfig
                ? productConfig.ingestion?.supported_extensions?.join(", ")
                : "PDF, HTML, MD, TXT"}
            </span>
          </div>

          <label
            className="upload-zone"
            htmlFor="source-files"
            onDragOver={(event) => event.preventDefault()}
            onDrop={(event) => {
              event.preventDefault();
              setFiles(Array.from(event.dataTransfer.files));
            }}
          >
            <UploadCloud aria-hidden="true" size={28} />
            <strong>Choose files</strong>
            <span>
              Drop policies, docs, papers, specs, or knowledge-base files.
            </span>
            <input
              id="source-files"
              aria-label="Choose files"
              multiple
              type="file"
              accept=".txt,.md,.markdown,.html,.htm,.pdf,text/plain,text/markdown,text/html,application/pdf"
              onChange={(event) =>
                setFiles(Array.from(event.currentTarget.files ?? []))
              }
            />
          </label>

          <div className="selected-files" aria-label="Selected files">
            {files.length === 0 ? (
              <span>No files selected</span>
            ) : (
              files.map((file) => (
                <span key={`${file.name}-${file.size}`}>
                  {file.name} · {formatBytes(file.size)}
                </span>
              ))
            )}
          </div>

          <div className="config-grid">
            <label>
              Strategy
              <select
                aria-label="Chunking strategy"
                value={chunkingStrategy}
                onChange={(event) =>
                  setChunkingStrategy(
                    event.currentTarget.value as ChunkingStrategy,
                  )
                }
              >
                <option value="structured">Structured document</option>
                <option value="whitespace">Whitespace</option>
              </select>
            </label>
            <label>
              Target tokens
              <input
                min={1}
                type="number"
                value={targetTokens}
                onChange={(event) =>
                  setTargetTokens(Number(event.currentTarget.value))
                }
              />
            </label>
            <label>
              Overlap tokens
              <input
                min={0}
                type="number"
                value={overlapTokens}
                onChange={(event) =>
                  setOverlapTokens(Number(event.currentTarget.value))
                }
              />
            </label>
          </div>

          <button
            className="primary-button"
            disabled={
              files.length === 0 ||
              isUploading ||
              targetTokens <= 0 ||
              overlapTokens >= targetTokens
            }
            type="button"
            onClick={() => void handleUpload()}
          >
            {isUploading ? (
              <Loader2 aria-hidden="true" className="spin" size={18} />
            ) : (
              <UploadCloud aria-hidden="true" size={18} />
            )}
            Ingest files
          </button>
        </div>

        <div className="panel results-panel">
          <div className="panel-heading">
            <h2>Last Run</h2>
            {uploadResponse ? (
              <span className="status-pill">
                {uploadResponse.totals.documents_created} documents
              </span>
            ) : null}
          </div>
          <UploadResults results={uploadResponse?.documents ?? []} />
        </div>
      </section>

      <section className="sources-layout wide">
        <div className="panel document-panel">
          <div className="panel-heading">
            <h2>Documents</h2>
            <span className="status-pill">
              {isLoadingSources ? "Loading" : `${documents.length} indexed`}
            </span>
          </div>
          <DocumentList
            documents={documents}
            selectedDocumentId={selectedDocumentId}
            onSelect={selectDocument}
          />
        </div>

        <div className="panel chunk-panel">
          <div className="panel-heading">
            <h2>Chunk Preview</h2>
            {selectedDocument ? (
              <span className="status-pill">
                {selectedDocument.chunk_count} chunks
              </span>
            ) : null}
          </div>
          <ChunkList
            chunks={selectedDocumentId ? chunks : []}
            isLoading={isLoadingChunks}
          />
        </div>
      </section>
    </section>
  );
}

function UploadResults({ results }: { results: DocumentIngestResult[] }) {
  if (results.length === 0) {
    return <p>No ingestion run yet.</p>;
  }

  return (
    <div className="result-list">
      {results.map((result) => (
        <article className="result-row" key={result.file_name}>
          {result.status === "success" ? (
            <CheckCircle2 aria-hidden="true" size={18} />
          ) : (
            <AlertCircle aria-hidden="true" size={18} />
          )}
          <span>
            <strong>{result.file_name}</strong>
            <small>
              {result.status === "success"
                ? `${result.chunk_count} chunks created`
                : result.message}
            </small>
          </span>
        </article>
      ))}
    </div>
  );
}

function DocumentList({
  documents,
  selectedDocumentId,
  onSelect,
}: {
  documents: DocumentSummary[];
  selectedDocumentId: string | null;
  onSelect: (documentId: string) => void;
}) {
  if (documents.length === 0) {
    return <p>No documents indexed yet.</p>;
  }

  return (
    <div className="document-list">
      {documents.map(({ document, chunk_count }) => (
        <button
          className={
            document.id === selectedDocumentId
              ? "document-row selected"
              : "document-row"
          }
          key={document.id}
          type="button"
          onClick={() => onSelect(document.id)}
        >
          <FileText aria-hidden="true" size={18} />
          <span>
            <strong>{document.path}</strong>
            <small>
              {prettyLabel(document.profile ?? "general")} ·{" "}
              {document.extraction_quality ?? "unknown"} · {chunk_count} chunks
              · {formatBytes(document.byte_size)}
            </small>
          </span>
          {(document.warnings ?? []).length > 0 ? (
            <span className="row-badge warning">
              {(document.warnings ?? []).length} warnings
            </span>
          ) : (
            <span className="row-badge">ready</span>
          )}
        </button>
      ))}
    </div>
  );
}

function ChunkList({
  chunks,
  isLoading,
}: {
  chunks: ChunkPreview[];
  isLoading: boolean;
}) {
  if (isLoading) {
    return <p>Loading chunks...</p>;
  }

  if (chunks.length === 0) {
    return <p>Select a document to inspect its chunks.</p>;
  }

  return (
    <div className="chunk-list">
      {chunks.map((chunk) => (
        <article className="chunk-card" key={chunk.id}>
          <header>
            <strong>Chunk {chunk.ordinal + 1}</strong>
            <span>{chunk.token_count} tokens</span>
          </header>
          <div className="chunk-meta">
            <span>{CHUNKING_STRATEGY_LABELS[chunk.strategy]}</span>
            <span>{chunk.section_title ?? "Unsectioned"}</span>
            <span>{SPLIT_REASON_LABELS[chunk.split_reason]}</span>
            <span>{Math.round(chunk.text_density * 100)}% density</span>
            <span>{Math.round(chunk.evidence_score_hint * 100)} evidence</span>
          </div>
          {(chunk.quality_flags ?? []).length > 0 ? (
            <div className="quality-badges" aria-label="Chunk quality flags">
              {(chunk.quality_flags ?? []).map((flag) => (
                <span
                  className={
                    flag === "good_evidence_candidate"
                      ? "quality-badge good"
                      : "quality-badge warning"
                  }
                  key={flag}
                >
                  {flag === "good_evidence_candidate" ? null : (
                    <ShieldAlert aria-hidden="true" size={13} />
                  )}
                  {CHUNK_QUALITY_LABELS[flag] ?? prettyLabel(flag)}
                </span>
              ))}
            </div>
          ) : null}
          <p>{chunk.text}</p>
          <small>
            bytes {chunk.byte_range.start}-{chunk.byte_range.end} ·{" "}
            {chunk.checksum.slice(0, 12)}
          </small>
        </article>
      ))}
    </div>
  );
}

function hasDocument(sources: SourceSummary[], documentId: string) {
  return sources.some((source) =>
    source.documents.some((document) => document.document.id === documentId),
  );
}

function errorMessage(cause: unknown) {
  return cause instanceof Error ? cause.message : "Unexpected request failure";
}

function formatBytes(bytes: number) {
  if (bytes < 1024) {
    return `${bytes} B`;
  }

  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }

  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

function prettyLabel(value: string) {
  return value.replaceAll("_", " ");
}
