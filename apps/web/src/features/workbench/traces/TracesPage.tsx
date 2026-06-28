import {
  AlertCircle,
  ArrowRightLeft,
  CheckCircle2,
  GitBranch,
  Info,
  Loader2,
  RefreshCw,
  Save,
  Search,
  ShieldAlert,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import {
  createEvalLabCase,
  listEvalLabDatasets,
  type RetrievalEvalDatasetSummary,
} from "../../../lib/api/evalLab";
import {
  type EvidenceStrength,
  type RetrievalMode,
  type RetrievalQueryHit,
} from "../../../lib/api/retrieval";
import {
  getTrace,
  listTraces,
  rerunTrace,
  type FailureLabel,
  type Trace,
  type TraceRerunComparison,
  type TraceSpan,
  type TraceSummary,
} from "../../../lib/api/traces";
import { formatDateTime } from "../../../lib/dateTime";
import "./TracesPage.module.css";

const FAILURE_LABELS: Record<FailureLabel, string> = {
  missing_document: "Missing document",
  bad_chunking: "Chunking issue",
  bad_embedding: "Embedding issue",
  bad_ranking: "Ranking issue",
  bad_prompt: "Prompt issue",
  unsupported_question: "Unsupported question",
  hallucinated_answer: "Hallucinated answer",
  weak_evidence: "Weak evidence",
  missing_embedding_index: "Missing index",
  duplicate_evidence: "Duplicate evidence",
  heading_only_evidence: "Heading-only evidence",
};

const EVIDENCE_LABELS: Record<EvidenceStrength, string> = {
  strong: "Strong",
  medium: "Medium",
  weak: "Weak",
};

export function TracesPage() {
  const [summaries, setSummaries] = useState<TraceSummary[]>([]);
  const [evalDatasets, setEvalDatasets] = useState<
    RetrievalEvalDatasetSummary[]
  >([]);
  const [selectedTraceId, setSelectedTraceId] = useState<string | null>(null);
  const [trace, setTrace] = useState<Trace | null>(null);
  const [rerunMode, setRerunMode] = useState<RetrievalMode>("hybrid");
  const [rerunTopK, setRerunTopK] = useState(5);
  const [latestComparison, setLatestComparison] =
    useState<TraceRerunComparison | null>(null);
  const [isLoadingList, setIsLoadingList] = useState(true);
  const [isLoadingTrace, setIsLoadingTrace] = useState(false);
  const [isRerunning, setIsRerunning] = useState(false);
  const [isSavingEval, setIsSavingEval] = useState(false);
  const [evalMessage, setEvalMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    listTraces(controller.signal)
      .then((nextSummaries) => {
        setSummaries(nextSummaries);
        setSelectedTraceId(
          (currentId) => currentId ?? nextSummaries[0]?.id ?? null,
        );
      })
      .catch((cause: unknown) => {
        if (!controller.signal.aborted) {
          setError(errorMessage(cause));
        }
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setIsLoadingList(false);
        }
      });

    return () => controller.abort();
  }, []);

  useEffect(() => {
    const controller = new AbortController();
    listEvalLabDatasets(controller.signal)
      .then(setEvalDatasets)
      .catch((cause: unknown) => {
        if (!controller.signal.aborted) {
          setError(errorMessage(cause));
        }
      });

    return () => controller.abort();
  }, []);

  useEffect(() => {
    if (!selectedTraceId) {
      return;
    }

    const controller = new AbortController();
    getTrace(selectedTraceId, controller.signal)
      .then((nextTrace) => {
        setLatestComparison(null);
        setEvalMessage(null);
        setTrace(nextTrace);
        setRerunMode(nextTrace.retrieval?.run.retrieval_mode ?? "hybrid");
        setRerunTopK(nextTrace.retrieval?.run.top_k ?? 5);
      })
      .catch((cause: unknown) => {
        if (!controller.signal.aborted) {
          setError(errorMessage(cause));
        }
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setIsLoadingTrace(false);
        }
      });

    return () => controller.abort();
  }, [selectedTraceId]);

  const selectedSummary = useMemo(
    () => summaries.find((summary) => summary.id === selectedTraceId) ?? null,
    [selectedTraceId, summaries],
  );

  async function handleRerun() {
    if (!trace || isRerunning) {
      return;
    }

    setIsRerunning(true);
    setError(null);

    try {
      const result = await rerunTrace(trace.id, {
        retrieval_mode: rerunMode,
        top_k: rerunTopK,
      });
      setTrace(result.trace);
      setLatestComparison(result.comparison);
      setSummaries((currentSummaries) =>
        currentSummaries.map((summary) =>
          summary.id === result.trace.id
            ? {
                ...summary,
                rerun_count: result.trace.reruns.length,
                failure_labels: result.trace.failure_labels,
                evidence_strength:
                  result.trace.evidence_strength ?? summary.evidence_strength,
              }
            : summary,
        ),
      );
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setIsRerunning(false);
    }
  }

  async function handleSaveToEvalLab() {
    if (!trace || !trace.retrieval || trace.retrieval.hits.length === 0) {
      return;
    }

    setIsSavingEval(true);
    setError(null);

    try {
      const dataset = evalDatasets[0];
      if (!dataset) {
        throw new Error("Create an Eval Lab dataset before saving cases.");
      }

      const topHits = trace.retrieval.hits.slice(0, 3);
      await createEvalLabCase(dataset.id, {
        name: trace.input,
        query: trace.input,
        top_k: trace.retrieval.run.top_k,
        expected_chunk_ids: topHits.map((hit) => hit.chunk.id),
        expected_document_ids: uniqueStrings(
          topHits.map((hit) => hit.document.id),
        ),
        notes: `Saved from trace ${trace.id.slice(0, 8)}.`,
      });
      setEvalMessage(`Saved to ${dataset.name}.`);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setIsSavingEval(false);
    }
  }

  return (
    <section className="traces-page" aria-labelledby="traces-title">
      <header className="page-header">
        <div>
          <p className="eyebrow">RAG run debugger</p>
          <h1 id="traces-title">Trace Debugger</h1>
          <p>
            Save retrieval runs as timelines, inspect failure labels, and rerun
            the same query with alternate retrieval settings.
          </p>
        </div>
      </header>

      {error ? (
        <div className="alert" role="alert">
          <AlertCircle aria-hidden="true" size={18} />
          <span>{error}</span>
        </div>
      ) : null}

      <section className="trace-layout">
        <TraceList
          summaries={summaries}
          selectedTraceId={selectedTraceId}
          isLoading={isLoadingList}
          onSelect={setSelectedTraceId}
        />

        <TraceDetail
          trace={trace}
          selectedSummary={selectedSummary}
          isLoading={isLoadingTrace}
          isSavingEval={isSavingEval}
          evalMessage={evalMessage}
          onSaveToEvalLab={() => void handleSaveToEvalLab()}
        />

        <TraceRerunPanel
          comparison={latestComparison}
          isRerunning={isRerunning}
          mode={rerunMode}
          topK={rerunTopK}
          trace={trace}
          onModeChange={setRerunMode}
          onRerun={() => void handleRerun()}
          onTopKChange={setRerunTopK}
        />
      </section>
    </section>
  );
}

function TraceList({
  summaries,
  selectedTraceId,
  isLoading,
  onSelect,
}: {
  summaries: TraceSummary[];
  selectedTraceId: string | null;
  isLoading: boolean;
  onSelect: (traceId: string) => void;
}) {
  return (
    <aside className="panel trace-list-panel" aria-label="Saved traces">
      <div className="panel-heading">
        <h2>Saved traces</h2>
        <span className="status-pill">{summaries.length} runs</span>
      </div>

      {isLoading ? (
        <p>Loading traces...</p>
      ) : summaries.length === 0 ? (
        <div className="empty-state">
          <GitBranch aria-hidden="true" size={22} />
          <strong>No traces saved yet</strong>
          <p>
            Run a retrieval query, then save it as a trace from the Retrieval
            page.
          </p>
        </div>
      ) : (
        <div className="trace-list">
          {summaries.map((summary) => (
            <button
              className={
                summary.id === selectedTraceId
                  ? "trace-row selected"
                  : "trace-row"
              }
              key={summary.id}
              type="button"
              onClick={() => onSelect(summary.id)}
            >
              <span>
                <strong>{summary.query}</strong>
                <small>
                  {summary.retrieval_mode} · {summary.latency_ms} ms ·{" "}
                  {formatDateTime(summary.created_at)}
                </small>
              </span>
              <EvidencePill strength={summary.evidence_strength} />
              <FailureLabelRow labels={summary.failure_labels} />
            </button>
          ))}
        </div>
      )}
    </aside>
  );
}

function TraceDetail({
  trace,
  selectedSummary,
  isLoading,
  isSavingEval,
  evalMessage,
  onSaveToEvalLab,
}: {
  trace: Trace | null;
  selectedSummary: TraceSummary | null;
  isLoading: boolean;
  isSavingEval: boolean;
  evalMessage: string | null;
  onSaveToEvalLab: () => void;
}) {
  if (isLoading) {
    return (
      <section className="panel trace-detail-panel">
        <p>Loading trace timeline...</p>
      </section>
    );
  }

  if (!trace) {
    return (
      <section className="panel trace-detail-panel">
        <div className="empty-state">
          <Search aria-hidden="true" size={22} />
          <strong>Select a trace</strong>
          <p>Trace spans, evidence, and citations appear here.</p>
        </div>
      </section>
    );
  }

  return (
    <section className="trace-detail-stack">
      <div className="panel trace-hero-panel">
        <div className="trace-hero-header">
          <div>
            <span className={`trace-status ${trace.status}`}>
              {trace.status}
            </span>
            <h2>{trace.input}</h2>
            <p>{trace.summary}</p>
          </div>
          <div className="trace-metrics">
            <Metric
              label="Mode"
              value={trace.retrieval?.run.retrieval_mode ?? "unknown"}
            />
            <Metric
              label="Latency"
              value={`${trace.retrieval?.run.latency_ms ?? 0} ms`}
            />
            <Metric
              label="Evidence"
              value={EVIDENCE_LABELS[trace.evidence_strength ?? "weak"]}
            />
          </div>
        </div>
        <div className="panel-actions">
          <button
            className="secondary-button compact"
            disabled={!trace.retrieval?.hits.length || isSavingEval}
            type="button"
            onClick={onSaveToEvalLab}
          >
            {isSavingEval ? (
              <Loader2 aria-hidden="true" className="spin" size={16} />
            ) : (
              <Save aria-hidden="true" size={16} />
            )}
            Save to Eval Lab
          </button>
        </div>
        <FailureLabelRow labels={trace.failure_labels} />
        {selectedSummary ? (
          <small>
            {selectedSummary.span_count} spans · {selectedSummary.rerun_count}{" "}
            reruns
          </small>
        ) : null}
        {evalMessage ? (
          <div className="query-status-row">
            <span>{evalMessage}</span>
          </div>
        ) : null}
      </div>

      <div className="panel">
        <div className="panel-heading">
          <h2>Timeline</h2>
          <span className="status-pill">{trace.spans.length} spans</span>
        </div>
        <div className="trace-timeline">
          {trace.spans.map((span) => (
            <TraceSpanCard key={span.id} span={span} />
          ))}
        </div>
      </div>

      <div className="panel">
        <div className="panel-heading">
          <h2>Ranked evidence</h2>
          <span className="status-pill">
            {trace.retrieval?.hits.length ?? 0} hits
          </span>
        </div>
        {trace.retrieval && trace.retrieval.hits.length > 0 ? (
          <div className="trace-evidence-list">
            {trace.retrieval.hits.slice(0, 5).map((hit) => (
              <TraceEvidenceCard hit={hit} key={hit.chunk.id} />
            ))}
          </div>
        ) : (
          <p>No evidence was retrieved for this trace.</p>
        )}
      </div>
    </section>
  );
}

function TraceSpanCard({ span }: { span: TraceSpan }) {
  return (
    <article className={`trace-span-card ${span.status}`}>
      <div className="trace-span-icon">
        {span.status === "succeeded" ? (
          <CheckCircle2 aria-hidden="true" size={16} />
        ) : span.status === "failed" ? (
          <ShieldAlert aria-hidden="true" size={16} />
        ) : (
          <Info aria-hidden="true" size={16} />
        )}
      </div>
      <div>
        <header>
          <strong>{span.title}</strong>
          <span>{span.kind.replaceAll("_", " ")}</span>
        </header>
        <p>{span.description}</p>
        <TraceSpanDetail detail={span.detail} latencyMs={span.latency_ms} />
      </div>
    </article>
  );
}

function TraceSpanDetail({
  detail,
  latencyMs,
}: {
  detail: TraceSpan["detail"];
  latencyMs: number;
}) {
  if (detail.type === "query_input") {
    return (
      <div className="trace-detail-row">
        <span>mode {detail.retrieval_mode}</span>
        <span>top {detail.top_k}</span>
        <span>
          {detail.source_filter_count + detail.document_filter_count} filters
        </span>
      </div>
    );
  }

  if (detail.type === "retrieval") {
    return (
      <div className="trace-detail-row">
        <span>{detail.hit_count} hits</span>
        <span>top score {formatScore(detail.top_score)}</span>
        <span>{detail.embedding_readiness} embeddings</span>
        <span>{latencyMs} ms</span>
      </div>
    );
  }

  if (detail.type === "evidence_summary") {
    return (
      <div className="trace-detail-row">
        <span>{detail.answer_status}</span>
        <span>{detail.citation_count} citations</span>
        <span>{detail.strongest_evidence} evidence</span>
      </div>
    );
  }

  if (detail.type === "eval_check") {
    return (
      <div className="trace-detail-row">
        <span>{detail.checked ? "checked" : "not checked"}</span>
        <span>{detail.message}</span>
      </div>
    );
  }

  return (
    <div className="trace-detail-row">
      <span>{detail.model ?? "no generation model"}</span>
      <span>{detail.input_tokens} in</span>
      <span>{detail.output_tokens} out</span>
    </div>
  );
}

function TraceEvidenceCard({ hit }: { hit: RetrievalQueryHit }) {
  return (
    <article className="trace-evidence-card">
      <header>
        <strong>
          {hit.citation.label} {hit.document.path}
        </strong>
        <EvidencePill strength={hit.evidence_strength} />
      </header>
      <p>{hit.snippet}</p>
      <div className="trace-detail-row">
        <span>rank {hit.rank}</span>
        <span>score {formatScore(hit.score)}</span>
        <span>chunk {hit.chunk.ordinal + 1}</span>
        {hit.chunk.section_title ? (
          <span>{hit.chunk.section_title}</span>
        ) : null}
      </div>
      <MiniScoreBars hit={hit} />
    </article>
  );
}

function TraceRerunPanel({
  comparison,
  isRerunning,
  mode,
  topK,
  trace,
  onModeChange,
  onRerun,
  onTopKChange,
}: {
  comparison: TraceRerunComparison | null;
  isRerunning: boolean;
  mode: RetrievalMode;
  topK: number;
  trace: Trace | null;
  onModeChange: (mode: RetrievalMode) => void;
  onRerun: () => void;
  onTopKChange: (topK: number) => void;
}) {
  return (
    <aside className="trace-side-stack">
      <section className="panel">
        <div className="panel-heading">
          <h2>Rerun lab</h2>
          <ArrowRightLeft aria-hidden="true" size={18} />
        </div>
        <div className="config-grid">
          <label>
            Mode
            <select
              value={mode}
              onChange={(event) =>
                onModeChange(event.currentTarget.value as RetrievalMode)
              }
            >
              <option value="hybrid">Hybrid</option>
              <option value="vector">Vector</option>
              <option value="lexical">Lexical</option>
            </select>
          </label>
          <label>
            Top K
            <input
              min={1}
              max={25}
              type="number"
              value={topK}
              onChange={(event) =>
                onTopKChange(Number(event.currentTarget.value))
              }
            />
          </label>
        </div>
        <button
          className="primary-button"
          disabled={!trace || isRerunning || topK <= 0}
          type="button"
          onClick={onRerun}
        >
          {isRerunning ? (
            <Loader2 aria-hidden="true" className="spin" size={18} />
          ) : (
            <RefreshCw aria-hidden="true" size={18} />
          )}
          Rerun trace
        </button>

        {comparison ? (
          <div className="rerun-result">
            <Metric
              label="Score delta"
              value={`${comparison.score_delta >= 0 ? "+" : ""}${formatScore(
                comparison.score_delta,
              )}`}
            />
            <Metric
              label="Overlap"
              value={`${comparison.overlap_count} hits`}
            />
            <Metric
              label="Rank changes"
              value={`${comparison.changed_rank_count} chunks`}
            />
            <Metric
              label="Latency"
              value={`${comparison.latency_delta_ms >= 0 ? "+" : ""}${
                comparison.latency_delta_ms
              } ms`}
            />
          </div>
        ) : null}
      </section>

      <section className="panel explainer-panel">
        <div className="panel-heading">
          <h2>Explain it to me</h2>
          <Info aria-hidden="true" size={18} />
        </div>
        <ExplainerCard
          title="Trace"
          text="A trace is a saved RAG run: query, retrieval mode, evidence, citations, diagnosis, and reruns."
        />
        <ExplainerCard
          title="Span"
          text="A span is one step in the run. CorpusLab records query input, retrieval, evidence summary, eval status, and generation metadata."
        />
        <ExplainerCard
          title="Failure label"
          text="Labels identify likely failure sources such as missing documents, weak evidence, duplicate chunks, or missing embeddings."
        />
        <ExplainerCard
          title="Rerun"
          text="A rerun keeps the same question but changes retrieval settings so you can compare score, overlap, and ranking movement."
        />
      </section>
    </aside>
  );
}

function FailureLabelRow({ labels }: { labels: FailureLabel[] }) {
  if (labels.length === 0) {
    return (
      <div className="quality-badges">
        <span className="quality-badge good">No failure labels</span>
      </div>
    );
  }

  return (
    <div className="quality-badges">
      {labels.slice(0, 4).map((label) => (
        <span className="quality-badge warning" key={label}>
          {FAILURE_LABELS[label] ?? label}
        </span>
      ))}
    </div>
  );
}

function EvidencePill({ strength }: { strength: EvidenceStrength }) {
  return (
    <span className={`strength-pill ${strength}`}>
      {EVIDENCE_LABELS[strength]}
    </span>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <span className="trace-metric">
      <small>{label}</small>
      <strong>{value}</strong>
    </span>
  );
}

function MiniScoreBars({ hit }: { hit: RetrievalQueryHit }) {
  const rows: Array<[string, number]> = [
    ["semantic", hit.normalized_score_breakdown.semantic],
    ["lexical", hit.normalized_score_breakdown.lexical],
    ["section", hit.normalized_score_breakdown.section],
  ];

  return (
    <div className="mini-score-bars" aria-label="Trace score bars">
      {rows.map(([label, value]) => (
        <span key={label}>
          <small>{label}</small>
          <i style={{ width: `${Math.max(5, value * 100)}%` }} />
        </span>
      ))}
    </div>
  );
}

function ExplainerCard({ title, text }: { title: string; text: string }) {
  return (
    <article className="explainer-card">
      <strong>{title}</strong>
      <p>{text}</p>
    </article>
  );
}

function errorMessage(cause: unknown) {
  return cause instanceof Error ? cause.message : "Unexpected request failure";
}

function uniqueStrings(values: string[]) {
  return Array.from(new Set(values));
}

function formatScore(score: number) {
  return score.toFixed(2);
}
