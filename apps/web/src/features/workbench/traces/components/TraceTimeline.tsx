import type { Trace, TraceSpan } from "../../../../lib/api/traces";
import styles from "../TraceDetailPage.module.css";

export function TraceTimeline({ trace }: { trace: Trace }) {
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
  if (detail.type === "eval_check") return <span>{detail.message}</span>;
  return <span>{detail.model ?? "No generation model"}</span>;
}
