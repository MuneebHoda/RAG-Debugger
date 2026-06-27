import {
  ArrowRight,
  CheckCircle2,
  CircleAlert,
  GitCompare,
  Loader2,
  Save,
} from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { Link } from "react-router-dom";

import {
  createEvalLabCase,
  listEvalLabDatasets,
} from "../../../lib/api/evalLab";
import type {
  RetrievalMode,
  RetrievalQueryHit,
} from "../../../lib/api/retrieval";
import {
  rerunTrace,
  type FailureLabel,
  type Trace,
  type TraceRerunComparison,
  type TraceSpan,
} from "../../../lib/api/traces";
import styles from "./TraceDetailPage.module.css";

const failureLabels: Record<FailureLabel, string> = {
  missing_document: "Expected information may be missing from the corpus.",
  bad_chunking: "The relevant text may be split into weak evidence units.",
  bad_embedding: "Semantic indexing may not represent this evidence well.",
  bad_ranking: "Relevant evidence exists but ranked too low.",
  bad_prompt:
    "The answer instructions may not use the retrieved evidence correctly.",
  unsupported_question: "The corpus does not appear to support this question.",
  hallucinated_answer: "The answer contains claims not supported by citations.",
  weak_evidence: "Retrieved evidence is too weak for a confident answer.",
  missing_embedding_index:
    "Some chunks are not available to semantic retrieval.",
  duplicate_evidence: "Repeated evidence is crowding out distinct results.",
  heading_only_evidence: "A heading ranked without enough supporting content.",
};

export function TraceSummaryPanel({ trace }: { trace: Trace }) {
  const recommendation = recommendationFor(trace.failure_labels);
  return (
    <div className={styles.stack}>
      <div className={styles.summaryGrid}>
        <section className={styles.diagnosis}>
          <span className={styles.diagnosisLabel}>What happened</span>
          <h2>{trace.summary}</h2>
          <p>
            {trace.failure_labels.length === 0
              ? "CorpusLab found usable evidence without a deterministic failure signal."
              : "CorpusLab found the following likely causes."}
          </p>
          <ul className={styles.signalList}>
            {trace.failure_labels.length === 0 ? (
              <li>
                <CheckCircle2 aria-hidden="true" size={16} /> No failure signals
              </li>
            ) : (
              trace.failure_labels.map((label) => (
                <li key={label}>
                  <CircleAlert aria-hidden="true" size={16} />
                  {failureLabels[label] ?? label}
                </li>
              ))
            )}
          </ul>
        </section>

        <section className={styles.actionPanel}>
          <h2>Recommended next action</h2>
          <p>{recommendation.detail}</p>
          <Link to={recommendation.route}>
            {recommendation.label} <ArrowRight aria-hidden="true" size={16} />
          </Link>
        </section>
      </div>

      <section className={styles.panel}>
        <div className={styles.panelHeading}>
          <div>
            <h2>Evidence summary</h2>
            <p>The answer produced from the strongest retrieved excerpts.</p>
          </div>
        </div>
        <p className={styles.answer}>
          {trace.output ??
            trace.retrieval?.answer.text ??
            "No answer was produced."}
        </p>
      </section>

      <SaveToQualityPanel trace={trace} />
    </div>
  );
}

export function TraceEvidencePanel({ trace }: { trace: Trace }) {
  const hits = trace.retrieval?.hits ?? [];
  return (
    <section className={styles.panel}>
      <div className={styles.panelHeading}>
        <div>
          <h2>Ranked evidence</h2>
          <p>{hits.length} chunks were returned for this run.</p>
        </div>
      </div>
      {hits.length === 0 ? (
        <p className={styles.answer}>No evidence was retrieved.</p>
      ) : (
        <div className={styles.evidenceList}>
          {hits.map((hit) => (
            <EvidenceCard hit={hit} key={hit.chunk.id} />
          ))}
        </div>
      )}
    </section>
  );
}

export function TraceTimelinePanel({ trace }: { trace: Trace }) {
  return (
    <section className={styles.panel}>
      <div className={styles.panelHeading}>
        <div>
          <h2>Run timeline</h2>
          <p>Ordered processing stages for this retrieval test.</p>
        </div>
      </div>
      <div className={styles.timeline}>
        {trace.spans.map((span) => (
          <SpanCard key={span.id} span={span} />
        ))}
      </div>
    </section>
  );
}

export function TraceComparePanel({ trace }: { trace: Trace }) {
  const queryClient = useQueryClient();
  const [mode, setMode] = useState<RetrievalMode>(
    trace.retrieval?.run.retrieval_mode ?? "hybrid",
  );
  const [topK, setTopK] = useState(trace.retrieval?.run.top_k ?? 5);
  const [comparison, setComparison] = useState<TraceRerunComparison | null>(
    trace.reruns.at(-1) ?? null,
  );
  const rerunMutation = useMutation({
    mutationFn: () =>
      rerunTrace(trace.id, { retrieval_mode: mode, top_k: topK }),
    onSuccess: (result) => {
      queryClient.setQueryData(["trace", trace.id], result.trace);
      queryClient.invalidateQueries({ queryKey: ["traces"] });
      queryClient.invalidateQueries({ queryKey: ["overview"] });
      setComparison(result.comparison);
    },
  });

  return (
    <div className={styles.stack}>
      <section className={styles.panel}>
        <div className={styles.panelHeading}>
          <div>
            <h2>Compare retrieval settings</h2>
            <p>Keep the question fixed and change how evidence is ranked.</p>
          </div>
          <GitCompare aria-hidden="true" size={19} />
        </div>
        <div className={styles.compareForm}>
          <div className={styles.formGrid}>
            <label>
              Retrieval mode
              <select
                value={mode}
                onChange={(event) =>
                  setMode(event.currentTarget.value as RetrievalMode)
                }
              >
                <option value="hybrid">Hybrid</option>
                <option value="vector">Vector</option>
                <option value="lexical">Lexical</option>
              </select>
            </label>
            <label>
              Results to return
              <input
                min={1}
                max={25}
                type="number"
                value={topK}
                onChange={(event) => setTopK(Number(event.currentTarget.value))}
              />
            </label>
          </div>
          <button
            className={styles.primaryButton}
            disabled={rerunMutation.isPending || topK < 1}
            type="button"
            onClick={() => rerunMutation.mutate()}
          >
            {rerunMutation.isPending ? (
              <Loader2 aria-hidden="true" className="spin" size={16} />
            ) : (
              <GitCompare aria-hidden="true" size={16} />
            )}
            Run comparison
          </button>
          {rerunMutation.isError ? (
            <p className={styles.errorMessage} role="alert">
              {errorMessage(rerunMutation.error)}
            </p>
          ) : null}
        </div>
      </section>

      {comparison ? <ComparisonResult comparison={comparison} /> : null}
    </div>
  );
}

function SaveToQualityPanel({ trace }: { trace: Trace }) {
  const [open, setOpen] = useState(false);
  const [datasetId, setDatasetId] = useState("");
  const [selectedChunkIds, setSelectedChunkIds] = useState<string[]>([]);
  const datasetsQuery = useQuery({
    queryKey: ["eval-datasets"],
    queryFn: ({ signal }) => listEvalLabDatasets(signal),
    enabled: open,
  });
  const saveMutation = useMutation({
    mutationFn: () => {
      const hits = trace.retrieval?.hits ?? [];
      const selectedHits = hits.filter((hit) =>
        selectedChunkIds.includes(hit.chunk.id),
      );
      return createEvalLabCase(datasetId, {
        name: trace.input,
        query: trace.input,
        top_k: trace.retrieval?.run.top_k ?? 5,
        expected_chunk_ids: selectedChunkIds,
        expected_document_ids: Array.from(
          new Set(selectedHits.map((hit) => hit.document.id)),
        ),
        notes: `Saved from run ${trace.id.slice(0, 8)}.`,
      });
    },
  });
  const hits = trace.retrieval?.hits ?? [];

  return (
    <section className={styles.qualityPanel}>
      <div className={styles.panelHeading}>
        <div>
          <h2>Add to Quality</h2>
          <p>
            Record the evidence this question should retrieve in future tests.
          </p>
        </div>
        <button
          className={styles.secondaryButton}
          type="button"
          onClick={() => setOpen((current) => !current)}
        >
          <Save aria-hidden="true" size={15} />
          {open ? "Close" : "Choose evidence"}
        </button>
      </div>

      {open ? (
        <div className={styles.qualityForm}>
          <label>
            Quality dataset
            <select
              value={datasetId}
              onChange={(event) => setDatasetId(event.currentTarget.value)}
            >
              <option value="">Choose a dataset</option>
              {(datasetsQuery.data ?? []).map((dataset) => (
                <option key={dataset.id} value={dataset.id}>
                  {dataset.name}
                </option>
              ))}
            </select>
          </label>
          <div className={styles.hitOptions}>
            {hits.slice(0, 5).map((hit) => (
              <label className={styles.hitOption} key={hit.chunk.id}>
                <input
                  checked={selectedChunkIds.includes(hit.chunk.id)}
                  type="checkbox"
                  onChange={() =>
                    setSelectedChunkIds((current) =>
                      current.includes(hit.chunk.id)
                        ? current.filter((id) => id !== hit.chunk.id)
                        : [...current, hit.chunk.id],
                    )
                  }
                />
                <span>
                  <strong>
                    #{hit.rank} {hit.document.path}
                  </strong>
                  <small>{hit.snippet}</small>
                </span>
              </label>
            ))}
          </div>
          <button
            className={styles.primaryButton}
            disabled={
              !datasetId ||
              selectedChunkIds.length === 0 ||
              saveMutation.isPending
            }
            type="button"
            onClick={() => saveMutation.mutate()}
          >
            {saveMutation.isPending ? (
              <Loader2 aria-hidden="true" className="spin" size={16} />
            ) : (
              <Save aria-hidden="true" size={16} />
            )}
            Save quality case
          </button>
          {saveMutation.isSuccess ? (
            <p className={styles.message}>Quality case saved.</p>
          ) : null}
          {saveMutation.isError ? (
            <p className={styles.errorMessage} role="alert">
              {errorMessage(saveMutation.error)}
            </p>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}

function EvidenceCard({ hit }: { hit: RetrievalQueryHit }) {
  const scores = [
    ["semantic", hit.normalized_score_breakdown.semantic],
    ["lexical", hit.normalized_score_breakdown.lexical],
    ["section", hit.normalized_score_breakdown.section],
  ] as const;
  return (
    <article className={styles.evidenceCard}>
      <div className={styles.evidenceHeader}>
        <strong>
          #{hit.rank} {hit.document.path}
        </strong>
        <span className={styles[hit.evidence_strength]}>
          {hit.evidence_strength}
        </span>
      </div>
      <p>{hit.snippet}</p>
      <div className={styles.metadata}>
        <span>score {hit.score.toFixed(2)}</span>
        <span>chunk {hit.chunk.ordinal + 1}</span>
        {hit.chunk.section_title ? (
          <span>{hit.chunk.section_title}</span>
        ) : null}
        <span>{hit.citation.checksum_prefix}</span>
      </div>
      <div className={styles.scoreBars} aria-label="Score breakdown">
        {scores.map(([label, value]) => (
          <div className={styles.scoreBar} key={label}>
            <span>{label}</span>
            <div className={styles.track}>
              <i style={{ width: `${Math.max(3, value * 100)}%` }} />
            </div>
          </div>
        ))}
      </div>
    </article>
  );
}

function SpanCard({ span }: { span: TraceSpan }) {
  return (
    <article className={styles.spanCard}>
      <div className={styles.spanHeader}>
        <strong>{span.title}</strong>
        <span className={styles.metaPill}>{span.status}</span>
      </div>
      <p>{span.description}</p>
      <div className={styles.metadata}>
        <span>{span.kind.replaceAll("_", " ")}</span>
        <span>{span.latency_ms} ms</span>
        {spanDetail(span)}
      </div>
    </article>
  );
}

function spanDetail(span: TraceSpan) {
  const detail = span.detail;
  if (detail.type === "query_input") {
    return (
      <span>
        top {detail.top_k} · {detail.retrieval_mode}
      </span>
    );
  }
  if (detail.type === "retrieval") {
    return (
      <span>
        {detail.hit_count} hits · {detail.embedding_readiness} index
      </span>
    );
  }
  if (detail.type === "evidence_summary") {
    return (
      <span>
        {detail.citation_count} citations · {detail.strongest_evidence}
      </span>
    );
  }
  if (detail.type === "eval_check") {
    return <span>{detail.message}</span>;
  }
  return <span>{detail.model ?? "No generation model"}</span>;
}

function ComparisonResult({
  comparison,
}: {
  comparison: TraceRerunComparison;
}) {
  const metrics = [
    ["Top-score change", signed(comparison.score_delta)],
    ["Latency change", `${signed(comparison.latency_delta_ms)} ms`],
    ["Evidence overlap", `${comparison.overlap_count} chunks`],
    ["Rank movement", `${comparison.changed_rank_count} chunks`],
  ];
  return (
    <section className={styles.panel}>
      <div className={styles.panelHeading}>
        <div>
          <h2>Latest comparison</h2>
          <p>
            {comparison.response.run.retrieval_mode} · top{" "}
            {comparison.response.run.top_k}
          </p>
        </div>
      </div>
      <div className={styles.comparisonGrid}>
        {metrics.map(([label, value]) => (
          <div className={styles.comparisonMetric} key={label}>
            <small>{label}</small>
            <strong>{value}</strong>
          </div>
        ))}
      </div>
    </section>
  );
}

function recommendationFor(labels: FailureLabel[]) {
  if (
    labels.includes("missing_document") ||
    labels.includes("unsupported_question")
  ) {
    return {
      label: "Review Corpus",
      detail: "Confirm the supporting document is present and readable.",
      route: "/app/sources",
    };
  }
  if (
    labels.includes("bad_embedding") ||
    labels.includes("missing_embedding_index")
  ) {
    return {
      label: "Review indexing",
      detail: "Refresh embeddings before comparing semantic retrieval.",
      route: "/app/retrieval",
    };
  }
  if (labels.length > 0) {
    return {
      label: "Compare retrieval",
      detail:
        "Rerun this question with another ranking mode and compare evidence.",
      route: "?tab=compare",
    };
  }
  return {
    label: "Add quality case",
    detail: "Preserve this successful result as regression coverage.",
    route: "?tab=summary#quality",
  };
}

function signed(value: number) {
  return `${value >= 0 ? "+" : ""}${value.toFixed(2)}`;
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "Request failed";
}
