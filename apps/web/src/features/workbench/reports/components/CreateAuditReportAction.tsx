import { FilePlus2, X } from "lucide-react";
import { useId, useRef, useState, type FormEvent } from "react";
import { useNavigate } from "react-router-dom";

import type { DebugReportPrivacyMode } from "../../../../lib/api/reports";
import { useCreateDebugReport } from "../hooks/useReports";
import styles from "./CreateAuditReportAction.module.css";

export type AuditReportSource =
  | { sourceType: "trace"; sourceId: string }
  | { sourceType: "experiment"; sourceId: string }
  | { sourceType: "ci_run"; sourceId: string };

interface CreateAuditReportActionProps {
  source: AuditReportSource;
  compact?: boolean;
}

export function CreateAuditReportAction({
  source,
  compact = false,
}: CreateAuditReportActionProps) {
  const navigate = useNavigate();
  const privacyId = useId();
  const [isOpen, setIsOpen] = useState(false);
  const [privacyMode, setPrivacyMode] =
    useState<DebugReportPrivacyMode>("metadata_only");
  const submissionInFlight = useRef(false);
  const createReport = useCreateDebugReport();

  function close() {
    if (submissionInFlight.current || createReport.isPending) return;
    setIsOpen(false);
    setPrivacyMode("metadata_only");
    createReport.reset();
  }

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (submissionInFlight.current || createReport.isPending) return;
    submissionInFlight.current = true;

    createReport.mutate(
      { ...source, privacyMode },
      {
        onSuccess: (report) => navigate(`/app/reports/${report.id}`),
        onSettled: () => {
          submissionInFlight.current = false;
        },
      },
    );
  }

  if (!isOpen) {
    return (
      <button
        className={compact ? styles.compactTrigger : styles.trigger}
        type="button"
        onClick={() => setIsOpen(true)}
      >
        <FilePlus2 aria-hidden="true" size={15} /> Create audit report
      </button>
    );
  }

  return (
    <form
      className={compact ? styles.compactPanel : styles.panel}
      onSubmit={submit}
    >
      <div className={styles.copy}>
        <strong>Create audit report</strong>
        <span>{privacyDescription(privacyMode)}</span>
      </div>
      <label htmlFor={privacyId}>Privacy</label>
      <select
        id={privacyId}
        value={privacyMode}
        disabled={createReport.isPending}
        onChange={(event) => {
          setPrivacyMode(event.target.value as DebugReportPrivacyMode);
          createReport.reset();
        }}
      >
        <option value="metadata_only">Metadata only</option>
        <option value="snippets_allowed">Approved snippets</option>
        <option value="full_local_only">Full local diagnostics</option>
      </select>
      <button
        className={styles.confirm}
        type="submit"
        disabled={createReport.isPending}
      >
        <FilePlus2 aria-hidden="true" size={15} />
        {createReport.isPending ? "Creating…" : "Create report"}
      </button>
      <button
        aria-label="Cancel audit report"
        className={styles.cancel}
        type="button"
        disabled={createReport.isPending}
        onClick={close}
      >
        <X aria-hidden="true" size={15} />
      </button>
      {createReport.isError ? (
        <p className={styles.error} role="alert">
          {createReport.error instanceof Error
            ? createReport.error.message
            : "The audit report could not be created."}
        </p>
      ) : null}
    </form>
  );
}

function privacyDescription(mode: DebugReportPrivacyMode) {
  if (mode === "snippets_allowed") {
    return "Includes approved query and bounded evidence snippets.";
  }
  if (mode === "full_local_only") {
    return "Keeps unrestricted diagnostics local and blocks export.";
  }
  return "Safest default. Excludes query, path, section, and snippet content.";
}
