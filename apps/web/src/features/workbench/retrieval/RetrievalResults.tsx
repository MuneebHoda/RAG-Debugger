import { FileSearch, GitBranch, Loader2 } from "lucide-react";

import type {
  RetrievalQueryHit,
  RetrievalQueryResponse,
} from "../../../lib/api/retrieval";
import styles from "./RetrievalPage.module.css";

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

const ANSWER_SUPPORT_LABELS: Record<string, string> = {
  direct_body_support: "Direct body support",
  insufficient_body_overlap: "Insufficient body overlap",
  semantic_only_match: "Semantic-only candidate",
  metadata_only_match: "Metadata-only candidate",
  path_only_match: "Path-only candidate",
  section_only_match: "Section-only candidate",
  weak_evidence: "Weak evidence",
  heading_only_evidence: "Heading-only evidence",
  unassessed: "Not assessed",
};

export function AnswerPanel({
  response,
  isQuerying,
  isSavingTrace,
  onSaveTrace,
}: {
  response: RetrievalQueryResponse | null;
  isQuerying: boolean;
  isSavingTrace: boolean;
  onSaveTrace: () => void;
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
            title="Save and diagnose this run"
            type="button"
            onClick={onSaveTrace}
          >
            {isSavingTrace ? (
              <Loader2 aria-hidden="true" className="spin" size={16} />
            ) : (
              <GitBranch aria-hidden="true" size={16} />
            )}
            Debug this run
          </button>
        </div>
      </div>

      {isQuerying ? (
        <p>Retrieving local evidence...</p>
      ) : response ? (
        <>
          <EmbeddingQueryStatus response={response} />
          {response.diagnosis ? (
            <div className={styles.diagnosisNotice}>
              <strong>{response.diagnosis.outcome}</strong>
              <span>{response.diagnosis.summary}</span>
            </div>
          ) : null}
          <div
            className={`${styles.answerState} ${
              response.answer.status === "answered"
                ? styles.answerStateAnswered
                : styles.answerStateInsufficient
            }`}
          >
            <strong>
              {response.answer.status === "answered"
                ? "Answered from chunk body evidence"
                : "Insufficient evidence"}
            </strong>
            <span>
              {response.answer.status === "answered"
                ? "Every citation below passed the direct body-support gate."
                : "Candidates may still appear below for debugging, but none can be cited as direct support."}
            </span>
          </div>
          <p className={styles.answerText}>{response.answer.text}</p>
          {response.answer.citations.length > 0 ? (
            <div className={styles.citationList}>
              {response.answer.citations.map((citation) => (
                <span key={`${citation.label}-${citation.chunk_id}`}>
                  {citation.label} {citation.document_path} · chunk{" "}
                  {citation.chunk_ordinal + 1}
                </span>
              ))}
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

function EmbeddingQueryStatus({
  response,
}: {
  response: RetrievalQueryResponse;
}) {
  if (!response.embedding_status.required) {
    return null;
  }

  return (
    <div className={styles.queryStatusRow}>
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
    <div className={styles.hitList}>
      {Object.entries(groups).map(([documentId, documentHits]) => (
        <section className={styles.hitGroup} key={documentId}>
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
  const support = hit.answer_support ?? {
    status: "unassessed",
    reason: "unassessed",
  };
  return (
    <article className={styles.hitCard}>
      <header>
        <strong>
          {hit.citation.label} Rank {hit.rank}
        </strong>
        <span className={`strength-pill ${hit.evidence_strength ?? "medium"}`}>
          {EVIDENCE_LABELS[hit.evidence_strength ?? "medium"]} ·{" "}
          {formatScore(hit.score)}
        </span>
      </header>

      <div className={styles.hitSource}>
        <FileSearch aria-hidden="true" size={16} />
        <span>
          {hit.document.path} · chunk {hit.chunk.ordinal + 1}
          {hit.chunk.section_title ? ` · ${hit.chunk.section_title}` : ""}
        </span>
      </div>

      <p>{hit.snippet}</p>

      <div className={styles.termRow}>
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
        <span
          className={`${styles.supportBadge} ${
            support.status === "supported"
              ? styles.supported
              : styles.unsupported
          }`}
        >
          {support.status === "supported"
            ? "Supports answer"
            : "Candidate only"}
          {` · ${ANSWER_SUPPORT_LABELS[support.reason] ?? support.reason}`}
        </span>
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
    <div className={styles.scoreBars} aria-label="Score breakdown">
      {rows.map(([label, raw, normalized]) => (
        <div className={styles.scoreRow} key={label}>
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
