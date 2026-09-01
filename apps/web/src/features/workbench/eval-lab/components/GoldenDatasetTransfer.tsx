import { useMutation, type UseMutationResult } from "@tanstack/react-query";
import { useState } from "react";

import { WorkbenchPanel } from "../../../../components/workbench/WorkbenchPanel";
import {
  goldenDatasetExportUrl,
  importGoldenDataset,
  type GoldenDataset,
  type GoldenDatasetImportMode,
  type GoldenDatasetImportSummary,
  type RetrievalEvalDatasetSummary,
} from "../../../../lib/api/evalLab";
import styles from "../QualityPage.module.css";

export function GoldenDatasetExportPanel({ datasetId }: { datasetId: string }) {
  return (
    <details className={styles.details}>
      <summary>
        Export golden dataset. Full JSON exposes queries, notes, checksums, and
        provenance; review before sharing.
      </summary>
      <a
        className="secondary-button"
        href={goldenDatasetExportUrl(datasetId, "metadata_only")}
      >
        Export metadata only
      </a>
      <a
        className="primary-button"
        href={goldenDatasetExportUrl(datasetId, "full")}
      >
        Export full dataset
      </a>
    </details>
  );
}

export function GoldenDatasetImportPanel({
  datasets,
  onApplied,
}: {
  datasets: RetrievalEvalDatasetSummary[];
  onApplied: () => void;
}) {
  const [portable, setPortable] = useState<GoldenDataset | null>(null);
  const [mode, setMode] = useState<GoldenDatasetImportMode>("create_new");
  const [targetDatasetId, setTargetDatasetId] = useState("");
  const mutation: UseMutationResult<
    GoldenDatasetImportSummary,
    Error,
    boolean
  > = useMutation({
    mutationFn: (dryRun: boolean) =>
      importGoldenDataset(portable!, {
        mode,
        targetDatasetId,
        dryRun,
        confirmReplace: !dryRun && mode === "replace_dataset",
        validationToken: mutation.data?.validation_token,
      }),
    onSuccess: (_, dryRun) => {
      if (!dryRun) onApplied();
    },
    onError: window.alert,
  });
  const summary = mutation.data;
  const canApply = Boolean(summary?.valid && summary.validation_token);
  return (
    <WorkbenchPanel
      className={styles.panel}
      description="Imports expose queries, notes, evidence IDs, and provenance. Data stays workspace-local; unresolved IDs are rejected."
      title="Import golden dataset"
    >
      <div className={styles.form} onChange={mutation.reset}>
        <input
          aria-label="Dataset JSON"
          type="file"
          onChange={(event) => readFile(event.currentTarget.files?.[0])}
        />
        <select
          aria-label="Import mode"
          value={mode}
          onChange={(event) => {
            setMode(event.currentTarget.value as GoldenDatasetImportMode);
            setTargetDatasetId("");
          }}
        >
          <option>create_new</option>
          <option>merge_by_case_key</option>
          <option>replace_dataset</option>
          <option>validate_only</option>
        </select>
        <select
          aria-label="Target (merge or replace)"
          value={targetDatasetId}
          onChange={(event) => setTargetDatasetId(event.currentTarget.value)}
        >
          <option value="">Select dataset</option>
          {datasets.map((dataset) => (
            <option key={dataset.id} value={dataset.id}>
              {dataset.name}
            </option>
          ))}
        </select>
        <button
          className="secondary-button"
          disabled={!portable || mutation.isPending}
          onClick={() => mutation.mutate(true)}
        >
          Validate dry run
        </button>
        <button
          className="primary-button"
          disabled={!canApply}
          onClick={() => {
            if (
              mode !== "replace_dataset" ||
              window.confirm("Replace dataset and delete unlisted cases?")
            ) {
              mutation.mutate(false);
            }
          }}
        >
          Apply import
        </button>
      </div>
      {summary ? (
        <output className={styles.empty}>{JSON.stringify(summary)}</output>
      ) : null}
    </WorkbenchPanel>
  );

  function readFile(file: File | undefined) {
    setPortable(null);
    file
      ?.text()
      .then((text) => JSON.parse(text) as GoldenDataset)
      .then(setPortable, () => window.alert("Invalid JSON file."));
  }
}
