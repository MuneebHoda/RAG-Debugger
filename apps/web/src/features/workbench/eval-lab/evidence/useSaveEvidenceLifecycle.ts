import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useRef, useState } from "react";

import {
  createEvalLabCase,
  getEvalLabDataset,
  listEvalLabDatasets,
  type CreateRetrievalEvalCaseRequest,
} from "../../../../lib/api/evalLab";
import {
  emptyEvidenceSelection,
  findSimilarCases,
  hasExpectedEvidence,
  normalizeEvidenceSelection,
  type EvidenceSelection,
} from "./evidenceSelection";

export interface SaveEvidenceSource {
  identity: string;
  query: string;
  topK: number;
  note: string;
  traceId?: string;
}

interface SourceDraft {
  sourceIdentity: string;
  caseName: string;
  notes: string;
  selection: EvidenceSelection;
}

interface SaveSubmission {
  sourceIdentity: string;
  datasetId: string;
  payload: CreateRetrievalEvalCaseRequest;
  controller: AbortController;
}

export function useSaveEvidenceLifecycle({
  open,
  source,
  sourcePending,
}: {
  open: boolean;
  source: SaveEvidenceSource;
  sourcePending: boolean;
}) {
  const queryClient = useQueryClient();
  const incomingDraft = createSourceDraft(source);
  const [draft, setDraft] = useState<SourceDraft>(incomingDraft);
  const [datasetId, setDatasetId] = useState("");
  const activeController = useRef<AbortController | null>(null);
  const submissionInFlight = useRef(false);
  const previousSourceIdentity = useRef(source.identity);
  const previousDatasetId = useRef(datasetId);
  const activeDraft =
    draft.sourceIdentity === source.identity ? draft : incomingDraft;
  const normalizedSelection = normalizeEvidenceSelection(activeDraft.selection);

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
      datasetQuery.data
        ? findSimilarCases(datasetQuery.data.cases, source.query)
        : [],
    [datasetQuery.data, source.query],
  );

  const saveMutation = useMutation({
    mutationFn: async (submission: SaveSubmission) => ({
      savedCase: await createEvalLabCase(
        submission.datasetId,
        submission.payload,
        submission.controller.signal,
      ),
      submission,
    }),
    onSuccess: ({ submission }) => {
      void queryClient.invalidateQueries({ queryKey: ["eval-datasets"] });
      void queryClient.invalidateQueries({
        queryKey: ["eval-dataset", submission.datasetId],
      });
    },
    onSettled: (_data, _error, submission) => {
      if (activeController.current === submission.controller) {
        activeController.current = null;
        submissionInFlight.current = false;
      }
    },
  });
  const resetSaveMutation = saveMutation.reset;
  const sourceIdentity = source.identity;
  const sourceNote = source.note;
  const sourceQuery = source.query;
  const sourceTopK = source.topK;

  useEffect(() => {
    if (previousSourceIdentity.current === sourceIdentity) return;

    previousSourceIdentity.current = sourceIdentity;
    activeController.current?.abort();
    activeController.current = null;
    submissionInFlight.current = false;
    setDraft(
      createSourceDraft({
        identity: sourceIdentity,
        note: sourceNote,
        query: sourceQuery,
        topK: sourceTopK,
      }),
    );
    resetSaveMutation();
  }, [resetSaveMutation, sourceIdentity, sourceNote, sourceQuery, sourceTopK]);

  useEffect(() => {
    if (previousDatasetId.current === datasetId) return;

    previousDatasetId.current = datasetId;
    activeController.current?.abort();
    activeController.current = null;
    submissionInFlight.current = false;
    resetSaveMutation();
  }, [datasetId, resetSaveMutation]);

  useEffect(
    () => () => {
      activeController.current?.abort();
    },
    [],
  );

  const feedbackMatchesCurrent =
    saveMutation.variables?.sourceIdentity === source.identity &&
    saveMutation.variables.datasetId === datasetId;
  const datasetReady = Boolean(datasetId) && datasetQuery.isSuccess;
  const canSave =
    datasetReady &&
    Boolean(source.query.trim()) &&
    hasExpectedEvidence(normalizedSelection) &&
    !sourcePending &&
    !saveMutation.isPending;

  function updateDraft(update: (current: SourceDraft) => SourceDraft): void {
    setDraft((current) =>
      update(
        current.sourceIdentity === source.identity
          ? current
          : createSourceDraft(source),
      ),
    );
  }

  function save(): void {
    if (!canSave || submissionInFlight.current) return;

    const controller = new AbortController();
    activeController.current?.abort();
    activeController.current = controller;
    submissionInFlight.current = true;
    saveMutation.mutate({
      sourceIdentity: source.identity,
      datasetId,
      controller,
      payload: {
        name: activeDraft.caseName.trim() || source.query.trim(),
        query: source.query.trim(),
        top_k: source.topK,
        expected_chunk_ids: normalizedSelection.chunkIds,
        expected_document_ids: normalizedSelection.documentIds,
        notes: activeDraft.notes.trim() || null,
        source_trace_id: source.traceId,
      },
    });
  }

  return {
    canSave,
    caseName: activeDraft.caseName,
    datasetId,
    datasetQuery,
    datasetsQuery,
    isSaving: saveMutation.isPending,
    notes: activeDraft.notes,
    normalizedSelection,
    save,
    saveError:
      feedbackMatchesCurrent && saveMutation.isError
        ? errorMessage(saveMutation.error)
        : null,
    saveSucceeded: feedbackMatchesCurrent && saveMutation.isSuccess,
    selection: activeDraft.selection,
    setCaseName: (caseName: string) =>
      updateDraft((current) => ({ ...current, caseName })),
    setDatasetId,
    setNotes: (notes: string) =>
      updateDraft((current) => ({ ...current, notes })),
    setSelection: (selection: EvidenceSelection) =>
      updateDraft((current) => ({ ...current, selection })),
    similarCases,
  };
}

function createSourceDraft(source: SaveEvidenceSource): SourceDraft {
  return {
    sourceIdentity: source.identity,
    caseName: source.query,
    notes: source.note,
    selection: emptyEvidenceSelection(),
  };
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "Request failed";
}
