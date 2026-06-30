import { FilePlus2 } from "lucide-react";
import { useState, type FormEvent } from "react";

import type { RetrievalEvalExperiment } from "../../../../lib/api/evalLab";
import type { DebugReportPrivacyMode } from "../../../../lib/api/reports";
import type { TraceSummary } from "../../../../lib/api/traces";
import { useCreateDebugReport } from "../hooks/useReports";
import styles from "../ReportsPage.module.css";

interface ReportCreationPanelProps {
  traces: TraceSummary[];
  experiments: RetrievalEvalExperiment[];
  sourcesLoading: boolean;
  onCreated: (reportId: string) => void;
}

export function ReportCreationPanel({
  traces,
  experiments,
  sourcesLoading,
  onCreated,
}: ReportCreationPanelProps) {
  const [sourceType, setSourceType] = useState<"trace" | "experiment">("trace");
  const [sourceId, setSourceId] = useState("");
  const [privacyMode, setPrivacyMode] =
    useState<DebugReportPrivacyMode>("metadata_only");
  const createReport = useCreateDebugReport();
  const options = sourceType === "trace" ? traces : experiments;

  function changeSourceType(nextType: "trace" | "experiment") {
    setSourceType(nextType);
    setSourceId("");
    createReport.reset();
  }

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!sourceId || createReport.isPending) return;

    createReport.mutate(
      { sourceType, sourceId, privacyMode },
      { onSuccess: (report) => onCreated(report.id) },
    );
  }

  return (
    <section className={styles.creationPanel} aria-labelledby="create-report">
      <div className={styles.panelHeading}>
        <div>
          <h2 id="create-report">Create an audit report</h2>
          <p>
            Turn a saved run or Eval Lab experiment into a reviewable diagnosis.
          </p>
        </div>
        <FilePlus2 aria-hidden="true" size={18} />
      </div>

      <form className={styles.creationForm} onSubmit={submit}>
        <div className={styles.formField}>
          <label htmlFor="report-source-type">Report source</label>
          <select
            id="report-source-type"
            value={sourceType}
            onChange={(event) =>
              changeSourceType(event.target.value as "trace" | "experiment")
            }
          >
            <option value="trace">Saved retrieval run</option>
            <option value="experiment">Eval Lab experiment</option>
          </select>
        </div>
        <div className={styles.formField}>
          <label htmlFor="report-source-id">
            {sourceType === "trace" ? "Run" : "Experiment"}
          </label>
          <select
            id="report-source-id"
            value={sourceId}
            onChange={(event) => setSourceId(event.target.value)}
            disabled={sourcesLoading}
          >
            <option value="">
              {sourcesLoading ? "Loading sources…" : "Select a source"}
            </option>
            {options.map((option) => (
              <option key={option.id} value={option.id}>
                {"query" in option ? option.query : option.name}
              </option>
            ))}
          </select>
        </div>
        <div className={styles.formField}>
          <label htmlFor="report-privacy-mode">Privacy mode</label>
          <select
            id="report-privacy-mode"
            value={privacyMode}
            onChange={(event) =>
              setPrivacyMode(event.target.value as DebugReportPrivacyMode)
            }
          >
            <option value="metadata_only">Metadata only</option>
            <option value="snippets_allowed">Approved snippets</option>
            <option value="full_local_only">Full local diagnostics</option>
          </select>
        </div>
        <button type="submit" disabled={!sourceId || createReport.isPending}>
          <FilePlus2 aria-hidden="true" size={16} />
          {createReport.isPending ? "Creating…" : "Create report"}
        </button>
      </form>
      <p className={styles.privacyHint}>{privacyDescription(privacyMode)}</p>
      {createReport.isError ? (
        <p className={styles.formError} role="alert">
          {createReport.error instanceof Error
            ? createReport.error.message
            : "The report could not be created."}
        </p>
      ) : null}
    </section>
  );
}

function privacyDescription(mode: DebugReportPrivacyMode) {
  switch (mode) {
    case "snippets_allowed":
      return "Includes approved query and bounded evidence snippets in shareable output.";
    case "full_local_only":
      return "Keeps complete diagnostics local. Markdown export is intentionally blocked.";
    default:
      return "Safest default: includes identifiers, metrics, labels, and recommendations only.";
  }
}
