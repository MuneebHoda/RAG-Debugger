import type { Trace, TraceSpan } from "../../../../lib/api/traces";
import styles from "../TraceDetailPage.module.css";

export function TraceTimeline({ trace }: { trace: Trace }) {
  const imported = trace.ingestion?.spans ?? [];
  return (
    <section className={styles.panel}>
      <div className={styles.panelHeading}>
        <div>
          <h2>Run timeline</h2>
          <p>
            {trace.ingestion
              ? "Mapped parent and child operations from the external trace."
              : "Ordered processing stages for this retrieval test."}
          </p>
        </div>
      </div>
      {trace.ingestion ? (
        imported.length === 0 ? (
          <p>No spans were mapped.</p>
        ) : (
          <ul className={styles.timeline} aria-label="Imported span hierarchy">
            {imported
              .filter(
                (span) =>
                  !span.parent_span_id ||
                  !imported.some(
                    (candidate) =>
                      candidate.external_span_id === span.parent_span_id,
                  ),
              )
              .map((span) => (
                <ImportedSpanNode
                  key={span.external_span_id}
                  span={span}
                  spans={imported}
                />
              ))}
          </ul>
        )
      ) : (
        <div className={styles.timeline}>
          {trace.spans.map((span) => (
            <SpanCard key={span.id} span={span} />
          ))}
        </div>
      )}
    </section>
  );
}

function ImportedSpanNode({
  span,
  spans,
}: {
  span: NonNullable<Trace["ingestion"]>["spans"][number];
  spans: NonNullable<Trace["ingestion"]>["spans"];
}) {
  const children = spans.filter(
    (candidate) => candidate.parent_span_id === span.external_span_id,
  );
  return (
    <li>
      <article className={styles.spanCard}>
        <div className={styles.spanHeader}>
          <strong>{span.name}</strong>
          <span className={styles.metaPill}>{span.status}</span>
        </div>
        <div className={styles.metadata}>
          <span>ID {span.external_span_id}</span>
          <span>{span.operation}</span>
          <span>{span.kind}</span>
          <span>{span.latency_ms} ms</span>
          {span.provider ? <span>{span.provider}</span> : null}
          {span.model ? <span>{span.model}</span> : null}
          {span.input_tokens != null ? (
            <span>{span.input_tokens} input tokens</span>
          ) : null}
          {span.output_tokens != null ? (
            <span>{span.output_tokens} output tokens</span>
          ) : null}
          {span.error_type ? <span>Error {span.error_type}</span> : null}
        </div>
      </article>
      {children.length > 0 ? (
        <ul>
          {children.map((child) => (
            <ImportedSpanNode
              key={child.external_span_id}
              span={child}
              spans={spans}
            />
          ))}
        </ul>
      ) : null}
    </li>
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
