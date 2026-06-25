import {
  AlertCircle,
  Database,
  Loader2,
  RefreshCw,
  Search,
  SlidersHorizontal,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import {
  getEmbeddingStatus,
  indexEmbeddings,
  type EmbeddingStatus,
} from "../../../lib/api/embeddings";
import {
  createEvalLabCase,
  listEvalLabDatasets,
  type RetrievalEvalCase,
  type RetrievalEvalDatasetSummary,
  type RetrievalEvalRun,
} from "../../../lib/api/evalLab";
import {
  listRetrievalEvalCases,
  queryRetrieval,
  runRetrievalEvals,
  type RetrievalMode,
  type RetrievalQueryResponse,
} from "../../../lib/api/retrieval";
import {
  listSources,
  type DocumentSummary,
  type SourceSummary,
} from "../../../lib/api/sources";
import { createTraceFromRetrievalRun } from "../../../lib/api/traces";
import { AnswerPanel, EvalPanel, HitsPanel } from "./RetrievalResults";
import "./RetrievalPage.module.css";

const DEFAULT_TOP_K = 5;

export function RetrievalPage() {
  const [sources, setSources] = useState<SourceSummary[]>([]);
  const [query, setQuery] = useState("");
  const [topK, setTopK] = useState(DEFAULT_TOP_K);
  const [retrievalMode, setRetrievalMode] = useState<RetrievalMode>("hybrid");
  const [selectedSourceIds, setSelectedSourceIds] = useState<string[]>([]);
  const [selectedDocumentIds, setSelectedDocumentIds] = useState<string[]>([]);
  const [response, setResponse] = useState<RetrievalQueryResponse | null>(null);
  const [embeddingStatus, setEmbeddingStatus] =
    useState<EmbeddingStatus | null>(null);
  const [evalCases, setEvalCases] = useState<RetrievalEvalCase[]>([]);
  const [evalDatasets, setEvalDatasets] = useState<
    RetrievalEvalDatasetSummary[]
  >([]);
  const [evalRun, setEvalRun] = useState<RetrievalEvalRun | null>(null);
  const [isLoadingSources, setIsLoadingSources] = useState(true);
  const [isQuerying, setIsQuerying] = useState(false);
  const [isIndexing, setIsIndexing] = useState(false);
  const [isSavingEval, setIsSavingEval] = useState(false);
  const [isSavingTrace, setIsSavingTrace] = useState(false);
  const [isRunningEvals, setIsRunningEvals] = useState(false);
  const [traceMessage, setTraceMessage] = useState<string | null>(null);
  const [evalMessage, setEvalMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    Promise.all([
      listSources(controller.signal),
      getEmbeddingStatus(controller.signal),
      listRetrievalEvalCases(controller.signal),
      listEvalLabDatasets(controller.signal),
    ])
      .then(
        ([
          nextSources,
          nextEmbeddingStatus,
          nextEvalCases,
          nextEvalDatasets,
        ]) => {
          setSources(nextSources);
          setEmbeddingStatus(nextEmbeddingStatus);
          setEvalCases(nextEvalCases);
          setEvalDatasets(nextEvalDatasets);
        },
      )
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
      setTraceMessage(null);
      setEvalMessage(null);
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

  async function handleSaveEval() {
    if (!response || response.hits.length === 0 || isSavingEval) {
      return;
    }

    setIsSavingEval(true);
    setError(null);

    try {
      const dataset = evalDatasets[0];
      if (!dataset) {
        throw new Error("Create an Eval Lab dataset before saving cases.");
      }

      const topHits = response.hits.slice(0, 3);
      const evalCase = await createEvalLabCase(dataset.id, {
        name: response.run.query,
        query: response.run.query,
        top_k: response.run.top_k,
        expected_chunk_ids: topHits.map((hit) => hit.chunk.id),
        expected_document_ids: uniqueStrings(
          topHits.map((hit) => hit.document.id),
        ),
      });
      setEvalCases((currentCases) => [evalCase, ...currentCases]);
      setEvalMessage(`Saved to ${dataset.name}.`);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setIsSavingEval(false);
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
      setTraceMessage(
        `Saved trace ${trace.id.slice(0, 8)} for debugger review.`,
      );
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setIsSavingTrace(false);
    }
  }

  async function handleRunEvals() {
    if (evalCases.length === 0 || isRunningEvals) {
      return;
    }

    setIsRunningEvals(true);
    setError(null);

    try {
      const result = await runRetrievalEvals({
        retrieval_mode: retrievalMode,
      });
      setEvalRun(result);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setIsRunningEvals(false);
    }
  }

  return (
    <section className="retrieval-page" aria-labelledby="retrieval-title">
      <header className="page-header">
        <div>
          <p className="eyebrow">Evidence diagnosis</p>
          <h1 id="retrieval-title">Retrieval Playground</h1>
          <p>
            Ask questions across corpora, compare lexical/vector/hybrid modes,
            inspect ranking signals, and explain cited evidence.
          </p>
        </div>
      </header>

      {error ? (
        <div className="alert" role="alert">
          <AlertCircle aria-hidden="true" size={18} />
          <span>{error}</span>
        </div>
      ) : null}

      <section className="retrieval-layout">
        <div className="panel retrieval-controls">
          <div className="panel-heading">
            <h2>Query</h2>
            <span className="status-pill">
              {isLoadingSources
                ? "Loading"
                : `${allDocuments.length} documents`}
            </span>
          </div>

          <label className="query-field">
            Question
            <textarea
              value={query}
              onChange={(event) => setQuery(event.currentTarget.value)}
              placeholder="Which chunks explain the policy exception, product behavior, or technical decision?"
            />
          </label>

          <div className="mode-tabs" aria-label="Ranking comparison">
            {(["hybrid", "vector", "lexical"] as RetrievalMode[]).map(
              (mode) => (
                <button
                  className={
                    mode === retrievalMode ? "mode-tab active" : "mode-tab"
                  }
                  key={mode}
                  type="button"
                  onClick={() => setRetrievalMode(mode)}
                >
                  {mode}
                </button>
              ),
            )}
          </div>

          <div className="config-grid">
            <label>
              Top K
              <input
                min={1}
                max={25}
                type="number"
                value={topK}
                onChange={(event) => setTopK(Number(event.currentTarget.value))}
              />
            </label>
            <label>
              Mode
              <select
                value={retrievalMode}
                onChange={(event) =>
                  setRetrievalMode(event.currentTarget.value as RetrievalMode)
                }
              >
                <option value="hybrid">Hybrid</option>
                <option value="vector">Vector</option>
                <option value="lexical">Lexical</option>
              </select>
            </label>
          </div>

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

          <button
            className="primary-button"
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
        </div>

        <div className="retrieval-results">
          <AnswerPanel
            response={response}
            isQuerying={isQuerying}
            onSaveEval={() => void handleSaveEval()}
            onSaveTrace={() => void handleSaveTrace()}
            isSavingEval={isSavingEval}
            isSavingTrace={isSavingTrace}
            traceMessage={traceMessage}
            evalMessage={evalMessage}
          />
          <HitsPanel response={response} isQuerying={isQuerying} />
          <EvalPanel
            cases={evalCases}
            evalRun={evalRun}
            isRunning={isRunningEvals}
            onRun={() => void handleRunEvals()}
          />
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
    <div className="filter-stack">
      <div className="filter-heading">
        <SlidersHorizontal aria-hidden="true" size={16} />
        <strong>Filters</strong>
      </div>

      <div className="checkbox-list" aria-label="Source filters">
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

      <div className="checkbox-list" aria-label="Document filters">
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
    <div className="embedding-status">
      <div>
        <div className="filter-heading">
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

function uniqueStrings(values: string[]) {
  return Array.from(new Set(values));
}

function errorMessage(cause: unknown) {
  return cause instanceof Error ? cause.message : "Unexpected request failure";
}
