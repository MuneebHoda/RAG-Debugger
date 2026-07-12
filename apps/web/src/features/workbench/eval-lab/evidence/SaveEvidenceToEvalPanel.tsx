import { Loader2, Save } from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState } from "react";

import {
  createEvalLabCase,
  getEvalLabDataset,
  listEvalLabDatasets,
} from "../../../../lib/api/evalLab";
import { EvidencePicker } from "./EvidencePicker";
import { EvidenceSelectionReview } from "./EvidenceSelectionReview";
import {
  emptyEvidenceSelection,
  findSimilarCases,
  hasExpectedEvidence,
  normalizeEvidenceSelection,
  type EvidenceHitRef,
  type EvidenceSelection,
} from "./evidenceSelection";
import styles from "./EvidencePicker.module.css";

export function SaveEvidenceToEvalPanel({
  query,
  topK,
  candidateHits = [],
  sourceNote,
  defaultOpen = false,
}: {
  query: string;
  topK: number;
  candidateHits?: EvidenceHitRef[];
  sourceNote: string;
  defaultOpen?: boolean;
}) {
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(defaultOpen);
  const [datasetId, setDatasetId] = useState("");
  const [caseName, setCaseName] = useState(query);
  const [notes, setNotes] = useState(sourceNote);
  const [selection, setSelection] = useState<EvidenceSelection>(() =>
    emptyEvidenceSelection(),
  );
  const datasetsQuery = useQuery({
    queryKey: ["eval-datasets"],
    queryFn: ({ signal }) => listEvalLabDatasets(signal),
    enabled: open,
  });
  const datasetQuery = useQuery({
    queryKey: ["eval-dataset", datasetId],
    queryFn: ({ signal }) => getEvalLabDataset(datasetId, signal),
    enabled: open && Boolean(datasetId),
  });
  const similarCases = useMemo(
    () =>
      datasetQuery.data ? findSimilarCases(datasetQuery.data.cases, query) : [],
    [datasetQuery.data, query],
  );
  const normalizedSelection = normalizeEvidenceSelection(selection);
  const saveMutation = useMutation({
    mutationFn: () =>
      createEvalLabCase(datasetId, {
        name: caseName.trim() || query.trim(),
        query: query.trim(),
        top_k: topK,
        expected_chunk_ids: normalizedSelection.chunkIds,
        expected_document_ids: normalizedSelection.documentIds,
        notes: notes.trim() || null,
      }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["eval-datasets"] });
      void queryClient.invalidateQueries({
        queryKey: ["eval-dataset", datasetId],
      });
    },
  });

  return (
    <section className={styles.savePanel} id="quality">
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
          <label className={styles.field}>
            Quality dataset
            <select
              value={datasetId}
              onChange={(event) => setDatasetId(event.currentTarget.value)}
            >
              <option value="">Choose a dataset</option>
              {(datasetsQuery.data ?? []).map((dataset) => (
                <option key={dataset.id} value={dataset.id}>
                  {dataset.name}
                </option>
              ))}
            </select>
          </label>

          <label className={styles.field}>
            Case name
            <input
              value={caseName}
              onChange={(event) => setCaseName(event.currentTarget.value)}
            />
          </label>

          <EvidencePicker
            candidateHits={candidateHits}
            query={query}
            selection={selection}
            onSelectionChange={setSelection}
          />
          <EvidenceSelectionReview
            selection={selection}
            onSelectionChange={setSelection}
          />

          {similarCases.length > 0 ? (
            <div className={styles.warning} role="status">
              Similar case already exists:{" "}
              {similarCases.map((evalCase) => evalCase.name).join(", ")}.
            </div>
          ) : null}

          <label className={styles.field}>
            Notes
            <textarea
              value={notes}
              onChange={(event) => setNotes(event.currentTarget.value)}
            />
          </label>

          <div className={styles.actions}>
            <button
              className="primary-button"
              disabled={
                !datasetId ||
                !query.trim() ||
                !hasExpectedEvidence(normalizedSelection) ||
                saveMutation.isPending
              }
              type="button"
              onClick={() => saveMutation.mutate()}
            >
              {saveMutation.isPending ? (
                <Loader2 aria-hidden="true" className="spin" size={16} />
              ) : (
                <Save aria-hidden="true" size={16} />
              )}
              Save quality case
            </button>
            <button
              className="secondary-button"
              type="button"
              onClick={() => setSelection(emptyEvidenceSelection())}
            >
              Clear evidence
            </button>
          </div>
          {saveMutation.isSuccess ? (
            <p className={styles.empty}>Quality case saved.</p>
          ) : null}
          {saveMutation.isError ? (
            <p className={styles.error} role="alert">
              {errorMessage(saveMutation.error)}
            </p>
          ) : null}
        </>
      ) : null}
    </section>
  );
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "Request failed";
}
