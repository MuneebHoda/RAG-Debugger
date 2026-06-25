import {
  AlertTriangle,
  Download,
  FileBarChart,
  GitBranch,
  Search,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import {
  getEmbeddingStatus,
  type EmbeddingStatus,
} from "../lib/api/embeddings";
import { listSources, type SourceSummary } from "../lib/api/sources";
import { listTraces, type TraceSummary } from "../lib/api/traces";

export function ReportsPage() {
  const [sources, setSources] = useState<SourceSummary[]>([]);
  const [embeddingStatus, setEmbeddingStatus] =
    useState<EmbeddingStatus | null>(null);
  const [traces, setTraces] = useState<TraceSummary[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    Promise.all([
      listSources(controller.signal),
      getEmbeddingStatus(controller.signal),
      listTraces(controller.signal),
    ])
      .then(([nextSources, nextEmbeddingStatus, nextTraces]) => {
        setSources(nextSources);
        setEmbeddingStatus(nextEmbeddingStatus);
        setTraces(nextTraces);
      })
      .catch((cause: unknown) => {
        if (!controller.signal.aborted) {
          setError(cause instanceof Error ? cause.message : "Request failed");
        }
      });

    return () => controller.abort();
  }, []);

  const documents = useMemo(
    () => sources.flatMap((source) => source.documents),
    [sources],
  );
  const weakDocuments = documents.filter(
    (item) =>
      item.document.extraction_quality === "low" ||
      item.document.warnings.length > 0,
  );

  return (
    <section className="reports-page" aria-labelledby="reports-title">
      <header className="page-header">
        <div>
          <p className="eyebrow">Shareable diagnostics</p>
          <h1 id="reports-title">Reports</h1>
          <p>
            Prepare retrieval-run summaries, failed-query diagnosis, cited
            evidence, and corpus health notes for review.
          </p>
        </div>
      </header>

      {error ? (
        <div className="alert" role="alert">
          <AlertTriangle aria-hidden="true" size={18} />
          <span>{error}</span>
        </div>
      ) : null}

      <section className="two-column-layout">
        <div className="panel report-preview">
          <div className="panel-heading">
            <h2>Corpus Report</h2>
            <button className="secondary-button compact" type="button">
              <Download aria-hidden="true" size={16} />
              Export view
            </button>
          </div>
          <div className="report-block">
            <FileBarChart aria-hidden="true" size={20} />
            <span>
              <strong>{documents.length} documents</strong>
              <small>
                {totalChunks(sources)} chunks available for retrieval
              </small>
            </span>
          </div>
          <div className="report-block">
            <Search aria-hidden="true" size={20} />
            <span>
              <strong>
                {embeddingStatus
                  ? `${embeddingStatus.indexed_chunks}/${embeddingStatus.total_chunks} indexed`
                  : "Embedding status unknown"}
              </strong>
              <small>semantic readiness for vector and hybrid modes</small>
            </span>
          </div>
          <div className="report-block">
            <GitBranch aria-hidden="true" size={20} />
            <span>
              <strong>{traces.length} saved traces</strong>
              <small>
                trace timelines can support failed-query diagnosis and evidence
                review
              </small>
            </span>
          </div>
        </div>

        <div className="panel">
          <div className="panel-heading">
            <h2>Evidence Issues</h2>
            <span className="status-pill">{weakDocuments.length} flagged</span>
          </div>
          <div className="table-list">
            {weakDocuments.length === 0 ? (
              <p>No extraction warnings are currently reported.</p>
            ) : (
              weakDocuments.map((item) => (
                <article className="table-row" key={item.document.id}>
                  <strong>{item.document.path}</strong>
                  <span>{item.document.profile.replaceAll("_", " ")}</span>
                  <small>
                    {item.document.extraction_quality} extraction ·{" "}
                    {item.document.warnings.length} warnings
                  </small>
                </article>
              ))
            )}
          </div>
        </div>
      </section>
    </section>
  );
}

function totalChunks(sources: SourceSummary[]) {
  return sources.reduce((total, source) => total + source.chunk_count, 0);
}
