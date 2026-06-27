import {
  AlertCircle,
  Database,
  Loader2,
  RefreshCw,
  Search,
  SlidersHorizontal,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";

import {
  getEmbeddingStatus,
  indexEmbeddings,
  type EmbeddingStatus,
} from "../../../lib/api/embeddings";
import {
  queryRetrieval,
  type RetrievalMode,
  type RetrievalQueryResponse,
} from "../../../lib/api/retrieval";
import {
  listSources,
  type DocumentSummary,
  type SourceSummary,
} from "../../../lib/api/sources";
import { createTraceFromRetrievalRun } from "../../../lib/api/traces";
import { AnswerPanel, HitsPanel } from "./RetrievalResults";
import styles from "./RetrievalPage.module.css";

const DEFAULT_TOP_K = 5;

export function RetrievalPage() {
  const navigate = useNavigate();
  const [sources, setSources] = useState<SourceSummary[]>([]);
  const [query, setQuery] = useState("");
  const [topK, setTopK] = useState(DEFAULT_TOP_K);
  const [retrievalMode, setRetrievalMode] = useState<RetrievalMode>("hybrid");
  const [selectedSourceIds, setSelectedSourceIds] = useState<string[]>([]);
  const [selectedDocumentIds, setSelectedDocumentIds] = useState<string[]>([]);
  const [response, setResponse] = useState<RetrievalQueryResponse | null>(null);
  const [embeddingStatus, setEmbeddingStatus] =
    useState<EmbeddingStatus | null>(null);
  const [isLoadingSources, setIsLoadingSources] = useState(true);
  const [isQuerying, setIsQuerying] = useState(false);
  const [isIndexing, setIsIndexing] = useState(false);
  const [isSavingTrace, setIsSavingTrace] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    Promise.all([
      listSources(controller.signal),
      getEmbeddingStatus(controller.signal),
    ])
      .then(([nextSources, nextEmbeddingStatus]) => {
        setSources(nextSources);
        setEmbeddingStatus(nextEmbeddingStatus);
      })
      .catch((cause: unknown) => {
        if (!controller.signal.aborted) {
          setError(errorMessage(cause));
        }
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setIsLoadingSources(false);
        }
      });

    return () => controller.abort();
  }, []);

  const allDocuments = useMemo(
    () => sources.flatMap((source) => source.documents),
    [sources],
  );
  const visibleDocuments = useMemo(
    () =>
      selectedSourceIds.length === 0
        ? allDocuments
        : allDocuments.filter((document) =>
            selectedSourceIds.includes(document.document.source_id),
          ),
    [allDocuments, selectedSourceIds],
  );
  const activeSelectedDocumentIds = useMemo(
    () =>
      selectedDocumentIds.filter((documentId) =>
        visibleDocuments.some(
          (document) => document.document.id === documentId,
        ),
      ),
    [selectedDocumentIds, visibleDocuments],
  );

  async function handleSubmit() {
    if (query.trim().length === 0 || isQuerying) {
      return;
    }

    setIsQuerying(true);
    setError(null);

    try {
      const nextResponse = await queryRetrieval({
        query: query.trim(),
        top_k: topK,
        retrieval_mode: retrievalMode,
        source_ids: selectedSourceIds,
        document_ids: activeSelectedDocumentIds,
      });
      setResponse(nextResponse);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setIsQuerying(false);
    }
  }

  async function handleIndexEmbeddings() {
    if (isIndexing) {
      return;
    }

    setIsIndexing(true);
    setError(null);

    try {
      const result = await indexEmbeddings();
      setEmbeddingStatus(result.status);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setIsIndexing(false);
    }
  }

  async function handleSaveTrace() {
    if (!response || isSavingTrace) {
      return;
    }

    setIsSavingTrace(true);
    setError(null);

    try {
      const trace = await createTraceFromRetrievalRun({
        run_id: response.run.id,
      });
      navigate(`/app/traces/${trace.id}`);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setIsSavingTrace(false);
    }
  }

  return (
    <section className={styles.page} aria-labelledby="retrieval-title">
      <header className={styles.pageHeader}>
        <div>
          <p className={styles.eyebrow}>Build</p>
          <h1 id="retrieval-title">Test retrieval</h1>
          <p>Ask one question and inspect the evidence CorpusLab would use.</p>
        </div>
      </header>

      {error ? (
        <div className="alert" role="alert">
          <AlertCircle aria-hidden="true" size={18} />
          <span>{error}</span>
        </div>
      ) : null}

      <section className={styles.layout}>
        <div className={`panel ${styles.controls}`}>
          <div className="panel-heading">
            <h2>Question</h2>
            <span className="status-pill">
              {isLoadingSources
                ? "Loading"
                : `${allDocuments.length} documents`}
            </span>
          </div>

          <label className={styles.queryField}>
            What should the corpus answer?
            <textarea
              value={query}
              onChange={(event) => setQuery(event.currentTarget.value)}
              placeholder="Which chunks explain the policy exception, product behavior, or technical decision?"
            />
          </label>

          <div className={styles.modeTabs} aria-label="Retrieval mode">
            {(["hybrid", "vector", "lexical"] as RetrievalMode[]).map(
              (mode) => (
                <button
                  className={
                    mode === retrievalMode
                      ? styles.activeModeTab
                      : styles.modeTab
                  }
                  key={mode}
                  type="button"
                  aria-pressed={mode === retrievalMode}
                  onClick={() => setRetrievalMode(mode)}
                >
                  {mode}
                </button>
              ),
            )}
          </div>

          <button
            className={`primary-button ${styles.primaryAction}`}
            disabled={query.trim().length === 0 || isQuerying || topK <= 0}
            type="button"
            onClick={() => void handleSubmit()}
          >
            {isQuerying ? (
              <Loader2 aria-hidden="true" className="spin" size={18} />
            ) : (
              <Search aria-hidden="true" size={18} />
            )}
            Run retrieval
          </button>

          <details className={styles.advanced}>
            <summary>
              <SlidersHorizontal aria-hidden="true" size={16} /> Advanced
            </summary>
            <div className={styles.advancedContent}>
              <label className={styles.topKField}>
                Results to return
                <input
                  min={1}
                  max={25}
                  type="number"
                  value={topK}
                  onChange={(event) =>
                    setTopK(Number(event.currentTarget.value))
                  }
                />
              </label>
              <EmbeddingPanel
                status={embeddingStatus}
                isIndexing={isIndexing}
                onIndex={() => void handleIndexEmbeddings()}
              />
              <FilterSection
                documents={visibleDocuments}
                selectedDocumentIds={activeSelectedDocumentIds}
                selectedSourceIds={selectedSourceIds}
                sources={sources}
                onToggleDocument={(documentId) =>
                  setSelectedDocumentIds((currentIds) =>
                    toggleId(currentIds, documentId),
                  )
                }
                onToggleSource={(sourceId) =>
                  setSelectedSourceIds((currentIds) =>
                    toggleId(currentIds, sourceId),
                  )
                }
              />
            </div>
          </details>
        </div>

        <div className={styles.results}>
          <AnswerPanel
            response={response}
            isQuerying={isQuerying}
            onSaveTrace={() => void handleSaveTrace()}
            isSavingTrace={isSavingTrace}
          />
          <HitsPanel response={response} isQuerying={isQuerying} />
        </div>
      </section>
    </section>
  );
}

function FilterSection({
  documents,
  selectedDocumentIds,
  selectedSourceIds,
  sources,
  onToggleDocument,
  onToggleSource,
}: {
  documents: DocumentSummary[];
  selectedDocumentIds: string[];
  selectedSourceIds: string[];
  sources: SourceSummary[];
  onToggleDocument: (documentId: string) => void;
  onToggleSource: (sourceId: string) => void;
}) {
  return (
    <div className={styles.filterStack}>
      <div className={styles.filterHeading}>
        <SlidersHorizontal aria-hidden="true" size={16} />
        <strong>Filters</strong>
      </div>

      <div className={styles.checkboxList} aria-label="Source filters">
        {sources.length === 0 ? (
          <span>No sources indexed yet.</span>
        ) : (
          sources.map((source) => (
            <label key={source.source.id}>
              <input
                checked={selectedSourceIds.includes(source.source.id)}
                type="checkbox"
                onChange={() => onToggleSource(source.source.id)}
              />
              <span>{source.source.name}</span>
            </label>
          ))
        )}
      </div>

      <div className={styles.checkboxList} aria-label="Document filters">
        {documents.length === 0 ? (
          <span>No documents available for the selected source filter.</span>
        ) : (
          documents.map(({ document, chunk_count }) => (
            <label key={document.id}>
              <input
                checked={selectedDocumentIds.includes(document.id)}
                type="checkbox"
                onChange={() => onToggleDocument(document.id)}
              />
              <span>
                {document.path} · {chunk_count} chunks
              </span>
            </label>
          ))
        )}
      </div>
    </div>
  );
}

function EmbeddingPanel({
  status,
  isIndexing,
  onIndex,
}: {
  status: EmbeddingStatus | null;
  isIndexing: boolean;
  onIndex: () => void;
}) {
  const readiness =
    status === null
      ? "Unknown"
      : status.total_chunks === 0
        ? "No chunks"
        : status.missing_chunks === 0 && status.stale_chunks === 0
          ? "Ready"
          : "Needs index";

  return (
    <div className={styles.embeddingStatus}>
      <div>
        <div className={styles.filterHeading}>
          <Database aria-hidden="true" size={16} />
          <strong>Embeddings</strong>
        </div>
        <small>
          {status
            ? `${status.indexed_chunks}/${status.total_chunks} indexed · ${status.model.model_name}`
            : "Status unavailable"}
        </small>
      </div>
      <span className="status-pill">{readiness}</span>
      <button
        className="secondary-button"
        disabled={isIndexing || status?.total_chunks === 0}
        type="button"
        onClick={onIndex}
      >
        {isIndexing ? (
          <Loader2 aria-hidden="true" className="spin" size={16} />
        ) : (
          <RefreshCw aria-hidden="true" size={16} />
        )}
        Index
      </button>
    </div>
  );
}

function toggleId(currentIds: string[], id: string) {
  return currentIds.includes(id)
    ? currentIds.filter((currentId) => currentId !== id)
    : [...currentIds, id];
}

function errorMessage(cause: unknown) {
  return cause instanceof Error ? cause.message : "Unexpected request failure";
}
