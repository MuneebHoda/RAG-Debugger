import { AlertTriangle, ArrowLeft, FileText } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { Link, useParams } from "react-router-dom";

import { listDocumentChunks, listSources } from "../../../lib/api/sources";
import { ChunkList } from "./SourcesPanels";
import styles from "./DocumentDetailPage.module.css";

export function DocumentDetailPage() {
  const { documentId } = useParams<{ documentId: string }>();
  const sourcesQuery = useQuery({
    queryKey: ["sources"],
    queryFn: ({ signal }) => listSources(signal),
  });
  const chunksQuery = useQuery({
    queryKey: ["document-chunks", documentId],
    queryFn: ({ signal }) => listDocumentChunks(documentId!, signal),
    enabled: Boolean(documentId),
  });
  const documentSummary = sourcesQuery.data
    ?.flatMap((source) => source.documents)
    .find((entry) => entry.document.id === documentId);

  if (sourcesQuery.isLoading || chunksQuery.isLoading) {
    return <div className={styles.loading}>Loading document…</div>;
  }

  if (sourcesQuery.isError || chunksQuery.isError || !documentSummary) {
    return (
      <section className={styles.error} role="alert">
        <AlertTriangle aria-hidden="true" size={24} />
        <strong>This document could not be opened</strong>
        <button
          type="button"
          onClick={() => {
            void sourcesQuery.refetch();
            void chunksQuery.refetch();
          }}
        >
          Retry
        </button>
        <Link className={styles.backLink} to="/app/sources">
          Back to Corpus
        </Link>
      </section>
    );
  }

  const { document, chunk_count: chunkCount } = documentSummary;
  return (
    <section className={styles.page} aria-labelledby="document-title">
      <Link className={styles.backLink} to="/app/sources">
        <ArrowLeft aria-hidden="true" size={15} /> Back to Corpus
      </Link>

      <header className={styles.header}>
        <div>
          <h1 id="document-title">{document.path}</h1>
          <p>Extraction and chunk evidence for this document.</p>
        </div>
        <div className={styles.badges}>
          <span className={styles.badge}>{prettyLabel(document.profile)}</span>
          <span className={styles.badge}>
            {document.extraction_quality} extraction
          </span>
          {(document.warnings ?? []).length > 0 ? (
            <span className={styles.warning}>
              {(document.warnings ?? []).length} warnings
            </span>
          ) : null}
        </div>
      </header>

      <section className={styles.summary} aria-label="Document summary">
        <Metric label="Type" value={document.mime_type ?? "Unknown"} />
        <Metric label="Size" value={formatBytes(document.byte_size)} />
        <Metric label="Chunks" value={String(chunkCount)} />
        <Metric label="Checksum" value={document.checksum.slice(0, 12)} />
      </section>

      <section className={styles.panel}>
        <div className={styles.panelHeading}>
          <h2>
            <FileText aria-hidden="true" size={17} /> Chunks
          </h2>
          <span className={styles.badge}>{chunkCount}</span>
        </div>
        <ChunkList chunks={chunksQuery.data ?? []} isLoading={false} />
      </section>
    </section>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className={styles.metric}>
      <small>{label}</small>
      <strong>{value}</strong>
    </div>
  );
}

function prettyLabel(value: string) {
  return value.replaceAll("_", " ");
}

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}
