import {
  AlertCircle,
  CheckCircle2,
  Database,
  FileBarChart,
  FileSearch,
  FlaskConical,
  GitBranch,
  Loader2,
  RefreshCw,
  Save,
  Search,
  SlidersHorizontal,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import {
  createEvalLabCase,
  createTraceFromRetrievalRun,
  getEmbeddingStatus,
  indexEmbeddings,
  listEvalLabDatasets,
  listSources,
  listRetrievalEvalCases,
  queryRetrieval,
  runRetrievalEvals,
  type DocumentSummary,
  type EmbeddingStatus,
  type RetrievalEvalCase,
  type RetrievalEvalDatasetSummary,
  type RetrievalEvalRun,
  type RetrievalMode,
  type RetrievalQueryHit,
  type RetrievalQueryResponse,
  type SourceSummary,
} from "../lib/apiClient";

const DEFAULT_TOP_K = 5;

const EVIDENCE_LABELS = {
  strong: "Strong",
  medium: "Medium",
  weak: "Weak",
};

const RETRIEVAL_QUALITY_LABELS: Record<string, string> = {
  duplicate: "Duplicate merged",
  heading_only: "Heading only",
  too_short: "Too short",
  weak_evidence: "Weak evidence",
  semantic_match: "Semantic match",
  exact_term_match: "Exact term",
  section_only_match: "Section-only",
};

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

function AnswerPanel({
  response,
  isQuerying,
  isSavingEval,
  isSavingTrace,
  onSaveEval,
  onSaveTrace,
  traceMessage,
  evalMessage,
}: {
  response: RetrievalQueryResponse | null;
  isQuerying: boolean;
  isSavingEval: boolean;
  isSavingTrace: boolean;
  onSaveEval: () => void;
  onSaveTrace: () => void;
  traceMessage: string | null;
  evalMessage: string | null;
}) {
  return (
    <div className="panel answer-panel">
      <div className="panel-heading">
        <h2>Evidence Summary</h2>
        <div className="panel-actions">
          {response ? (
            <>
              <span className="status-pill">{response.run.retrieval_mode}</span>
              <span className="status-pill">{response.run.latency_ms} ms</span>
            </>
          ) : null}
          <button
            className="secondary-button compact"
            disabled={!response || isSavingTrace}
            title="Save this run as a trace"
            type="button"
            onClick={onSaveTrace}
          >
            {isSavingTrace ? (
              <Loader2 aria-hidden="true" className="spin" size={16} />
            ) : (
              <GitBranch aria-hidden="true" size={16} />
            )}
            Save trace
          </button>
          <button
            className="icon-button"
            title="Create report from this run"
            type="button"
            disabled={!response}
          >
            <FileBarChart aria-hidden="true" size={16} />
          </button>
          <button
            className="icon-button"
            disabled={!response || response.hits.length === 0 || isSavingEval}
            title="Save top hits as an eval case"
            type="button"
            onClick={onSaveEval}
          >
            {isSavingEval ? (
              <Loader2 aria-hidden="true" className="spin" size={16} />
            ) : (
              <Save aria-hidden="true" size={16} />
            )}
          </button>
        </div>
      </div>

      {isQuerying ? (
        <p>Retrieving local evidence...</p>
      ) : response ? (
        <>
          <EmbeddingQueryStatus response={response} />
          <p className="answer-text">{response.answer.text}</p>
          {response.answer.citations.length > 0 ? (
            <div className="citation-list">
              {response.answer.citations.map((citation) => (
                <span key={`${citation.label}-${citation.chunk_id}`}>
                  {citation.label} {citation.document_path} · chunk{" "}
                  {citation.chunk_ordinal + 1}
                </span>
              ))}
            </div>
          ) : null}
          {traceMessage ? (
            <div className="query-status-row">
              <span>{traceMessage}</span>
            </div>
          ) : null}
          {evalMessage ? (
            <div className="query-status-row">
              <span>{evalMessage}</span>
            </div>
          ) : null}
        </>
      ) : (
        <p>Run a query to see the strongest local evidence.</p>
      )}
    </div>
  );
}

function EmbeddingQueryStatus({
  response,
}: {
  response: RetrievalQueryResponse;
}) {
  if (!response.embedding_status.required) {
    return null;
  }

  return (
    <div className="query-status-row">
      <span>
        embeddings {response.embedding_status.readiness} ·{" "}
        {response.embedding_status.indexed_chunks}/
        {response.embedding_status.total_chunks} indexed
      </span>
      {response.embedding_status.stale_chunks > 0 ? (
        <span>{response.embedding_status.stale_chunks} stale</span>
      ) : null}
    </div>
  );
}

function HitsPanel({
  response,
  isQuerying,
}: {
  response: RetrievalQueryResponse | null;
  isQuerying: boolean;
}) {
  return (
    <div className="panel hits-panel">
      <div className="panel-heading">
        <h2>Ranked Evidence</h2>
        {response ? (
          <span className="status-pill">{response.hits.length} hits</span>
        ) : null}
      </div>

      {isQuerying ? (
        <p>Scoring chunks...</p>
      ) : response && response.hits.length > 0 ? (
        <GroupedHitList hits={response.hits} />
      ) : response ? (
        <p>No chunks matched this query.</p>
      ) : (
        <p>Matching chunks will appear here with score breakdowns.</p>
      )}
    </div>
  );
}

function GroupedHitList({ hits }: { hits: RetrievalQueryHit[] }) {
  const groups = hits.reduce<Record<string, RetrievalQueryHit[]>>(
    (acc, hit) => {
      acc[hit.document.id] = [...(acc[hit.document.id] ?? []), hit];
      return acc;
    },
    {},
  );

  return (
    <div className="hit-list">
      {Object.entries(groups).map(([documentId, documentHits]) => (
        <section className="hit-group" key={documentId}>
          <header>
            <strong>{documentHits[0].document.path}</strong>
            <span>{documentHits.length} hits</span>
          </header>
          {documentHits.map((hit) => (
            <HitCard hit={hit} key={hit.chunk.id} />
          ))}
        </section>
      ))}
    </div>
  );
}

function HitCard({ hit }: { hit: RetrievalQueryHit }) {
  return (
    <article className="hit-card">
      <header>
        <strong>
          {hit.citation.label} Rank {hit.rank}
        </strong>
        <span className={`strength-pill ${hit.evidence_strength ?? "medium"}`}>
          {EVIDENCE_LABELS[hit.evidence_strength ?? "medium"]} ·{" "}
          {formatScore(hit.score)}
        </span>
      </header>

      <div className="hit-source">
        <FileSearch aria-hidden="true" size={16} />
        <span>
          {hit.document.path} · chunk {hit.chunk.ordinal + 1}
          {hit.chunk.section_title ? ` · ${hit.chunk.section_title}` : ""}
        </span>
      </div>

      <p>{hit.snippet}</p>

      <div className="term-row">
        {hit.matched_terms.length > 0 ? (
          hit.matched_terms.map((term) => (
            <span key={term.term}>
              {term.term} × {term.count}
            </span>
          ))
        ) : (
          <span>semantic match</span>
        )}
      </div>

      <div className="quality-badges">
        {(hit.quality_flags ?? []).map((flag) => (
          <span className="quality-badge" key={flag}>
            {RETRIEVAL_QUALITY_LABELS[flag] ?? flag}
          </span>
        ))}
        {(hit.duplicate_count ?? 1) > 1 ? (
          <span className="quality-badge warning">
            {hit.duplicate_count} duplicates
          </span>
        ) : null}
      </div>

      <ScoreBars hit={hit} />

      <small>checksum {hit.chunk.checksum.slice(0, 12)}</small>
    </article>
  );
}

function ScoreBars({ hit }: { hit: RetrievalQueryHit }) {
  const normalized =
    hit.normalized_score_breakdown ?? normalizeBreakdown(hit.score_breakdown);
  const rows: Array<[string, number, number]> = [
    ["semantic", hit.score_breakdown.semantic, normalized.semantic],
    ["lexical", hit.score_breakdown.lexical, normalized.lexical],
    ["phrase", hit.score_breakdown.phrase, normalized.phrase],
    ["section", hit.score_breakdown.section, normalized.section],
    ["path", hit.score_breakdown.path, normalized.path],
    ["metadata", hit.score_breakdown.metadata, normalized.metadata],
  ];

  return (
    <div className="score-bars" aria-label="Score breakdown">
      {rows.map(([label, raw, normalized]) => (
        <div className="score-row" key={label}>
          <span>{label}</span>
          <div>
            <i style={{ width: `${Math.max(4, normalized * 100)}%` }} />
          </div>
          <strong>{formatScore(raw)}</strong>
        </div>
      ))}
    </div>
  );
}

function normalizeBreakdown(breakdown: RetrievalQueryHit["score_breakdown"]) {
  const max = Math.max(
    breakdown.semantic,
    breakdown.lexical,
    breakdown.phrase,
    breakdown.section,
    breakdown.path,
    breakdown.metadata,
  );

  if (max <= 0) {
    return {
      semantic: 0,
      lexical: 0,
      phrase: 0,
      section: 0,
      path: 0,
      metadata: 0,
    };
  }

  return {
    semantic: breakdown.semantic / max,
    lexical: breakdown.lexical / max,
    phrase: breakdown.phrase / max,
    section: breakdown.section / max,
    path: breakdown.path / max,
    metadata: breakdown.metadata / max,
  };
}

function EvalPanel({
  cases,
  evalRun,
  isRunning,
  onRun,
}: {
  cases: RetrievalEvalCase[];
  evalRun: RetrievalEvalRun | null;
  isRunning: boolean;
  onRun: () => void;
}) {
  return (
    <div className="panel eval-panel">
      <div className="panel-heading">
        <h2>Retrieval Evals</h2>
        <div className="panel-actions">
          <span className="status-pill">{cases.length} cases</span>
          <button
            className="secondary-button compact"
            disabled={cases.length === 0 || isRunning}
            type="button"
            onClick={onRun}
          >
            {isRunning ? (
              <Loader2 aria-hidden="true" className="spin" size={16} />
            ) : (
              <FlaskConical aria-hidden="true" size={16} />
            )}
            Run
          </button>
        </div>
      </div>

      {evalRun ? (
        <>
          <div className="eval-summary">
            <span>
              <CheckCircle2 aria-hidden="true" size={16} />
              {evalRun.passed_count}/{evalRun.case_count} passed
            </span>
            <span>recall {formatPercent(evalRun.average_recall_at_k)}</span>
            <span>
              precision {formatPercent(evalRun.average_precision_at_k)}
            </span>
          </div>
          <div className="eval-result-list">
            {evalRun.results.map((result) => (
              <div className="eval-result-row" key={result.case_id}>
                <strong>{result.query}</strong>
                <span>{result.passed ? "pass" : "fail"}</span>
                <small>
                  recall {formatPercent(result.recall_at_k)} · precision{" "}
                  {formatPercent(result.precision_at_k)} · top rank{" "}
                  {result.top_hit_rank ?? "none"}
                </small>
              </div>
            ))}
          </div>
        </>
      ) : cases.length > 0 ? (
        <div className="eval-result-list">
          {cases.slice(0, 4).map((evalCase) => (
            <div className="eval-result-row" key={evalCase.id}>
              <strong>{evalCase.name}</strong>
              <span>top {evalCase.top_k}</span>
              <small>{evalCase.query}</small>
            </div>
          ))}
        </div>
      ) : (
        <p>Save a cited result to create the first eval case.</p>
      )}
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

function formatScore(score: number) {
  return score.toFixed(2);
}

function formatPercent(score: number) {
  return `${Math.round(score * 100)}%`;
}
