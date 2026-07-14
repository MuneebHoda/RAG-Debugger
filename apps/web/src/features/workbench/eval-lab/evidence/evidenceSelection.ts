import type {
  EvalLabEvidenceChunk,
  EvalLabEvidenceDocument,
  RetrievalEvalCase,
  RetrievalEvalFailureLabel,
  UpdateRetrievalEvalCaseRequest,
} from "../../../../lib/api/evalLab";

export interface EvidenceSelection {
  documentIds: string[];
  chunkIds: string[];
}

export type EvidenceSelectionUpdatePayload = Pick<
  UpdateRetrievalEvalCaseRequest,
  "expected_chunk_ids" | "expected_document_ids"
>;

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

export interface EvidenceRetrievedHitRef {
  chunkId: string;
  rank?: number | null;
  weak?: boolean;
  duplicate?: boolean;
}

export type EvidenceStateContext =
  | { kind: "expectation_only" }
  | { kind: "retrieval"; hits: EvidenceRetrievedHitRef[] };

export type EvidenceMetadata =
  | {
      status: "resolved";
      documents: EvalLabEvidenceDocument[];
      chunks: EvalLabEvidenceChunk[];
      unresolvedDocumentIds: string[];
      unresolvedChunkIds: string[];
    }
  | { status: "unavailable" };

export type EvidenceStateKind =
  | "expected_document"
  | "expected_exact_chunk"
  | "expected_retrieved"
  | "expected_missing"
  | "retrieved_not_expected"
  | "wrong_chunk"
  | "duplicate_evidence"
  | "weak_evidence"
  | "stale_evidence"
  | "metadata_unavailable";

export interface EvidenceState {
  kind: EvidenceStateKind;
  label: string;
  description: string;
  severity: "success" | "warning" | "danger" | "neutral";
  evidenceId: string;
  snippet?: string;
}

export interface EvidenceStateInput {
  selection: EvidenceSelection;
  context: EvidenceStateContext;
  metadata: EvidenceMetadata;
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

export function evidenceSelectionsEqual(
  left: EvidenceSelection,
  right: EvidenceSelection,
): boolean {
  const normalizedLeft = normalizeEvidenceSelection(left);
  const normalizedRight = normalizeEvidenceSelection(right);
  return (
    idSetsEqual(normalizedLeft.documentIds, normalizedRight.documentIds) &&
    idSetsEqual(normalizedLeft.chunkIds, normalizedRight.chunkIds)
  );
}

export function buildUpdateCaseEvidencePayload(
  original: EvidenceSelection,
  current: EvidenceSelection,
): EvidenceSelectionUpdatePayload {
  if (evidenceSelectionsEqual(original, current)) {
    return {};
  }

  const normalized = normalizeEvidenceSelection(current);
  return {
    expected_chunk_ids: normalized.chunkIds,
    expected_document_ids: normalized.documentIds,
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
  if (input.metadata.status === "unavailable") {
    return unavailableMetadataStates(selection, input.context);
  }

  const chunksById = new Map(
    input.metadata.chunks.map((chunk) => [chunk.id, chunk]),
  );
  const documentsById = new Map(
    input.metadata.documents.map((document) => [document.id, document]),
  );
  const retrievedHits =
    input.context.kind === "retrieval" ? input.context.hits : [];
  const resolvedRetrievedHits = retrievedHits.flatMap((hit) => {
    const chunk = chunksById.get(hit.chunkId);
    return chunk ? [{ hit, chunk }] : [];
  });
  const retrievedChunkIds = new Set(
    resolvedRetrievedHits.map(({ hit }) => hit.chunkId),
  );
  const retrievedDocumentIds = new Set(
    resolvedRetrievedHits.map(({ chunk }) => chunk.document_id),
  );
  const unresolvedDocumentIds = new Set(input.metadata.unresolvedDocumentIds);
  const unresolvedChunkIds = new Set(input.metadata.unresolvedChunkIds);
  const states: EvidenceState[] = [];

  for (const chunkId of selection.chunkIds) {
    const chunk = chunksById.get(chunkId);
    if (!chunk) {
      states.push(
        unresolvedChunkIds.has(chunkId) &&
          !retrievedHits.some((hit) => hit.chunkId === chunkId)
          ? staleExpectedChunk(chunkId)
          : unavailableEvidenceState(
              chunkId,
              "Expected exact chunk metadata unavailable",
              "The selected chunk could not be resolved, so its retrieval state is unknown.",
            ),
      );
      continue;
    }
    if (input.context.kind === "expectation_only") {
      states.push({
        kind: "expected_exact_chunk",
        label: "Expected exact chunk",
        description: `${chunkLabel(chunk)} must be retrieved for this case.`,
        evidenceId: chunkId,
        severity: "neutral",
        snippet: chunkSnippet(chunk),
      });
      continue;
    }
    if (retrievedChunkIds.has(chunkId)) {
      states.push({
        kind: "expected_retrieved",
        label: "Expected and retrieved",
        description: `${chunkLabel(chunk)} was retrieved.`,
        evidenceId: chunkId,
        severity: "success",
        snippet: chunkSnippet(chunk),
      });
    } else if (retrievedDocumentIds.has(chunk.document_id)) {
      states.push({
        kind: "wrong_chunk",
        label: "Expected document retrieved, wrong chunk",
        description: `${chunk.document_path} was retrieved, but ${chunkLabel(chunk)} was missing.`,
        evidenceId: chunkId,
        severity: "warning",
        snippet: chunkSnippet(chunk),
      });
    } else {
      states.push({
        kind: "expected_missing",
        label: "Expected but missing",
        description: `${chunkLabel(chunk)} was not retrieved.`,
        evidenceId: chunkId,
        severity: "danger",
        snippet: chunkSnippet(chunk),
      });
    }
  }

  for (const documentId of selection.documentIds) {
    const document = documentsById.get(documentId);
    if (!document) {
      const retrievedChild = resolvedRetrievedHits.find(
        ({ chunk }) => chunk.document_id === documentId,
      );
      if (input.context.kind === "retrieval" && retrievedChild) {
        states.push({
          kind: "expected_retrieved",
          label: "Expected document retrieved",
          description: `${retrievedChild.chunk.document_path} appeared in retrieved evidence through ${chunkLabel(retrievedChild.chunk)}.`,
          evidenceId: documentId,
          severity: "success",
        });
        continue;
      }
      states.push(
        unresolvedDocumentIds.has(documentId)
          ? staleExpectedDocument(documentId)
          : unavailableEvidenceState(
              documentId,
              "Expected document metadata unavailable",
              "The selected document could not be resolved, so its retrieval state is unknown.",
            ),
      );
      continue;
    }
    if (input.context.kind === "expectation_only") {
      states.push({
        kind: "expected_document",
        label: "Expected document",
        description: `Any suitable chunk from ${document.path} may satisfy this case.`,
        evidenceId: documentId,
        severity: "neutral",
      });
      continue;
    }
    const wasRetrieved = retrievedDocumentIds.has(documentId);
    states.push({
      kind: wasRetrieved ? "expected_retrieved" : "expected_missing",
      label: wasRetrieved
        ? "Expected document retrieved"
        : "Expected document missing",
      description: `${document.path} ${
        wasRetrieved
          ? "appeared in retrieved evidence."
          : "did not appear in retrieved evidence."
      }`,
      evidenceId: documentId,
      severity: wasRetrieved ? "success" : "danger",
    });
  }

  for (const hit of retrievedHits) {
    const chunk = chunksById.get(hit.chunkId);
    if (!chunk) {
      if (!selection.chunkIds.includes(hit.chunkId)) {
        states.push(
          unavailableEvidenceState(
            hit.chunkId,
            "Retrieved evidence metadata unavailable",
            `Retrieved chunk ${compactId(hit.chunkId)} could not be resolved.`,
          ),
        );
      }
      continue;
    }
    const expectedChunk = selection.chunkIds.includes(hit.chunkId);
    const expectedDocument = selection.documentIds.includes(chunk.document_id);
    if (!expectedChunk && !expectedDocument) {
      states.push({
        kind: "retrieved_not_expected",
        label: "Retrieved but not expected",
        description: `${chunkLabel(chunk)} was retrieved but is not selected as expected evidence.`,
        evidenceId: hit.chunkId,
        severity: "neutral",
        snippet: chunkSnippet(chunk),
      });
    }
    if (hit.duplicate) {
      states.push({
        kind: "duplicate_evidence",
        label: "Duplicate evidence",
        description: `${chunkLabel(chunk)} appears duplicated in the ranked evidence.`,
        evidenceId: hit.chunkId,
        severity: "warning",
      });
    }
    if (hit.weak) {
      states.push({
        kind: "weak_evidence",
        label: "Weak evidence",
        description: `${chunkLabel(chunk)} was retrieved with weak support.`,
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

  return dedupeStates(states);
}

export function evidenceLookupChunkIds(
  selection: EvidenceSelection,
  context: EvidenceStateContext,
): string[] {
  return dedupeIds([
    ...selection.chunkIds,
    ...(context.kind === "retrieval"
      ? context.hits.map((hit) => hit.chunkId)
      : []),
  ]);
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

function unavailableMetadataStates(
  selection: EvidenceSelection,
  context: EvidenceStateContext,
): EvidenceState[] {
  const states = [
    ...selection.documentIds.map((documentId) =>
      unavailableEvidenceState(
        documentId,
        "Expected document metadata unavailable",
        `Expected document ${compactId(documentId)} could not be resolved.`,
      ),
    ),
    ...selection.chunkIds.map((chunkId) =>
      unavailableEvidenceState(
        chunkId,
        "Expected exact chunk metadata unavailable",
        `Expected chunk ${compactId(chunkId)} could not be resolved.`,
      ),
    ),
  ];

  if (context.kind === "retrieval") {
    for (const hit of context.hits) {
      if (!selection.chunkIds.includes(hit.chunkId)) {
        states.push(
          unavailableEvidenceState(
            hit.chunkId,
            "Retrieved evidence metadata unavailable",
            `Retrieved chunk ${compactId(hit.chunkId)} could not be resolved.`,
          ),
        );
      }
    }
  }

  return dedupeStates(states);
}

function staleExpectedDocument(documentId: string): EvidenceState {
  return {
    kind: "stale_evidence",
    label: "Stale/deleted expected evidence",
    description: `Expected document ${compactId(documentId)} is no longer available.`,
    evidenceId: documentId,
    severity: "danger",
  };
}

function staleExpectedChunk(chunkId: string): EvidenceState {
  return {
    kind: "stale_evidence",
    label: "Stale/deleted expected evidence",
    description: `Expected chunk ${compactId(chunkId)} is no longer available.`,
    evidenceId: chunkId,
    severity: "danger",
  };
}

function unavailableEvidenceState(
  evidenceId: string,
  label: string,
  description: string,
): EvidenceState {
  return {
    kind: "metadata_unavailable",
    label,
    description,
    evidenceId,
    severity: "neutral",
  };
}

function chunkLabel(chunk: EvalLabEvidenceChunk): string {
  const section = chunk.section_title ? ` · ${chunk.section_title}` : "";
  return `${chunk.document_path} · chunk ${chunk.ordinal + 1}${section}`;
}

function chunkSnippet(chunk: EvalLabEvidenceChunk): string | undefined {
  const text = chunk.text.trim();
  if (!text) {
    return undefined;
  }
  return text.length > 150 ? `${text.slice(0, 147)}…` : text;
}

function dedupeIds(ids: string[]): string[] {
  return Array.from(new Set(ids.filter(Boolean)));
}

function idSetsEqual(left: string[], right: string[]): boolean {
  if (left.length !== right.length) {
    return false;
  }
  const rightIds = new Set(right);
  return left.every((id) => rightIds.has(id));
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
