import { WorkbenchPanel } from "../../../../components/workbench/WorkbenchPanel";
import type { Trace } from "../../../../lib/api/traces";
import styles from "../TraceDetailPage.module.css";
import { SaveToQualityPanel } from "./SaveToQualityPanel";
import { TraceDiagnosisPanel } from "./TraceDiagnosisPanel";
import { TraceFailureLabels } from "./TraceFailureLabels";

export function TraceSummaryPanel({ trace }: { trace: Trace }) {
  return (
    <div className={styles.stack}>
      {trace.diagnosis ? (
        <TraceDiagnosisPanel
          answerStatus={trace.retrieval?.answer.status}
          diagnosis={trace.diagnosis}
        />
      ) : (
        <section className={styles.diagnosis}>
          <span className={styles.diagnosisLabel}>
            {trace.ingestion ? "Limited diagnosis" : "Legacy diagnosis"}
          </span>
          <h2>{trace.summary}</h2>
          <p>
            {trace.ingestion
              ? "Diagnosis is limited by the imported fields and active privacy policy."
              : "This saved run uses the earlier failure-label format."}
          </p>
          <TraceFailureLabels labels={trace.failure_labels} />
        </section>
      )}

      <WorkbenchPanel
        className={styles.panel}
        description="The answer produced from the strongest retrieved excerpts."
        title="Evidence summary"
      >
        <p className={styles.answer}>
          {trace.output ??
            trace.retrieval?.answer.text ??
            "No answer was produced."}
        </p>
      </WorkbenchPanel>

      {trace.ingestion ? (
        <WorkbenchPanel
          className={styles.panel}
          description="Trusted import metadata and mapping limitations."
          title="Import details"
        >
          <dl className={styles.metadata}>
            <div>
              <dt>Source</dt>
              <dd>{trace.ingestion.source.replaceAll("_", "/")}</dd>
            </div>
            <div>
              <dt>External trace</dt>
              <dd>{trace.ingestion.external_trace_id}</dd>
            </div>
            <div>
              <dt>Mapping</dt>
              <dd>{trace.ingestion.mapping_status.replaceAll("_", " ")}</dd>
            </div>
            <div>
              <dt>Schema / mapper</dt>
              <dd>
                {trace.ingestion.schema_version} /{" "}
                {trace.ingestion.mapper_version}
              </dd>
            </div>
            <div>
              <dt>Privacy</dt>
              <dd>{trace.ingestion.privacy_mode.replaceAll("_", " ")}</dd>
            </div>
            {trace.ingestion.retrieval_mode ? (
              <div>
                <dt>Retrieval</dt>
                <dd>
                  {trace.ingestion.retrieval_mode}
                  {trace.ingestion.top_k
                    ? ` · top ${trace.ingestion.top_k}`
                    : ""}
                </dd>
              </div>
            ) : null}
            {trace.ingestion.model_config ? (
              <div>
                <dt>Model configuration</dt>
                <dd>
                  {[
                    trace.ingestion.model_config.configuration_label,
                    trace.ingestion.model_config.provider,
                    trace.ingestion.model_config.generation_model,
                    trace.ingestion.model_config.embedding_model,
                    trace.ingestion.model_config.ranker,
                  ]
                    .filter(Boolean)
                    .join(" · ") || "No model labels mapped"}
                </dd>
              </div>
            ) : null}
            {trace.ingestion.service_name ? (
              <div>
                <dt>Service</dt>
                <dd>
                  {trace.ingestion.service_name}
                  {trace.ingestion.service_version
                    ? ` · ${trace.ingestion.service_version}`
                    : ""}
                </dd>
              </div>
            ) : null}
            {trace.ingestion.deployment_environment ? (
              <div>
                <dt>Environment</dt>
                <dd>{trace.ingestion.deployment_environment}</dd>
              </div>
            ) : null}
            {trace.ingestion.instrumentation_scope_name ? (
              <div>
                <dt>Instrumentation scope</dt>
                <dd>
                  {trace.ingestion.instrumentation_scope_name}
                  {trace.ingestion.instrumentation_scope_version
                    ? ` · ${trace.ingestion.instrumentation_scope_version}`
                    : ""}
                </dd>
              </div>
            ) : null}
          </dl>
          {trace.ingestion.limitations.length > 0 ? (
            <div>
              <strong>Mapping limitations</strong>
              <ul>
                {trace.ingestion.limitations.map((limitation) => (
                  <li key={limitation}>{limitation.replaceAll("_", " ")}</li>
                ))}
              </ul>
            </div>
          ) : null}
        </WorkbenchPanel>
      ) : null}

      <SaveToQualityPanel trace={trace} />
    </div>
  );
}
