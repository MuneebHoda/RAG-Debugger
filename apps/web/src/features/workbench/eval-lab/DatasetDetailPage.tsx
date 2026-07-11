import {
  AlertTriangle,
  FlaskConical,
  Loader2,
  Pencil,
  Plus,
  Trash2,
  X,
} from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";

import { WorkbenchPageHeader } from "../../../components/workbench/WorkbenchPageHeader";
import {
  createEvalLabCase,
  deleteEvalLabCase,
  getEvalLabDatasetTrends,
  getEvalLabDataset,
  listEvalLabDatasetExperiments,
  runEvalLabExperiment,
  updateEvalLabCase,
  type RetrievalEvalCase,
} from "../../../lib/api/evalLab";
import type { RetrievalMode } from "../../../lib/api/retrieval";
import { formatDateTime } from "../../../lib/dateTime";
import {
  ExperimentHistoryPanel,
  TrendSummaryPanel,
} from "./components/QualityViews";
import { EvidencePicker } from "./evidence/EvidencePicker";
import { EvidenceSelectionReview } from "./evidence/EvidenceSelectionReview";
import { EvidenceStateList } from "./evidence/EvidenceStateList";
import {
  emptyEvidenceSelection,
  hasExpectedEvidence,
  normalizeEvidenceSelection,
  selectionFromCase,
  type EvidenceSelection,
} from "./evidence/evidenceSelection";
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
  const [evidenceSelection, setEvidenceSelection] = useState<EvidenceSelection>(
    () => emptyEvidenceSelection(),
  );
  const [experimentName, setExperimentName] = useState("");
  const [topK, setTopK] = useState(5);
  const [modes, setModes] = useState<RetrievalMode[]>(["hybrid"]);

  const datasetQuery = useQuery({
    queryKey: ["eval-dataset", datasetId],
    queryFn: ({ signal }) => getEvalLabDataset(datasetId!, signal),
    enabled: Boolean(datasetId),
  });
  const datasetExperimentsQuery = useQuery({
    queryKey: ["eval-dataset-experiments", datasetId],
    queryFn: ({ signal }) => listEvalLabDatasetExperiments(datasetId!, signal),
    enabled: Boolean(datasetId),
  });
  const trendQuery = useQuery({
    queryKey: ["eval-dataset-trends", datasetId],
    queryFn: ({ signal }) => getEvalLabDatasetTrends(datasetId!, 10, signal),
    enabled: Boolean(datasetId),
  });
  const normalizedEvidenceSelection =
    normalizeEvidenceSelection(evidenceSelection);

  const createCaseMutation = useMutation({
    mutationFn: () =>
      createEvalLabCase(datasetId!, {
        name: caseName.trim() || query.trim(),
        query: query.trim(),
        top_k: topK,
        expected_document_ids: normalizedEvidenceSelection.documentIds,
        expected_chunk_ids: normalizedEvidenceSelection.chunkIds,
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
      void queryClient.invalidateQueries({
        queryKey: ["eval-dataset-experiments", datasetId],
      });
      void queryClient.invalidateQueries({
        queryKey: ["eval-dataset-trends", datasetId],
      });
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
        <Link className="secondary-button" to="/app/evals">
          Back to Quality
        </Link>
      </section>
    );
  }

  const dataset = datasetQuery.data;
  return (
    <section className={styles.page} aria-labelledby="dataset-title">
      <WorkbenchPageHeader
        actions={
          <button
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
        }
        back={{ label: "Back to Eval Lab", to: "/app/evals" }}
        description={
          dataset.description ??
          "Questions paired with the evidence a correct retrieval must find."
        }
        section="Eval dataset"
        title={dataset.name}
        titleId="dataset-title"
      />

      <section className={styles.stats} aria-label="Dataset summary">
        <Stat label="Cases" value={String(dataset.cases.length)} />
        <Stat label="Created" value={formatDateTime(dataset.created_at)} />
        <Stat label="Updated" value={formatDateTime(dataset.updated_at)} />
        <Stat label="Expected evidence" value="Required" />
      </section>

      <div className={styles.grid}>
        <TrendSummaryPanel trend={trendQuery.data} />
        <ExperimentHistoryPanel
          experiments={datasetExperimentsQuery.data ?? []}
          isLoading={datasetExperimentsQuery.isLoading}
        />
      </div>

      {caseFormOpen ? (
        <section className={styles.panel} aria-labelledby="new-case-title">
          <div className={styles.panelHeading}>
            <div>
              <h2 id="new-case-title">Add an important question</h2>
              <p>
                Search and select the evidence a good retrieval run must find.
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
            </div>
            <EvidencePicker
              query={query}
              selection={evidenceSelection}
              onSelectionChange={setEvidenceSelection}
            />
            <EvidenceSelectionReview
              selection={evidenceSelection}
              onSelectionChange={setEvidenceSelection}
            />
            <label>
              Notes <small>Optional</small>
              <textarea
                value={notes}
                onChange={(event) => setNotes(event.currentTarget.value)}
              />
            </label>
            <button
              className="primary-button"
              disabled={
                !query.trim() ||
                !hasExpectedEvidence(normalizedEvidenceSelection) ||
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
              className={`primary-button ${styles.experimentAction}`}
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
    setEvidenceSelection(emptyEvidenceSelection());
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
  const [notes, setNotes] = useState(evalCase.notes ?? "");
  const [topK, setTopK] = useState(evalCase.top_k);
  const [evidenceSelection, setEvidenceSelection] = useState<EvidenceSelection>(
    () => selectionFromCase(evalCase),
  );
  const normalizedEvidenceSelection =
    normalizeEvidenceSelection(evidenceSelection);
  const updateMutation = useMutation({
    mutationFn: () =>
      updateEvalLabCase(evalCase.id, {
        name: name.trim(),
        query: query.trim(),
        top_k: topK,
        expected_chunk_ids: normalizedEvidenceSelection.chunkIds,
        expected_document_ids: normalizedEvidenceSelection.documentIds,
        notes: notes.trim() || null,
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
          <EvidencePicker
            query={query}
            selection={evidenceSelection}
            onSelectionChange={setEvidenceSelection}
          />
          <EvidenceSelectionReview
            selection={evidenceSelection}
            onSelectionChange={setEvidenceSelection}
          />
          <label>
            Notes
            <textarea
              value={notes}
              onChange={(event) => setNotes(event.currentTarget.value)}
            />
          </label>
          <div className={styles.inlineActions}>
            <button
              className="primary-button"
              disabled={
                !name.trim() ||
                !query.trim() ||
                !hasExpectedEvidence(normalizedEvidenceSelection) ||
                updateMutation.isPending
              }
              type="button"
              onClick={() => updateMutation.mutate()}
            >
              Save changes
            </button>
            <button
              className="secondary-button"
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
          <EvidenceStateList
            selection={selectionFromCase(evalCase)}
            title="Expected evidence status"
          />
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

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "Request failed";
}
