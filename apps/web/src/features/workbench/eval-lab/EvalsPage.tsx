import {
  AlertCircle,
  CheckCircle2,
  Database,
  FileText,
  FlaskConical,
  GitCompare,
  Loader2,
  Plus,
  Save,
  Trash2,
  XCircle,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import { ProgressBar } from "../../../components/dashboard/ProgressBar";
import {
  createEvalLabCase,
  createEvalLabDataset,
  deleteEvalLabCase,
  getEvalLabDataset,
  listEvalLabDatasets,
  listEvalLabExperiments,
  runEvalLabExperiment,
  updateEvalLabCase,
  type RetrievalEvalCase,
  type RetrievalEvalDataset,
  type RetrievalEvalDatasetSummary,
  type RetrievalEvalExperiment,
  type RetrievalEvalFailure,
  type RetrievalEvalModeResult,
} from "../../../lib/api/evalLab";
import type { RetrievalMode } from "../../../lib/api/retrieval";
import {
  listDocumentChunks,
  listSources,
  type ChunkPreview,
  type DocumentSummary,
  type SourceSummary,
} from "../../../lib/api/sources";
import styles from "./EvalsPage.module.css";

const MODES: RetrievalMode[] = ["hybrid", "vector", "lexical"];

export function EvalsPage() {
  const [datasets, setDatasets] = useState<RetrievalEvalDatasetSummary[]>([]);
  const [selectedDataset, setSelectedDataset] =
    useState<RetrievalEvalDataset | null>(null);
  const [experiments, setExperiments] = useState<RetrievalEvalExperiment[]>([]);
  const [sources, setSources] = useState<SourceSummary[]>([]);
  const [chunks, setChunks] = useState<ChunkPreview[]>([]);
  const [datasetName, setDatasetName] = useState("");
  const [datasetDescription, setDatasetDescription] = useState("");
  const [caseName, setCaseName] = useState("");
  const [caseQuery, setCaseQuery] = useState("");
  const [caseNotes, setCaseNotes] = useState("");
  const [topK, setTopK] = useState(5);
  const [expectedDocumentIds, setExpectedDocumentIds] = useState<string[]>([]);
  const [expectedChunkIds, setExpectedChunkIds] = useState<string[]>([]);
  const [selectedDocumentId, setSelectedDocumentId] = useState("");
  const [editingCaseId, setEditingCaseId] = useState<string | null>(null);
  const [experimentName, setExperimentName] = useState("");
  const [selectedModes, setSelectedModes] = useState<RetrievalMode[]>([
    "hybrid",
    "vector",
    "lexical",
  ]);
  const [isLoading, setIsLoading] = useState(true);
  const [isSavingDataset, setIsSavingDataset] = useState(false);
  const [isSavingCase, setIsSavingCase] = useState(false);
  const [isRunning, setIsRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    Promise.all([
      listEvalLabDatasets(controller.signal),
      listEvalLabExperiments(controller.signal),
      listSources(controller.signal),
    ])
      .then(([nextDatasets, nextExperiments, nextSources]) => {
        setDatasets(nextDatasets);
        setExperiments(nextExperiments);
        setSources(nextSources);
        const firstDatasetId = nextDatasets[0]?.id;
        if (firstDatasetId) {
          return getEvalLabDataset(firstDatasetId, controller.signal).then(
            setSelectedDataset,
          );
        }
        return undefined;
      })
      .catch((cause: unknown) => {
        if (!controller.signal.aborted) {
          setError(errorMessage(cause));
        }
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setIsLoading(false);
        }
      });
    return () => controller.abort();
  }, []);

  useEffect(() => {
    if (!selectedDocumentId) {
      return;
    }

    const controller = new AbortController();
    listDocumentChunks(selectedDocumentId, controller.signal)
      .then(setChunks)
      .catch((cause: unknown) => {
        if (!controller.signal.aborted) {
          setError(errorMessage(cause));
        }
      });
    return () => controller.abort();
  }, [selectedDocumentId]);

  const documents = useMemo(
    () => sources.flatMap((source) => source.documents),
    [sources],
  );
  const latestExperiment = experiments[0] ?? null;

  async function refreshDataset(datasetId: string) {
    setSelectedDataset(await getEvalLabDataset(datasetId));
    setDatasets(await listEvalLabDatasets());
  }

  async function handleCreateDataset() {
    if (!datasetName.trim() || isSavingDataset) {
      return;
    }

    setIsSavingDataset(true);
    setError(null);
    try {
      const dataset = await createEvalLabDataset({
        name: datasetName.trim(),
        description: datasetDescription.trim() || null,
      });
      setDatasetName("");
      setDatasetDescription("");
      setDatasets(await listEvalLabDatasets());
      setSelectedDataset(await getEvalLabDataset(dataset.id));
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setIsSavingDataset(false);
    }
  }

  async function handleSelectDataset(datasetId: string) {
    setError(null);
    try {
      setSelectedDataset(await getEvalLabDataset(datasetId));
      clearCaseForm();
    } catch (cause) {
      setError(errorMessage(cause));
    }
  }

  async function handleSaveCase() {
    if (!selectedDataset || !caseQuery.trim() || isSavingCase) {
      return;
    }
    if (expectedChunkIds.length === 0 && expectedDocumentIds.length === 0) {
      setError("Select at least one expected document or chunk.");
      return;
    }

    setIsSavingCase(true);
    setError(null);
    try {
      const body = {
        name: caseName.trim() || caseQuery.trim(),
        query: caseQuery.trim(),
        top_k: topK,
        expected_chunk_ids: expectedChunkIds,
        expected_document_ids: expectedDocumentIds,
        notes: caseNotes.trim() || null,
      };
      if (editingCaseId) {
        await updateEvalLabCase(editingCaseId, body);
      } else {
        await createEvalLabCase(selectedDataset.id, body);
      }
      clearCaseForm();
      await refreshDataset(selectedDataset.id);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setIsSavingCase(false);
    }
  }

  async function handleDeleteCase(caseId: string) {
    if (!selectedDataset) {
      return;
    }
    setError(null);
    try {
      await deleteEvalLabCase(caseId);
      await refreshDataset(selectedDataset.id);
    } catch (cause) {
      setError(errorMessage(cause));
    }
  }

  async function handleRunExperiment() {
    if (!selectedDataset || selectedModes.length === 0 || isRunning) {
      return;
    }

    setIsRunning(true);
    setError(null);
    try {
      const experiment = await runEvalLabExperiment({
        dataset_id: selectedDataset.id,
        name: experimentName.trim() || `${selectedDataset.name} comparison`,
        modes: selectedModes,
        top_k: topK,
      });
      setExperimentName("");
      setExperiments(
        [experiment, ...(await listEvalLabExperiments())].filter(uniqueById),
      );
      setDatasets(await listEvalLabDatasets());
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setIsRunning(false);
    }
  }

  function startEditingCase(evalCase: RetrievalEvalCase) {
    setEditingCaseId(evalCase.id);
    setCaseName(evalCase.name);
    setCaseQuery(evalCase.query);
    setCaseNotes(evalCase.notes ?? "");
    setTopK(evalCase.top_k);
    setExpectedChunkIds(evalCase.expected_chunk_ids);
    setExpectedDocumentIds(evalCase.expected_document_ids);
  }

  function clearCaseForm() {
    setEditingCaseId(null);
    setCaseName("");
    setCaseQuery("");
    setCaseNotes("");
    setTopK(5);
    setExpectedChunkIds([]);
    setExpectedDocumentIds([]);
    setSelectedDocumentId("");
    setChunks([]);
  }

  function handleDocumentSelect(documentId: string) {
    setChunks([]);
    setSelectedDocumentId(documentId);
  }

  return (
    <section className={styles.page} aria-labelledby="evals-title">
      <header className={styles.header}>
        <div>
          <p>Quality control</p>
          <h1 id="evals-title">Eval Lab</h1>
          <span>
            Manage golden datasets, compare retrieval modes, diagnose failures,
            and gate corpus changes before they reach production.
          </span>
        </div>
        <div className={styles.headerMetrics}>
          <Metric label="Datasets" value={String(datasets.length)} />
          <Metric
            label="Cases"
            value={String(selectedDataset?.cases.length ?? 0)}
          />
          <Metric
            label="Latest gate"
            value={latestExperiment ? latestExperiment.gate.status : "--"}
          />
        </div>
      </header>

      {error ? (
        <div className={styles.alert} role="alert">
          <AlertCircle aria-hidden="true" size={18} />
          <span>{error}</span>
        </div>
      ) : null}

      {isLoading ? (
        <div className={styles.loading}>Loading Eval Lab...</div>
      ) : null}

      <div className={styles.layout}>
        <section className={styles.panel} aria-labelledby="datasets-title">
          <PanelTitle
            eyebrow="Datasets"
            title="Golden retrieval sets"
            badge={`${datasets.length} total`}
          />
          <div className={styles.datasetList}>
            {datasets.map((dataset) => (
              <button
                className={
                  selectedDataset?.id === dataset.id
                    ? styles.selectedDataset
                    : styles.datasetButton
                }
                key={dataset.id}
                type="button"
                onClick={() => void handleSelectDataset(dataset.id)}
              >
                <strong>{dataset.name}</strong>
                <span>{dataset.case_count} cases</span>
                <small>
                  {dataset.latest_gate
                    ? `${dataset.latest_gate.status} gate`
                    : "no experiments"}
                </small>
              </button>
            ))}
          </div>

          <div className={styles.formStack}>
            <label>
              Dataset name
              <input
                value={datasetName}
                onChange={(event) => setDatasetName(event.currentTarget.value)}
                placeholder="Production support QA"
              />
            </label>
            <label>
              Description
              <input
                value={datasetDescription}
                onChange={(event) =>
                  setDatasetDescription(event.currentTarget.value)
                }
                placeholder="High-value retrieval questions"
              />
            </label>
            <button
              className={styles.primaryButton}
              disabled={!datasetName.trim() || isSavingDataset}
              type="button"
              onClick={() => void handleCreateDataset()}
            >
              {isSavingDataset ? (
                <Loader2 className="spin" size={16} />
              ) : (
                <Plus size={16} />
              )}
              Create dataset
            </button>
          </div>
        </section>

        <section className={styles.panel} aria-labelledby="cases-title">
          <PanelTitle
            eyebrow="Cases"
            title={selectedDataset?.name ?? "Select a dataset"}
            badge={`${selectedDataset?.cases.length ?? 0} cases`}
          />
          <div className={styles.caseList}>
            {selectedDataset?.cases.length ? (
              selectedDataset.cases.map((evalCase) => (
                <article className={styles.caseRow} key={evalCase.id}>
                  <button
                    type="button"
                    onClick={() => startEditingCase(evalCase)}
                  >
                    <strong>{evalCase.name}</strong>
                    <span>{evalCase.query}</span>
                    <small>
                      top {evalCase.top_k} ·{" "}
                      {evalCase.expected_chunk_ids.length} chunks ·{" "}
                      {evalCase.expected_document_ids.length} docs
                    </small>
                  </button>
                  <button
                    aria-label={`Delete ${evalCase.name}`}
                    className={styles.iconButton}
                    type="button"
                    onClick={() => void handleDeleteCase(evalCase.id)}
                  >
                    <Trash2 aria-hidden="true" size={15} />
                  </button>
                </article>
              ))
            ) : (
              <div className={styles.emptyState}>
                <FlaskConical aria-hidden="true" size={22} />
                <strong>No eval cases yet</strong>
                <span>
                  Select expected documents or chunks, then save a case.
                </span>
              </div>
            )}
          </div>
        </section>
      </div>

      <section className={styles.panel} aria-labelledby="case-editor-title">
        <PanelTitle
          eyebrow="Case editor"
          title={
            editingCaseId
              ? "Edit expected evidence"
              : "Create expected evidence"
          }
          badge="document and chunk picker"
        />
        <div className={styles.caseEditor}>
          <div className={styles.formStack}>
            <label>
              Case name
              <input
                value={caseName}
                onChange={(event) => setCaseName(event.currentTarget.value)}
                placeholder="GPU indexing evidence"
              />
            </label>
            <label>
              Query
              <textarea
                value={caseQuery}
                onChange={(event) => setCaseQuery(event.currentTarget.value)}
                placeholder="Which evidence explains GPU indexing workers?"
              />
            </label>
            <div className={styles.inlineFields}>
              <label>
                Top K
                <input
                  min={1}
                  max={25}
                  type="number"
                  value={topK}
                  onChange={(event) =>
                    setTopK(Number(event.currentTarget.value))
                  }
                />
              </label>
              <label>
                Notes
                <input
                  value={caseNotes}
                  onChange={(event) => setCaseNotes(event.currentTarget.value)}
                  placeholder="Why this case matters"
                />
              </label>
            </div>
          </div>

          <EvidencePicker
            chunks={chunks}
            documents={documents}
            expectedChunkIds={expectedChunkIds}
            expectedDocumentIds={expectedDocumentIds}
            selectedDocumentId={selectedDocumentId}
            onDocumentSelect={handleDocumentSelect}
            onToggleChunk={(chunkId) =>
              setExpectedChunkIds((current) => toggleId(current, chunkId))
            }
            onToggleExpectedDocument={(documentId) =>
              setExpectedDocumentIds((current) => toggleId(current, documentId))
            }
          />
        </div>
        <div className={styles.buttonRow}>
          <button
            className={styles.primaryButton}
            disabled={!selectedDataset || !caseQuery.trim() || isSavingCase}
            type="button"
            onClick={() => void handleSaveCase()}
          >
            {isSavingCase ? (
              <Loader2 className="spin" size={16} />
            ) : (
              <Save size={16} />
            )}
            {editingCaseId ? "Update case" : "Save case"}
          </button>
          {editingCaseId ? (
            <button
              className={styles.secondaryButton}
              type="button"
              onClick={clearCaseForm}
            >
              Cancel edit
            </button>
          ) : null}
        </div>
      </section>

      <section className={styles.panel} aria-labelledby="run-lab-title">
        <PanelTitle
          eyebrow="Run Lab"
          title="Compare retrieval modes"
          badge="lexical · vector · hybrid"
        />
        <div className={styles.runGrid}>
          <label>
            Experiment name
            <input
              value={experimentName}
              onChange={(event) => setExperimentName(event.currentTarget.value)}
              placeholder="June release retrieval gate"
            />
          </label>
          <div className={styles.modeChecks}>
            {MODES.map((mode) => (
              <label key={mode}>
                <input
                  checked={selectedModes.includes(mode)}
                  type="checkbox"
                  onChange={() =>
                    setSelectedModes((current) => toggleMode(current, mode))
                  }
                />
                {mode}
              </label>
            ))}
          </div>
          <button
            className={styles.primaryButton}
            disabled={
              !selectedDataset ||
              selectedDataset.cases.length === 0 ||
              selectedModes.length === 0 ||
              isRunning
            }
            type="button"
            onClick={() => void handleRunExperiment()}
          >
            {isRunning ? (
              <Loader2 className="spin" size={16} />
            ) : (
              <GitCompare size={16} />
            )}
            Run experiment
          </button>
        </div>
      </section>

      {latestExperiment ? (
        <ExperimentView experiment={latestExperiment} />
      ) : (
        <section className={styles.panel}>
          <div className={styles.emptyState}>
            <Database aria-hidden="true" size={22} />
            <strong>No experiments yet</strong>
            <span>
              Run a dataset across retrieval modes to see gates and failures.
            </span>
          </div>
        </section>
      )}
    </section>
  );
}

function EvidencePicker({
  documents,
  chunks,
  expectedDocumentIds,
  expectedChunkIds,
  selectedDocumentId,
  onDocumentSelect,
  onToggleExpectedDocument,
  onToggleChunk,
}: {
  documents: DocumentSummary[];
  chunks: ChunkPreview[];
  expectedDocumentIds: string[];
  expectedChunkIds: string[];
  selectedDocumentId: string;
  onDocumentSelect: (documentId: string) => void;
  onToggleExpectedDocument: (documentId: string) => void;
  onToggleChunk: (chunkId: string) => void;
}) {
  return (
    <div className={styles.evidencePicker}>
      <div>
        <strong>Expected documents</strong>
        <div className={styles.pickerList}>
          {documents.length === 0 ? (
            <span>No documents ingested yet.</span>
          ) : (
            documents.map((item) => (
              <label key={item.document.id}>
                <input
                  checked={expectedDocumentIds.includes(item.document.id)}
                  type="checkbox"
                  onChange={() => onToggleExpectedDocument(item.document.id)}
                />
                <button
                  type="button"
                  onClick={() => onDocumentSelect(item.document.id)}
                >
                  <FileText aria-hidden="true" size={14} />
                  {item.document.path}
                </button>
              </label>
            ))
          )}
        </div>
      </div>
      <div>
        <strong>Expected chunks</strong>
        <div className={styles.pickerList}>
          {!selectedDocumentId ? (
            <span>Select a document to inspect chunks.</span>
          ) : chunks.length === 0 ? (
            <span>No chunks found for this document.</span>
          ) : (
            chunks.map((chunk) => (
              <label key={chunk.id}>
                <input
                  checked={expectedChunkIds.includes(chunk.id)}
                  type="checkbox"
                  onChange={() => onToggleChunk(chunk.id)}
                />
                <span>
                  #{chunk.ordinal + 1} {chunk.section_title ?? "Untitled"} ·{" "}
                  {chunk.token_count} tokens
                </span>
              </label>
            ))
          )}
        </div>
      </div>
    </div>
  );
}

function ExperimentView({
  experiment,
}: {
  experiment: RetrievalEvalExperiment;
}) {
  return (
    <section className={styles.panel} aria-labelledby="experiment-title">
      <PanelTitle
        eyebrow="Experiments"
        title={experiment.name}
        badge={`${experiment.mode_results.length} modes`}
      />
      <div className={styles.gateBand}>
        {experiment.gate.status === "passed" ? (
          <CheckCircle2 aria-hidden="true" size={22} />
        ) : (
          <XCircle aria-hidden="true" size={22} />
        )}
        <div>
          <strong>
            {experiment.gate.status === "passed"
              ? "Gate passed"
              : "Gate failed"}
          </strong>
          <span>{experiment.comparison.summary}</span>
        </div>
        <small>{experiment.failures.length} failures</small>
      </div>
      <div className={styles.matrix}>
        {experiment.mode_results.map((result) => (
          <ModeResultCard key={result.retrieval_mode} result={result} />
        ))}
      </div>
      <div className={styles.failurePanel}>
        <h3>Failure diagnosis</h3>
        {experiment.failures.length === 0 ? (
          <p>No deterministic eval failures were found.</p>
        ) : (
          experiment.failures
            .slice(0, 8)
            .map((failure) => (
              <FailureRow
                failure={failure}
                key={`${failure.case_id}-${failure.label}-${failure.retrieval_mode}`}
              />
            ))
        )}
      </div>
    </section>
  );
}

function ModeResultCard({ result }: { result: RetrievalEvalModeResult }) {
  return (
    <article className={styles.modeCard}>
      <header>
        <strong>{result.retrieval_mode}</strong>
        <span>
          {result.passed_count}/{result.case_count} passed
        </span>
      </header>
      <Metric
        label="Recall"
        value={`${Math.round(result.average_recall_at_k * 100)}%`}
      />
      <ProgressBar value={result.average_recall_at_k} tone="good" />
      <Metric
        label="Precision"
        value={`${Math.round(result.average_precision_at_k * 100)}%`}
      />
      <ProgressBar value={result.average_precision_at_k} tone="neutral" />
      <div className={styles.modeStats}>
        <span>MRR {result.mean_reciprocal_rank.toFixed(2)}</span>
        <span>p95 {result.latency_p95_ms} ms</span>
        <span>{result.weak_evidence_count} weak</span>
      </div>
    </article>
  );
}

function FailureRow({ failure }: { failure: RetrievalEvalFailure }) {
  return (
    <article className={`${styles.failureRow} ${styles[failure.severity]}`}>
      <strong>{failure.label.replaceAll("_", " ")}</strong>
      <span>{failure.message}</span>
      <small>
        {failure.retrieval_mode} · rank {failure.top_hit_rank ?? "none"}
      </small>
    </article>
  );
}

function PanelTitle({
  eyebrow,
  title,
  badge,
}: {
  eyebrow: string;
  title: string;
  badge?: string;
}) {
  return (
    <div className={styles.panelTitle}>
      <div>
        <p>{eyebrow}</p>
        <h2>{title}</h2>
      </div>
      {badge ? <span>{badge}</span> : null}
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <span className={styles.metric}>
      <small>{label}</small>
      <strong>{value}</strong>
    </span>
  );
}

function toggleId(values: string[], id: string) {
  return values.includes(id)
    ? values.filter((value) => value !== id)
    : [...values, id];
}

function toggleMode(values: RetrievalMode[], mode: RetrievalMode) {
  return values.includes(mode)
    ? values.filter((value) => value !== mode)
    : [...values, mode];
}

function uniqueById(
  value: RetrievalEvalExperiment,
  index: number,
  values: RetrievalEvalExperiment[],
) {
  return values.findIndex((candidate) => candidate.id === value.id) === index;
}

function errorMessage(cause: unknown) {
  return cause instanceof Error ? cause.message : "Unexpected request failure";
}
