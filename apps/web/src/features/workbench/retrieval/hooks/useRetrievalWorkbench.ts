import { useEffect, useMemo, useState } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";

import {
  getDemoStatus,
  type DemoQueryId,
  type DemoStatus,
} from "../../../../lib/api/demo";
import {
  getEmbeddingStatus,
  indexEmbeddings,
  type EmbeddingStatus,
} from "../../../../lib/api/embeddings";
import {
  queryRetrieval,
  type RetrievalMode,
  type RetrievalQueryResponse,
} from "../../../../lib/api/retrieval";
import { listSources, type SourceSummary } from "../../../../lib/api/sources";
import { createTraceFromRetrievalRun } from "../../../../lib/api/traces";
import {
  collectDocuments,
  filterDocumentsBySources,
  retainVisibleDocumentIds,
  toggleSelection,
} from "../utils/retrievalFilters";

const DEFAULT_TOP_K = 5;

export function useRetrievalWorkbench() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const [sources, setSources] = useState<SourceSummary[]>([]);
  const [query, setQuery] = useState("");
  const [topK, setTopK] = useState(DEFAULT_TOP_K);
  const [retrievalMode, setRetrievalMode] = useState<RetrievalMode>("hybrid");
  const [selectedSourceIds, setSelectedSourceIds] = useState<string[]>([]);
  const [selectedDocumentIds, setSelectedDocumentIds] = useState<string[]>([]);
  const [response, setResponse] = useState<RetrievalQueryResponse | null>(null);
  const [embeddingStatus, setEmbeddingStatus] =
    useState<EmbeddingStatus | null>(null);
  const [demoStatus, setDemoStatus] = useState<DemoStatus | null>(null);
  const [isLoadingSources, setIsLoadingSources] = useState(true);
  const [isQuerying, setIsQuerying] = useState(false);
  const [isIndexing, setIsIndexing] = useState(false);
  const [isSavingTrace, setIsSavingTrace] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    Promise.all([
      listSources(controller.signal),
      getEmbeddingStatus(controller.signal),
      getDemoStatus(controller.signal),
    ])
      .then(([nextSources, nextEmbeddingStatus, nextDemoStatus]) => {
        setSources(nextSources);
        setEmbeddingStatus(nextEmbeddingStatus);
        setDemoStatus(nextDemoStatus);
        const demoQueryId = searchParams.get("demo_query");
        const suggestedQuery = nextDemoStatus.suggested_queries.find(
          (item) => item.id === demoQueryId,
        );
        if (suggestedQuery && nextDemoStatus.source_id) {
          setQuery(suggestedQuery.question);
          setSelectedSourceIds([nextDemoStatus.source_id]);
        }
      })
      .catch((cause: unknown) => {
        if (!controller.signal.aborted) setError(errorMessage(cause));
      })
      .finally(() => {
        if (!controller.signal.aborted) setIsLoadingSources(false);
      });

    return () => controller.abort();
  }, [searchParams]);

  const allDocuments = useMemo(() => collectDocuments(sources), [sources]);
  const visibleDocuments = useMemo(
    () => filterDocumentsBySources(allDocuments, selectedSourceIds),
    [allDocuments, selectedSourceIds],
  );
  const activeSelectedDocumentIds = useMemo(
    () => retainVisibleDocumentIds(selectedDocumentIds, visibleDocuments),
    [selectedDocumentIds, visibleDocuments],
  );

  async function submitQuery() {
    if (query.trim().length === 0 || isQuerying) return;

    setIsQuerying(true);
    setError(null);
    try {
      setResponse(
        await queryRetrieval({
          query: query.trim(),
          top_k: topK,
          retrieval_mode: retrievalMode,
          source_ids: selectedSourceIds,
          document_ids: activeSelectedDocumentIds,
        }),
      );
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setIsQuerying(false);
    }
  }

  async function refreshEmbeddings() {
    if (isIndexing) return;

    setIsIndexing(true);
    setError(null);
    try {
      const result = await indexEmbeddings({
        source_ids: selectedSourceIds,
        document_ids: activeSelectedDocumentIds,
      });
      setEmbeddingStatus(result.status);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setIsIndexing(false);
    }
  }

  async function saveTrace() {
    if (!response || isSavingTrace) return;

    setIsSavingTrace(true);
    setError(null);
    try {
      const trace = await createTraceFromRetrievalRun({
        run_id: response.run.id,
      });
      navigate(`/app/traces/${trace.id}`);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setIsSavingTrace(false);
    }
  }

  function toggleSource(sourceId: string) {
    setSelectedSourceIds((currentIds) => toggleSelection(currentIds, sourceId));
  }

  function toggleDocument(documentId: string) {
    setSelectedDocumentIds((currentIds) =>
      toggleSelection(currentIds, documentId),
    );
  }

  function selectSuggestedQuery(queryId: DemoQueryId) {
    const suggestion = demoStatus?.suggested_queries.find(
      (item) => item.id === queryId,
    );
    if (!suggestion || !demoStatus?.source_id) return;
    setQuery(suggestion.question);
    setSelectedSourceIds([demoStatus.source_id]);
    setSelectedDocumentIds([]);
    navigate(`/app/retrieval?demo_query=${queryId}`, { replace: true });
  }

  return {
    activeSelectedDocumentIds,
    allDocuments,
    embeddingStatus,
    demoStatus,
    error,
    isIndexing,
    isLoadingSources,
    isQuerying,
    isSavingTrace,
    query,
    response,
    retrievalMode,
    selectedSourceIds,
    sources,
    topK,
    visibleDocuments,
    refreshEmbeddings,
    saveTrace,
    selectSuggestedQuery,
    setQuery,
    setRetrievalMode,
    setTopK,
    submitQuery,
    toggleDocument,
    toggleSource,
  };
}

function errorMessage(cause: unknown) {
  return cause instanceof Error ? cause.message : "Unexpected request failure";
}
