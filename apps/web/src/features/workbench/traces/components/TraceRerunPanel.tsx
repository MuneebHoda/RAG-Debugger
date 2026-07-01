import { GitCompare, Loader2 } from "lucide-react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import type { RetrievalMode } from "../../../../lib/api/retrieval";
import {
  rerunTrace,
  type Trace,
  type TraceRerunComparison,
} from "../../../../lib/api/traces";
import styles from "../TraceDetailPage.module.css";
import { requestErrorMessage, signedNumber } from "../utils/traceFormatting";

export function TraceRerunPanel({ trace }: { trace: Trace }) {
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
                max={25}
                min={1}
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
              {requestErrorMessage(rerunMutation.error)}
            </p>
          ) : null}
        </div>
      </section>

      {comparison ? <ComparisonResult comparison={comparison} /> : null}
    </div>
  );
}

function ComparisonResult({
  comparison,
}: {
  comparison: TraceRerunComparison;
}) {
  const metrics = [
    ["Top-score change", signedNumber(comparison.score_delta)],
    ["Latency change", `${signedNumber(comparison.latency_delta_ms)} ms`],
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
      {comparison.diagnosis ? (
        <div className={styles.rerunDiagnosis}>
          <strong>{comparison.diagnosis.summary}</strong>
          <div>
            <span>
              Resolved:{" "}
              {comparison.diagnosis.resolved_failures.join(", ") || "none"}
            </span>
            <span>
              Introduced:{" "}
              {comparison.diagnosis.introduced_failures.join(", ") || "none"}
            </span>
            <span>
              Evidence: +{comparison.diagnosis.gained_evidence.length} / -
              {comparison.diagnosis.lost_evidence.length}
            </span>
            <span>
              Citations: +{comparison.diagnosis.gained_citations.length} / -
              {comparison.diagnosis.lost_citations.length}
            </span>
          </div>
        </div>
      ) : null}
    </section>
  );
}
