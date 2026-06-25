import {
  CheckCircle2,
  FileBarChart,
  FileSearch,
  FlaskConical,
  GitBranch,
  Loader2,
  Save,
} from "lucide-react";

import type {
  RetrievalEvalCase,
  RetrievalEvalRun,
} from "../../../lib/api/evalLab";
import type {
  RetrievalQueryHit,
  RetrievalQueryResponse,
} from "../../../lib/api/retrieval";

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

export function AnswerPanel({
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

export function HitsPanel({
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

export function EvalPanel({
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

function formatScore(score: number) {
  return score.toFixed(2);
}

function formatPercent(score: number) {
  return `${Math.round(score * 100)}%`;
}
