import { Loader2, Save } from "lucide-react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { useState } from "react";

import {
  createEvalLabCase,
  listEvalLabDatasets,
} from "../../../../lib/api/evalLab";
import type { Trace } from "../../../../lib/api/traces";
import styles from "../TraceDetailPage.module.css";
import { requestErrorMessage } from "../utils/traceFormatting";

export function SaveToQualityPanel({ trace }: { trace: Trace }) {
  const [open, setOpen] = useState(false);
  const [datasetId, setDatasetId] = useState("");
  const [selectedChunkIds, setSelectedChunkIds] = useState<string[]>([]);
  const datasetsQuery = useQuery({
    queryKey: ["eval-datasets"],
    queryFn: ({ signal }) => listEvalLabDatasets(signal),
    enabled: open,
  });
  const saveMutation = useMutation({
    mutationFn: () => {
      const hits = trace.retrieval?.hits ?? [];
      const selectedHits = hits.filter((hit) =>
        selectedChunkIds.includes(hit.chunk.id),
      );
      return createEvalLabCase(datasetId, {
        name: trace.input,
        query: trace.input,
        top_k: trace.retrieval?.run.top_k ?? 5,
        expected_chunk_ids: selectedChunkIds,
        expected_document_ids: Array.from(
          new Set(selectedHits.map((hit) => hit.document.id)),
        ),
        notes: `Saved from run ${trace.id.slice(0, 8)}.`,
      });
    },
  });
  const hits = trace.retrieval?.hits ?? [];

  return (
    <section className={styles.qualityPanel}>
      <div className={styles.panelHeading}>
        <div>
          <h2>Add to Quality</h2>
          <p>
            Record the evidence this question should retrieve in future tests.
          </p>
        </div>
        <button
          className={styles.secondaryButton}
          type="button"
          onClick={() => setOpen((current) => !current)}
        >
          <Save aria-hidden="true" size={15} />
          {open ? "Close" : "Choose evidence"}
        </button>
      </div>

      {open ? (
        <div className={styles.qualityForm}>
          <label>
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
          <div className={styles.hitOptions}>
            {hits.slice(0, 5).map((hit) => (
              <label className={styles.hitOption} key={hit.chunk.id}>
                <input
                  checked={selectedChunkIds.includes(hit.chunk.id)}
                  type="checkbox"
                  onChange={() => toggleChunk(hit.chunk.id)}
                />
                <span>
                  <strong>
                    #{hit.rank} {hit.document.path}
                  </strong>
                  <small>{hit.snippet}</small>
                </span>
              </label>
            ))}
          </div>
          <button
            className={styles.primaryButton}
            disabled={
              !datasetId ||
              selectedChunkIds.length === 0 ||
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
          {saveMutation.isSuccess ? (
            <p className={styles.message}>Quality case saved.</p>
          ) : null}
          {saveMutation.isError ? (
            <p className={styles.errorMessage} role="alert">
              {requestErrorMessage(saveMutation.error)}
            </p>
          ) : null}
        </div>
      ) : null}
    </section>
  );

  function toggleChunk(chunkId: string) {
    setSelectedChunkIds((current) =>
      current.includes(chunkId)
        ? current.filter((id) => id !== chunkId)
        : [...current, chunkId],
    );
  }
}
