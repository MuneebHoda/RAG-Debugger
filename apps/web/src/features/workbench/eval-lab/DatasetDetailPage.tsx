import {
  AlertTriangle,
  ArrowLeft,
  FlaskConical,
  Loader2,
  Pencil,
  Plus,
  Trash2,
  X,
} from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";

import {
  createEvalLabCase,
  deleteEvalLabCase,
  getEvalLabDataset,
  runEvalLabExperiment,
  updateEvalLabCase,
  type RetrievalEvalCase,
} from "../../../lib/api/evalLab";
import type { RetrievalMode } from "../../../lib/api/retrieval";
import {
  listDocumentChunks,
  listSources,
  type ChunkPreview,
} from "../../../lib/api/sources";
import { formatDateTime } from "../../../lib/dateTime";
import styles from "./QualityPage.module.css";

const retrievalModes: RetrievalMode[] = ["lexical", "vector", "hybrid"];

export function DatasetDetailPage() {
  const { datasetId } = useParams<{ datasetId: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [caseFormOpen, setCaseFormOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [caseName, setCaseName] = useState("");
  const [notes, setNotes] = useState("");
  const [documentId, setDocumentId] = useState("");
  const [chunkId, setChunkId] = useState("");
  const [experimentName, setExperimentName] = useState("");
  const [topK, setTopK] = useState(5);
  const [modes, setModes] = useState<RetrievalMode[]>(["hybrid"]);

  const datasetQuery = useQuery({
    queryKey: ["eval-dataset", datasetId],
    queryFn: ({ signal }) => getEvalLabDataset(datasetId!, signal),
    enabled: Boolean(datasetId),
  });
  const sourcesQuery = useQuery({
    queryKey: ["sources"],
    queryFn: ({ signal }) => listSources(signal),
  });
  const chunksQuery = useQuery({
    queryKey: ["document-chunks", documentId],
    queryFn: ({ signal }) => listDocumentChunks(documentId, signal),
    enabled: Boolean(documentId),
  });
  const documents = useMemo(
    () =>
      (sourcesQuery.data ?? []).flatMap((source) =>
        source.documents.map((entry) => entry.document),
      ),
    [sourcesQuery.data],
  );

  const createCaseMutation = useMutation({
    mutationFn: () =>
      createEvalLabCase(datasetId!, {
        name: caseName.trim() || query.trim(),
        query: query.trim(),
        top_k: topK,
        expected_document_ids: documentId ? [documentId] : [],
        expected_chunk_ids: chunkId ? [chunkId] : [],
        notes: notes.trim() || null,
      }),
    onSuccess: () => {
      resetCaseForm();
      void queryClient.invalidateQueries({
        queryKey: ["eval-dataset", datasetId],
      });
      void queryClient.invalidateQueries({ queryKey: ["eval-datasets"] });
    },
  });
  const experimentMutation = useMutation({
    mutationFn: () =>
      runEvalLabExperiment({
        dataset_id: datasetId!,
        name: experimentName.trim() || undefined,
        modes,
        top_k: topK,
      }),
    onSuccess: (experiment) => {
      void queryClient.invalidateQueries({ queryKey: ["eval-datasets"] });
      void queryClient.invalidateQueries({ queryKey: ["eval-experiments"] });
      navigate(`/app/evals/experiments/${experiment.id}`);
    },
  });

  if (datasetQuery.isLoading) {
    return <div className={styles.empty}>Loading quality dataset…</div>;
  }

  if (datasetQuery.isError || !datasetQuery.data) {
    return (
      <section className={styles.errorState} role="alert">
        <AlertTriangle aria-hidden="true" size={24} />
        <strong>This quality dataset could not be opened.</strong>
        <button type="button" onClick={() => void datasetQuery.refetch()}>
          Retry
        </button>
        <Link className={styles.secondaryButton} to="/app/evals">
          Back to Quality
        </Link>
      </section>
    );
  }

  const dataset = datasetQuery.data;
  return (
    <section className={styles.page} aria-labelledby="dataset-title">
      <Link className={styles.backLink} to="/app/evals">
        <ArrowLeft aria-hidden="true" size={15} /> Back to Quality
      </Link>

      <header className={styles.header}>
        <div>
          <p>Quality dataset</p>
          <h1 id="dataset-title">{dataset.name}</h1>
          <span>
            {dataset.description ?? "Questions with expected evidence."}
          </span>
        </div>
        <button
          className={styles.headerButton}
          type="button"
          onClick={() => setCaseFormOpen((current) => !current)}
        >
          {caseFormOpen ? (
            <X aria-hidden="true" size={16} />
          ) : (
            <Plus aria-hidden="true" size={16} />
          )}
          {caseFormOpen ? "Close" : "Add case"}
        </button>
      </header>

      <section className={styles.stats} aria-label="Dataset summary">
        <Stat label="Cases" value={String(dataset.cases.length)} />
        <Stat label="Created" value={formatDateTime(dataset.created_at)} />
        <Stat label="Updated" value={formatDateTime(dataset.updated_at)} />
        <Stat label="Expected evidence" value="Required" />
      </section>

      {caseFormOpen ? (
        <section className={styles.panel} aria-labelledby="new-case-title">
          <div className={styles.panelHeading}>
            <div>
              <h2 id="new-case-title">Add an important question</h2>
              <p>
                Choose the document and chunk that a good retrieval run must
                find.
              </p>
            </div>
          </div>
          <div className={styles.form}>
            <div className={styles.formGrid}>
              <label>
                Question
                <input
                  value={query}
                  onChange={(event) => setQuery(event.currentTarget.value)}
                  placeholder="What should this corpus answer?"
                />
              </label>
              <label>
                Case name <small>Optional</small>
                <input
                  value={caseName}
                  onChange={(event) => setCaseName(event.currentTarget.value)}
                />
              </label>
              <label>
                Expected document
                <select
                  value={documentId}
                  onChange={(event) => {
                    setDocumentId(event.currentTarget.value);
                    setChunkId("");
                  }}
                >
                  <option value="">Choose a document</option>
                  {documents.map((document) => (
                    <option key={document.id} value={document.id}>
                      {document.path}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                Expected chunk
                <select
                  disabled={!documentId || chunksQuery.isLoading}
                  value={chunkId}
                  onChange={(event) => setChunkId(event.currentTarget.value)}
                >
                  <option value="">Choose a chunk</option>
                  {(chunksQuery.data ?? []).map((chunk) => (
                    <option key={chunk.id} value={chunk.id}>
                      Chunk {chunk.ordinal + 1}: {chunkLabel(chunk)}
                    </option>
                  ))}
                </select>
              </label>
            </div>
            <label>
              Notes <small>Optional</small>
              <textarea
                value={notes}
                onChange={(event) => setNotes(event.currentTarget.value)}
              />
            </label>
            <button
              className={styles.primaryButton}
              disabled={
                !query.trim() ||
                !documentId ||
                !chunkId ||
                createCaseMutation.isPending
              }
              type="button"
              onClick={() => createCaseMutation.mutate()}
            >
              {createCaseMutation.isPending ? (
                <Loader2 aria-hidden="true" className="spin" size={16} />
              ) : (
                <Plus aria-hidden="true" size={16} />
              )}
              Save quality case
            </button>
            {createCaseMutation.isError ? (
              <p className={styles.error} role="alert">
                {errorMessage(createCaseMutation.error)}
              </p>
            ) : null}
          </div>
        </section>
      ) : null}

      <div className={styles.grid}>
        <section className={styles.panel}>
          <div className={styles.panelHeading}>
            <div className={styles.panelHeadingCopy}>
              <h2>Cases</h2>
              <p>The retrieval behavior this dataset protects.</p>
            </div>
          </div>
          <div className={styles.list}>
            {dataset.cases.map((evalCase) => (
              <EditableCase
                datasetId={dataset.id}
                evalCase={evalCase}
                key={evalCase.id}
              />
            ))}
            {dataset.cases.length === 0 ? (
              <p className={styles.empty}>
                No cases yet. Add the first question and its expected evidence.
              </p>
            ) : null}
          </div>
        </section>

        <section className={`${styles.panel} ${styles.experimentPanel}`}>
          <div className={`${styles.panelHeading} ${styles.experimentHeading}`}>
            <div className={styles.panelHeadingCopy}>
              <h2>Run an experiment</h2>
              <p>Test the same cases across selected retrieval modes.</p>
            </div>
            <FlaskConical aria-hidden="true" size={18} />
          </div>
          <div className={`${styles.form} ${styles.experimentForm}`}>
            <label>
              Experiment name <small>Optional</small>
              <input
                value={experimentName}
                onChange={(event) =>
                  setExperimentName(event.currentTarget.value)
                }
                placeholder="Release retrieval gate"
              />
            </label>
            <div className={styles.checkboxGrid} aria-label="Retrieval modes">
              {retrievalModes.map((mode) => (
                <label className={styles.checkboxLabel} key={mode}>
                  <input
                    checked={modes.includes(mode)}
                    type="checkbox"
                    onChange={() =>
                      setModes((current) =>
                        current.includes(mode)
                          ? current.filter((item) => item !== mode)
                          : [...current, mode],
                      )
                    }
                  />
                  {mode}
                </label>
              ))}
            </div>
            <label>
              Results per question
              <input
                max={25}
                min={1}
                type="number"
                value={topK}
                onChange={(event) => setTopK(Number(event.currentTarget.value))}
              />
            </label>
            <button
              className={`${styles.primaryButton} ${styles.experimentAction}`}
              disabled={
                dataset.cases.length === 0 ||
                modes.length === 0 ||
                experimentMutation.isPending
              }
              type="button"
              onClick={() => experimentMutation.mutate()}
            >
              {experimentMutation.isPending ? (
                <Loader2 aria-hidden="true" className="spin" size={16} />
              ) : (
                <FlaskConical aria-hidden="true" size={16} />
              )}
              Run experiment
            </button>
            {experimentMutation.isError ? (
              <p className={styles.error} role="alert">
                {errorMessage(experimentMutation.error)}
              </p>
            ) : null}
          </div>
        </section>
      </div>
    </section>
  );

  function resetCaseForm() {
    setCaseFormOpen(false);
    setQuery("");
    setCaseName("");
    setNotes("");
    setDocumentId("");
    setChunkId("");
  }
}

function EditableCase({
  datasetId,
  evalCase,
}: {
  datasetId: string;
  evalCase: RetrievalEvalCase;
}) {
  const queryClient = useQueryClient();
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(evalCase.name);
  const [query, setQuery] = useState(evalCase.query);
  const updateMutation = useMutation({
    mutationFn: () =>
      updateEvalLabCase(evalCase.id, {
        name: name.trim(),
        query: query.trim(),
      }),
    onSuccess: () => {
      setEditing(false);
      void queryClient.invalidateQueries({
        queryKey: ["eval-dataset", datasetId],
      });
    },
  });
  const deleteMutation = useMutation({
    mutationFn: () => deleteEvalLabCase(evalCase.id),
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: ["eval-dataset", datasetId],
      });
      void queryClient.invalidateQueries({ queryKey: ["eval-datasets"] });
    },
  });

  return (
    <article className={styles.caseCard}>
      {editing ? (
        <div className={styles.form}>
          <label>
            Case name
            <input
              value={name}
              onChange={(event) => setName(event.currentTarget.value)}
            />
          </label>
          <label>
            Question
            <textarea
              value={query}
              onChange={(event) => setQuery(event.currentTarget.value)}
            />
          </label>
          <div className={styles.inlineActions}>
            <button
              className={styles.primaryButton}
              disabled={
                !name.trim() || !query.trim() || updateMutation.isPending
              }
              type="button"
              onClick={() => updateMutation.mutate()}
            >
              Save changes
            </button>
            <button
              className={styles.secondaryButton}
              type="button"
              onClick={() => setEditing(false)}
            >
              Cancel
            </button>
          </div>
        </div>
      ) : (
        <>
          <div className={styles.caseHeader}>
            <div>
              <strong>{evalCase.name}</strong>
              <p>{evalCase.query}</p>
            </div>
            <div className={styles.caseActions}>
              <button
                aria-label={`Edit ${evalCase.name}`}
                type="button"
                onClick={() => setEditing(true)}
              >
                <Pencil aria-hidden="true" size={14} />
              </button>
              <button
                aria-label={`Delete ${evalCase.name}`}
                disabled={deleteMutation.isPending}
                type="button"
                onClick={() => deleteMutation.mutate()}
              >
                <Trash2 aria-hidden="true" size={14} />
              </button>
            </div>
          </div>
          <small>
            Top {evalCase.top_k} · {evalCase.expected_document_ids.length}{" "}
            expected document · {evalCase.expected_chunk_ids.length} expected
            chunk
          </small>
          {evalCase.notes ? <small>{evalCase.notes}</small> : null}
        </>
      )}
    </article>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className={styles.stat}>
      <small>{label}</small>
      <strong>{value}</strong>
    </div>
  );
}

function chunkLabel(chunk: ChunkPreview) {
  return (chunk.section_title ?? chunk.text).slice(0, 64);
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "Request failed";
}
