import { Loader2, Save } from "lucide-react";
import { useState } from "react";

import { EvidencePicker } from "./EvidencePicker";
import { EvidenceSelectionReview } from "./EvidenceSelectionReview";
import {
  emptyEvidenceSelection,
  type EvidenceHitRef,
} from "./evidenceSelection";
import { useSaveEvidenceLifecycle } from "./useSaveEvidenceLifecycle";
import styles from "./EvidencePicker.module.css";

export function SaveEvidenceToEvalPanel({
  sourceIdentity,
  query,
  topK,
  candidateHits = [],
  sourceNote,
  sourcePending = false,
  defaultOpen = false,
}: {
  sourceIdentity: string;
  query: string;
  topK: number;
  candidateHits?: EvidenceHitRef[];
  sourceNote: string;
  sourcePending?: boolean;
  defaultOpen?: boolean;
}) {
  const [open, setOpen] = useState(defaultOpen);
  const lifecycle = useSaveEvidenceLifecycle({
    open,
    source: {
      identity: sourceIdentity,
      query,
      topK,
      note: sourceNote,
    },
    sourcePending,
  });
  const controlsDisabled = sourcePending || lifecycle.isSaving;

  return (
    <section
      aria-busy={sourcePending || lifecycle.isSaving}
      className={styles.savePanel}
      id="quality"
    >
      <div className={styles.selectedHeader}>
        <div>
          <h3>Add to Quality</h3>
          <p className={styles.empty}>
            Save this question with expected evidence so Eval Lab can detect
            future regressions.
          </p>
        </div>
        <button
          className="secondary-button compact"
          type="button"
          onClick={() => setOpen((current) => !current)}
        >
          <Save aria-hidden="true" size={15} />
          {open ? "Close" : "Choose evidence"}
        </button>
      </div>

      {open ? (
        <>
          {sourcePending ? (
            <p
              aria-live="polite"
              className={`${styles.notice} ${styles.loading}`}
              role="status"
            >
              A new retrieval is running. Saving this previous result is paused
              until the new evidence is ready.
            </p>
          ) : null}

          {lifecycle.datasetsQuery.isPending ? (
            <p aria-live="polite" className={styles.loading} role="status">
              Loading Quality datasets…
            </p>
          ) : null}
          {lifecycle.datasetsQuery.isError ? (
            <div className={styles.error} role="alert">
              <span>Quality datasets could not be loaded.</span>
              <button
                className="secondary-button compact"
                type="button"
                onClick={() => void lifecycle.datasetsQuery.refetch()}
              >
                Retry datasets
              </button>
            </div>
          ) : null}

          <fieldset
            className={styles.form}
            disabled={controlsDisabled}
            aria-disabled={controlsDisabled}
          >
            <label className={styles.field}>
              Quality dataset
              <select
                disabled={
                  lifecycle.datasetsQuery.isPending ||
                  lifecycle.datasetsQuery.isError
                }
                value={lifecycle.datasetId}
                onChange={(event) =>
                  lifecycle.setDatasetId(event.currentTarget.value)
                }
              >
                <option value="">Choose a dataset</option>
                {(lifecycle.datasetsQuery.data ?? []).map((dataset) => (
                  <option key={dataset.id} value={dataset.id}>
                    {dataset.name}
                  </option>
                ))}
              </select>
            </label>

            {lifecycle.datasetId && lifecycle.datasetQuery.isPending ? (
              <p aria-live="polite" className={styles.loading} role="status">
                Loading selected dataset cases…
              </p>
            ) : null}
            {lifecycle.datasetQuery.isError ? (
              <div className={styles.error} role="alert">
                <span>
                  The selected dataset could not be loaded. Choose another
                  dataset or retry.
                </span>
                <button
                  className="secondary-button compact"
                  type="button"
                  onClick={() => void lifecycle.datasetQuery.refetch()}
                >
                  Retry selected dataset
                </button>
              </div>
            ) : null}

            <label className={styles.field}>
              Case name
              <input
                value={lifecycle.caseName}
                onChange={(event) =>
                  lifecycle.setCaseName(event.currentTarget.value)
                }
              />
            </label>

            <EvidencePicker
              candidateHits={candidateHits}
              disabled={controlsDisabled}
              query={query}
              selection={lifecycle.selection}
              sourceIdentity={sourceIdentity}
              onSelectionChange={lifecycle.setSelection}
            />
            <EvidenceSelectionReview
              disabled={controlsDisabled}
              selection={lifecycle.selection}
              onSelectionChange={lifecycle.setSelection}
            />

            {lifecycle.similarCases.length > 0 ? (
              <div aria-live="polite" className={styles.warning} role="status">
                Similar case already exists:{" "}
                {lifecycle.similarCases
                  .map((evalCase) => evalCase.name)
                  .join(", ")}
                .
              </div>
            ) : null}

            <label className={styles.field}>
              Notes
              <textarea
                value={lifecycle.notes}
                onChange={(event) =>
                  lifecycle.setNotes(event.currentTarget.value)
                }
              />
            </label>

            <div className={styles.actions}>
              <button
                className="primary-button"
                disabled={!lifecycle.canSave}
                type="button"
                onClick={lifecycle.save}
              >
                {lifecycle.isSaving ? (
                  <Loader2 aria-hidden="true" className="spin" size={16} />
                ) : (
                  <Save aria-hidden="true" size={16} />
                )}
                Save quality case
              </button>
              <button
                aria-label="Clear all selected evidence"
                className="secondary-button"
                disabled={
                  lifecycle.normalizedSelection.documentIds.length === 0 &&
                  lifecycle.normalizedSelection.chunkIds.length === 0
                }
                type="button"
                onClick={() => lifecycle.setSelection(emptyEvidenceSelection())}
              >
                Clear evidence
              </button>
            </div>
          </fieldset>
          {lifecycle.saveSucceeded ? (
            <p aria-live="polite" className={styles.success} role="status">
              Quality case saved.
            </p>
          ) : null}
          {lifecycle.saveError ? (
            <p className={styles.error} role="alert">
              {lifecycle.saveError}
            </p>
          ) : null}
        </>
      ) : null}
    </section>
  );
}
