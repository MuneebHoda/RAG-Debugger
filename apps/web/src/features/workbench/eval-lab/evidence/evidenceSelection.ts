import type {
  EvalLabEvidenceChunk,
  EvalLabEvidenceDocument,
  RetrievalEvalCase,
  RetrievalEvalFailureLabel,
} from "../../../../lib/api/evalLab";

export interface EvidenceSelection {
  documentIds: string[];
  chunkIds: string[];
}

export interface EvidenceHitRef {
  chunkId: string;
  documentId: string;
  label: string;
  rank?: number | null;
  path?: string | null;
  sectionTitle?: string | null;
  snippet?: string | null;
  weak?: boolean;
  duplicate?: boolean;
}

export type EvidenceStateKind =
  | "expected_retrieved"
  | "expected_missing"
  | "retrieved_not_expected"
  | "wrong_chunk"
  | "duplicate_evidence"
  | "weak_evidence"
  | "stale_evidence";

export interface EvidenceState {
  kind: EvidenceStateKind;
  label: string;
  description: string;
  severity: "success" | "warning" | "danger" | "neutral";
  evidenceId: string;
}

export interface EvidenceStateInput {
  selection: EvidenceSelection;
  documents?: EvalLabEvidenceDocument[];
  chunks?: EvalLabEvidenceChunk[];
  retrievedHits?: EvidenceHitRef[];
  unresolvedDocumentIds?: string[];
  unresolvedChunkIds?: string[];
  failureLabels?: RetrievalEvalFailureLabel[];
}

export function emptyEvidenceSelection(): EvidenceSelection {
  return { documentIds: [], chunkIds: [] };
}

export function normalizeEvidenceSelection(
  selection: EvidenceSelection,
): EvidenceSelection {
  return {
    documentIds: dedupeIds(selection.documentIds),
    chunkIds: dedupeIds(selection.chunkIds),
  };
}

export function hasExpectedEvidence(selection: EvidenceSelection): boolean {
  return selection.documentIds.length > 0 || selection.chunkIds.length > 0;
}

export function addEvidenceDocument(
  selection: EvidenceSelection,
  documentId: string,
): EvidenceSelection {
  return normalizeEvidenceSelection({
    ...selection,
    documentIds: [...selection.documentIds, documentId],
  });
}

export function addEvidenceChunk(
  selection: EvidenceSelection,
  chunkId: string,
): EvidenceSelection {
  return normalizeEvidenceSelection({
    ...selection,
    chunkIds: [...selection.chunkIds, chunkId],
  });
}

export function removeEvidenceDocument(
  selection: EvidenceSelection,
  documentId: string,
): EvidenceSelection {
  return {
    ...selection,
    documentIds: selection.documentIds.filter((id) => id !== documentId),
  };
}

export function removeEvidenceChunk(
  selection: EvidenceSelection,
  chunkId: string,
): EvidenceSelection {
  return {
    ...selection,
    chunkIds: selection.chunkIds.filter((id) => id !== chunkId),
  };
}

export function selectionFromCase(
  evalCase: RetrievalEvalCase,
): EvidenceSelection {
  return normalizeEvidenceSelection({
    documentIds: evalCase.expected_document_ids,
    chunkIds: evalCase.expected_chunk_ids,
  });
}

export function findSimilarCases(
  cases: RetrievalEvalCase[],
  query: string,
  excludeCaseId?: string,
): RetrievalEvalCase[] {
  const normalizedQuery = normalizeQuery(query);
  if (!normalizedQuery) {
    return [];
  }
  return cases.filter(
    (evalCase) =>
      evalCase.id !== excludeCaseId &&
      normalizeQuery(evalCase.query) === normalizedQuery,
  );
}

export function deriveEvidenceStates(
  input: EvidenceStateInput,
): EvidenceState[] {
  const selection = normalizeEvidenceSelection(input.selection);
  const chunksById = new Map(
    (input.chunks ?? []).map((chunk) => [chunk.id, chunk]),
  );
  const documentsById = new Map(
    (input.documents ?? []).map((document) => [document.id, document]),
  );
  const retrievedHits = input.retrievedHits ?? [];
  const retrievedChunkIds = new Set(retrievedHits.map((hit) => hit.chunkId));
  const retrievedDocumentIds = new Set(
    retrievedHits.map((hit) => hit.documentId),
  );
  const states: EvidenceState[] = [];

  for (const documentId of input.unresolvedDocumentIds ?? []) {
    states.push({
      kind: "stale_evidence",
      label: "Stale/deleted expected evidence",
      description: `Expected document ${compactId(documentId)} is no longer available.`,
      evidenceId: documentId,
      severity: "danger",
    });
  }

  for (const chunkId of input.unresolvedChunkIds ?? []) {
    states.push({
      kind: "stale_evidence",
      label: "Stale/deleted expected evidence",
      description: `Expected chunk ${compactId(chunkId)} is no longer available.`,
      evidenceId: chunkId,
      severity: "danger",
    });
  }

  for (const chunkId of selection.chunkIds) {
    const chunk = chunksById.get(chunkId);
    if (!chunk) {
      continue;
    }
    if (retrievedChunkIds.has(chunkId)) {
      states.push({
        kind: "expected_retrieved",
        label: "Expected and retrieved",
        description: `${chunk.document_path} chunk ${chunk.ordinal + 1} was retrieved.`,
        evidenceId: chunkId,
        severity: "success",
      });
    } else if (retrievedDocumentIds.has(chunk.document_id)) {
      states.push({
        kind: "wrong_chunk",
        label: "Expected document retrieved, wrong chunk",
        description: `${chunk.document_path} was retrieved, but chunk ${chunk.ordinal + 1} was missing.`,
        evidenceId: chunkId,
        severity: "warning",
      });
    } else {
      states.push({
        kind: "expected_missing",
        label: "Expected but missing",
        description: `${chunk.document_path} chunk ${chunk.ordinal + 1} was not retrieved.`,
        evidenceId: chunkId,
        severity: "danger",
      });
    }
  }

  for (const documentId of selection.documentIds) {
    const document = documentsById.get(documentId);
    if (!document) {
      continue;
    }
    states.push({
      kind: retrievedDocumentIds.has(documentId)
        ? "expected_retrieved"
        : "expected_missing",
      label: retrievedDocumentIds.has(documentId)
        ? "Expected document retrieved"
        : "Expected document missing",
      description: `${document.path} ${
        retrievedDocumentIds.has(documentId)
          ? "appeared in retrieved evidence."
          : "did not appear in retrieved evidence."
      }`,
      evidenceId: documentId,
      severity: retrievedDocumentIds.has(documentId) ? "success" : "danger",
    });
  }

  for (const hit of retrievedHits) {
    const expectedChunk = selection.chunkIds.includes(hit.chunkId);
    const expectedDocument = selection.documentIds.includes(hit.documentId);
    if (!expectedChunk && !expectedDocument) {
      states.push({
        kind: "retrieved_not_expected",
        label: "Retrieved but not expected",
        description: `${hit.label} was retrieved but is not selected as expected evidence.`,
        evidenceId: hit.chunkId,
        severity: "neutral",
      });
    }
    if (hit.duplicate) {
      states.push({
        kind: "duplicate_evidence",
        label: "Duplicate evidence",
        description: `${hit.label} appears duplicated in the ranked evidence.`,
        evidenceId: hit.chunkId,
        severity: "warning",
      });
    }
    if (hit.weak) {
      states.push({
        kind: "weak_evidence",
        label: "Weak evidence",
        description: `${hit.label} was retrieved with weak support.`,
        evidenceId: hit.chunkId,
        severity: "warning",
      });
    }
  }

  if (input.failureLabels?.includes("duplicate_evidence")) {
    states.push(caseLevelState("duplicate_evidence", "Duplicate evidence"));
  }
  if (input.failureLabels?.includes("weak_evidence")) {
    states.push(caseLevelState("weak_evidence", "Weak evidence"));
  }
  if (input.failureLabels?.includes("correct_document_wrong_chunk")) {
    states.push(
      caseLevelState("wrong_chunk", "Expected document retrieved, wrong chunk"),
    );
  }

  return dedupeStates(states);
}

export function compactId(id: string): string {
  return id.length > 12 ? `${id.slice(0, 8)}…${id.slice(-4)}` : id;
}

function caseLevelState(kind: EvidenceStateKind, label: string): EvidenceState {
  return {
    kind,
    label,
    description: `${label} was detected at the case level from experiment failure labels.`,
    evidenceId: kind,
    severity: "warning",
  };
}

function dedupeIds(ids: string[]): string[] {
  return Array.from(new Set(ids.filter(Boolean)));
}

function dedupeStates(states: EvidenceState[]): EvidenceState[] {
  const seen = new Set<string>();
  return states.filter((state) => {
    const key = `${state.kind}:${state.evidenceId}:${state.label}`;
    if (seen.has(key)) {
      return false;
    }
    seen.add(key);
    return true;
  });
}

function normalizeQuery(query: string): string {
  return query.trim().toLowerCase().replace(/\s+/g, " ");
}
