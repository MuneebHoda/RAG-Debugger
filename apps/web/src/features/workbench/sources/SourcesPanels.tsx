import { AlertCircle, CheckCircle2, FileText, ShieldAlert } from "lucide-react";
import { Link } from "react-router-dom";

import type {
  ChunkPreview,
  ChunkingStrategy,
  DocumentIngestResult,
  DocumentSummary,
} from "../../../lib/api/sources";
import styles from "./SourcesPage.module.css";

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

export function UploadResults({
  results,
}: {
  results: DocumentIngestResult[];
}) {
  if (results.length === 0) {
    return <p>No ingestion run yet.</p>;
  }

  return (
    <div className={styles.resultList}>
      {results.map((result) => (
        <article className={styles.resultRow} key={result.file_name}>
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

export function DocumentList({ documents }: { documents: DocumentSummary[] }) {
  if (documents.length === 0) {
    return <p>No documents indexed yet.</p>;
  }

  return (
    <div className={styles.documentList}>
      {documents.map(({ document, chunk_count }) => (
        <Link
          className={styles.documentRow}
          to={`/app/sources/${document.id}`}
          key={document.id}
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
        </Link>
      ))}
    </div>
  );
}

export function ChunkList({
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
    <div className={styles.chunkList}>
      {chunks.map((chunk) => (
        <article className={styles.chunkCard} key={chunk.id}>
          <header>
            <strong>Chunk {chunk.ordinal + 1}</strong>
            <span>{chunk.token_count} tokens</span>
          </header>
          <div className={styles.chunkMeta}>
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
