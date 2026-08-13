import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { Link, MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { EvidenceDiagnosisSummary } from "../../../lib/api/retrieval";
import type { Trace } from "../../../lib/api/traces";
import { RunsPage } from "./RunsPage";
import { TraceDetailPage } from "./TraceDetailPage";
import { SaveToQualityPanel } from "./components/SaveToQualityPanel";
import { TraceDiagnosisPanel } from "./components/TraceDiagnosisPanel";

const traceId = "018f7a2a-6e2e-7000-a000-000000000201";
const secondTraceId = "018f7a2a-6e2e-7000-a000-000000000202";
const datasetId = "018f7a2a-6e2e-7000-a000-000000000210";
const documentId = "document-1";
const chunkId = "018f7a2a-6e2e-7000-a000-000000000205";
const secondDocumentId = "document-2";
const secondChunkId = "018f7a2a-6e2e-7000-a000-000000000206";
let createdCaseBody: unknown;

describe("guided run workflow", () => {
  beforeEach(() => {
    createdCaseBody = undefined;
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
        const url = input.toString();
        if (url.endsWith("/api/v1/traces")) {
          return responseJson([traceSummary]);
        }
        if (url.endsWith(`/api/v1/traces/${traceId}/rerun`)) {
          return responseJson({
            trace: { ...trace, reruns: [comparison] },
            comparison,
          });
        }
        if (url.endsWith("/api/v1/eval-lab/datasets")) {
          return responseJson([
            {
              id: datasetId,
              name: "Critical questions",
              description: null,
              case_count: 0,
              latest_experiment_id: null,
              latest_gate: null,
              latest_average_recall_at_k: null,
              latest_average_precision_at_k: null,
              updated_at: "2026-06-27T10:46:19Z",
            },
          ]);
        }
        if (url.endsWith(`/api/v1/eval-lab/datasets/${datasetId}`)) {
          return responseJson({
            id: datasetId,
            name: "Critical questions",
            description: null,
            cases: [],
            created_at: "2026-06-27T10:46:19Z",
            updated_at: "2026-06-27T10:46:19Z",
          });
        }
        if (url.endsWith("/api/v1/eval-lab/evidence/query")) {
          return responseJson(evidenceLookup());
        }
        if (url.endsWith(`/api/v1/eval-lab/datasets/${datasetId}/cases`)) {
          createdCaseBody = JSON.parse(String(init?.body ?? "{}"));
          return responseJson({ id: "case-1" });
        }
        return responseJson(trace);
      }),
    );
  });

  afterEach(() => vi.unstubAllGlobals());

  it("lists saved runs and links to focused detail pages", async () => {
    renderWithClient(
      <MemoryRouter>
        <RunsPage />
      </MemoryRouter>,
    );

    expect(await screen.findByText(traceSummary.query)).toBeInTheDocument();
    expect(
      screen.getByText(/trace debugger is the diagnosis step/i),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("link", { name: /gpu embedding workers/i }),
    ).toHaveAttribute("href", `/app/traces/${traceId}`);
  });

  it("shows diagnosis tabs and comparison controls", async () => {
    renderWithClient(
      <MemoryRouter initialEntries={[`/app/traces/${traceId}`]}>
        <Routes>
          <Route path="/app/traces/:traceId" element={<TraceDetailPage />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(
      await screen.findByText(/answer support: supported/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/run detail explains what happened/i),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /evidence is too weak/i }),
    ).toBeInTheDocument();
    expect(screen.getByText(/broaden the candidate pool/i)).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Create audit report" }),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: /evidence/i }));
    expect(screen.getByText(/gpu workers speed up/i)).toBeInTheDocument();
    expect(screen.getByText(/strongest scoring signal/i)).toBeInTheDocument();
    expect(screen.getByText(/candidate only/i)).toBeInTheDocument();
    expect(screen.getAllByText(/weak evidence/i).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("tab", { name: /compare/i }));
    fireEvent.change(screen.getByLabelText(/retrieval mode/i), {
      target: { value: "lexical" },
    });
    fireEvent.click(screen.getByRole("button", { name: /run comparison/i }));
    expect(await screen.findByText("Top-score change")).toBeInTheDocument();
    expect(
      screen.getByText(/changed diagnosis from weak to mixed/i),
    ).toBeInTheDocument();
  });

  it("separates supported answers from mixed candidate quality", () => {
    const diagnosis: EvidenceDiagnosisSummary = {
      outcome: "mixed",
      summary:
        "The answer is supported by direct body evidence. Retrieval quality is mixed because some candidates have diagnostic warnings.",
      primary_issue: null,
      failures: [
        {
          code: "weak_evidence",
          severity: "warning",
          title: "Evidence is too weak",
          summary: "A lower-ranked candidate is weak.",
          evidence_refs: ["E2"],
        },
      ],
      score_explanations: [],
      recommendations: [],
    };

    render(
      <MemoryRouter>
        <TraceDiagnosisPanel answerStatus="answered" diagnosis={diagnosis} />
      </MemoryRouter>,
    );

    expect(screen.getByText(/answer support: supported/i)).toBeInTheDocument();
    expect(screen.getByText(/retrieval quality: mixed/i)).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /candidate warnings detected/i }),
    ).toBeInTheDocument();
    expect(screen.getByText("Evidence is too weak")).toBeInTheDocument();
  });

  it("shows insufficient support with its primary issue", () => {
    const primaryIssue = {
      code: "answerability_gap" as const,
      severity: "critical" as const,
      title: "Retrieved candidates cannot support an answer",
      summary: "No candidate contains enough direct body support.",
      evidence_refs: ["E1"],
    };
    const diagnosis: EvidenceDiagnosisSummary = {
      outcome: "failing",
      summary: "This run cannot support an answer.",
      primary_issue: primaryIssue,
      failures: [primaryIssue],
      score_explanations: [],
      recommendations: [],
    };

    render(
      <MemoryRouter>
        <TraceDiagnosisPanel
          answerStatus="insufficient_evidence"
          diagnosis={diagnosis}
        />
      </MemoryRouter>,
    );

    expect(
      screen.getByText(/answer support: insufficient/i),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: primaryIssue.title }),
    ).toBeInTheDocument();
  });

  it("shows unknown support for a legacy diagnosis without failures", () => {
    const diagnosis: EvidenceDiagnosisSummary = {
      outcome: "strong",
      summary: "No deterministic failure signal was found.",
      primary_issue: null,
      failures: [],
      score_explanations: [],
      recommendations: [],
    };

    render(
      <MemoryRouter>
        <TraceDiagnosisPanel diagnosis={diagnosis} />
      </MemoryRouter>,
    );

    expect(screen.getByText(/answer support: unknown/i)).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /no failure signal detected/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/no deterministic failure labels/i),
    ).toBeInTheDocument();
  });

  it("requires an explicit dataset and evidence selection for Quality", async () => {
    renderWithClient(
      <MemoryRouter initialEntries={[`/app/traces/${traceId}`]}>
        <Routes>
          <Route path="/app/traces/:traceId" element={<TraceDetailPage />} />
        </Routes>
      </MemoryRouter>,
    );

    await screen.findByText(/answer support: supported/i);
    fireEvent.click(screen.getByRole("button", { name: /choose evidence/i }));
    const datasetSelect = await screen.findByLabelText(/quality dataset/i);
    await screen.findByRole("option", { name: "Critical questions" });
    expect(datasetSelect).toHaveValue("");
    expect(
      screen.getByRole("button", { name: /save quality case/i }),
    ).toBeDisabled();

    fireEvent.change(datasetSelect, {
      target: { value: datasetId },
    });
    expect(datasetSelect).toHaveValue(datasetId);
    const saveButton = screen.getByRole("button", {
      name: /save quality case/i,
    });
    expect(saveButton).toBeDisabled();

    fireEvent.click(
      screen.getAllByRole("button", { name: "Expect this exact chunk" })[0],
    );
    await waitFor(() => expect(saveButton).toBeEnabled());
    fireEvent.click(saveButton);

    await waitFor(() => expect(createdCaseBody).toBeDefined());
    expect(createdCaseBody).toMatchObject({
      expected_chunk_ids: [chunkId],
      expected_document_ids: [],
    });
  });

  it("shows a loading state when navigating to a different trace", async () => {
    let resolveSecondTrace: ((response: Response) => void) | undefined;
    const secondTraceRequest = new Promise<Response>((resolve) => {
      resolveSecondTrace = resolve;
    });
    vi.stubGlobal(
      "fetch",
      vi.fn((input: RequestInfo | URL) => {
        const url = input.toString();
        if (url.endsWith(`/api/v1/traces/${secondTraceId}`)) {
          return secondTraceRequest;
        }
        return responseJson(trace);
      }),
    );

    renderWithClient(
      <MemoryRouter initialEntries={[`/app/traces/${traceId}`]}>
        <Link to={`/app/traces/${secondTraceId}`}>Open second run</Link>
        <Routes>
          <Route path="/app/traces/:traceId" element={<TraceDetailPage />} />
        </Routes>
      </MemoryRouter>,
    );

    await screen.findByText(/answer support: supported/i);
    fireEvent.click(screen.getByRole("link", { name: /open second run/i }));
    expect(
      await screen.findByText(/loading run diagnosis/i),
    ).toBeInTheDocument();

    await act(async () => {
      resolveSecondTrace?.(
        await responseJson({
          ...trace,
          id: secondTraceId,
          input: "second retrieval question",
        }),
      );
    });
    expect(
      await screen.findByRole("heading", {
        name: "second retrieval question",
      }),
    ).toBeInTheDocument();
  });

  it("resets source-owned Quality state when the trace identity changes", async () => {
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const view = render(
      <QueryClientProvider client={client}>
        <SaveToQualityPanel trace={trace} />
      </QueryClientProvider>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Choose evidence" }));
    const datasetSelect = await screen.findByLabelText("Quality dataset");
    await screen.findByRole("option", { name: "Critical questions" });
    fireEvent.change(datasetSelect, { target: { value: datasetId } });
    fireEvent.change(screen.getByLabelText("Case name"), {
      target: { value: "Edited first trace" },
    });
    fireEvent.change(screen.getByLabelText("Notes"), {
      target: { value: "First trace note" },
    });
    selectTraceCandidate("Expect this exact chunk");
    selectTraceCandidate("Accept evidence from this document");

    const nextTrace = secondTrace();
    view.rerender(
      <QueryClientProvider client={client}>
        <SaveToQualityPanel trace={nextTrace} />
      </QueryClientProvider>,
    );

    expect(datasetSelect).toHaveValue(datasetId);
    expect(screen.getByLabelText("Case name")).toHaveValue(nextTrace.input);
    expect(screen.getByLabelText("Notes")).toHaveValue(
      `Saved from trace ${secondTraceId.slice(0, 8)}.`,
    );
    expect(
      screen.getByText(/Select at least one expected document or chunk/i),
    ).toBeInTheDocument();
    selectTraceCandidate("Expect this exact chunk");
    selectTraceCandidate("Accept evidence from this document");
    const saveButton = screen.getByRole("button", {
      name: "Save quality case",
    });
    await waitFor(() => expect(saveButton).toBeEnabled());
    fireEvent.click(saveButton);

    await waitFor(() => expect(createdCaseBody).toBeDefined());
    expect(createdCaseBody).toEqual({
      name: nextTrace.input,
      query: nextTrace.input,
      top_k: 7,
      expected_chunk_ids: [secondChunkId],
      expected_document_ids: [secondDocumentId],
      notes: `Saved from trace ${secondTraceId.slice(0, 8)}.`,
      source_trace_id: secondTraceId,
    });
    expect(JSON.stringify(createdCaseBody)).not.toContain(chunkId);
    expect(JSON.stringify(createdCaseBody)).not.toContain(documentId);
  });

  it("explains imported and queryless Eval restrictions separately", () => {
    const imported = importedTrace();
    imported.ingestion!.privacy_mode = "metadata_only";
    const view = render(<SaveToQualityPanel trace={imported} />);

    expect(
      screen.getByText(/this imported trace cannot become an Eval Lab case/i),
    ).toBeInTheDocument();

    view.rerender(<SaveToQualityPanel trace={{ ...trace, input: "" }} />);
    expect(
      screen.getByText(/this trace has no retained query/i),
    ).toBeInTheDocument();
    expect(screen.queryByText(/imported trace/i)).not.toBeInTheDocument();
  });

  it("keeps legacy traces readable when structured diagnosis is absent", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(() => responseJson({ ...trace, diagnosis: undefined })),
    );

    renderWithClient(
      <MemoryRouter initialEntries={[`/app/traces/${traceId}`]}>
        <Routes>
          <Route path="/app/traces/:traceId" element={<TraceDetailPage />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(await screen.findByText(/legacy diagnosis/i)).toBeInTheDocument();
    expect(
      screen.getByText(/too weak for a confident answer/i),
    ).toBeInTheDocument();
  });

  it("shows imported metadata, evidence hierarchy, and privacy-gated actions", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(() => responseJson(importedTrace())),
    );
    renderWithClient(
      <MemoryRouter initialEntries={[`/app/traces/${traceId}`]}>
        <Routes>
          <Route path="/app/traces/:traceId" element={<TraceDetailPage />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(await screen.findByText("Limited diagnosis")).toBeInTheDocument();
    expect(screen.getAllByText("native")).toHaveLength(2);
    expect(screen.getByText("external-otel-1")).toBeInTheDocument();
    expect(screen.getByText("collector-demo · 1.2.3")).toBeInTheDocument();
    expect(screen.getByText("demo.tracer · 1.0")).toBeInTheDocument();
    expect(screen.queryByText("0 ms")).not.toBeInTheDocument();
    expect(
      screen.getByRole("heading", {
        name: /query withheld by privacy policy/i,
      }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Create audit report" }),
    ).toBeDisabled();
    expect(
      screen.getByText(/full-local imported traces cannot be reported/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/privacy classification cannot yet be preserved/i),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: /evidence/i }));
    expect(screen.getAllByText(/external-chunk-1/).length).toBeGreaterThan(0);
    expect(
      screen.getByText(/content withheld by privacy policy/i),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: /timeline/i }));
    const hierarchy = screen.getByRole("list", {
      name: /imported span hierarchy/i,
    });
    expect(
      within(hierarchy).getByText("<img src=x onerror=alert(1)>"),
    ).toBeInTheDocument();
    expect(document.querySelector('img[src="x"]')).toBeNull();
    expect(within(hierarchy).getByText("Generation")).toBeInTheDocument();
    expect(within(hierarchy).getByText("server")).toBeInTheDocument();
    expect(within(hierarchy).getByText("client")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: /compare/i }));
    expect(
      screen.getByText(/cannot rerun an external application/i),
    ).toBeInTheDocument();
  });

  it("shows precise empty imported evidence and span states", async () => {
    const imported = importedTrace();
    imported.ingestion = {
      ...imported.ingestion!,
      source: "otlp_http",
      privacy_mode: "metadata_only",
      evidence: [],
      spans: [],
    };
    vi.stubGlobal(
      "fetch",
      vi.fn(() => responseJson(imported)),
    );
    renderWithClient(
      <MemoryRouter initialEntries={[`/app/traces/${traceId}`]}>
        <Routes>
          <Route path="/app/traces/:traceId" element={<TraceDetailPage />} />
        </Routes>
      </MemoryRouter>,
    );

    await screen.findByText("Limited diagnosis");
    fireEvent.click(screen.getByRole("tab", { name: /evidence/i }));
    expect(
      screen.getByText(/no retrieval evidence metadata was mapped/i),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("tab", { name: /timeline/i }));
    expect(screen.getByText(/no spans were mapped/i)).toBeInTheDocument();
  });

  it("uses the same safe state for missing and inaccessible traces", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(() =>
        Promise.resolve(
          new Response(
            JSON.stringify({
              error: { code: "not_found", message: "trace not found" },
            }),
            { status: 404, headers: { "Content-Type": "application/json" } },
          ),
        ),
      ),
    );
    renderWithClient(
      <MemoryRouter initialEntries={[`/app/traces/${traceId}`]}>
        <Routes>
          <Route path="/app/traces/:traceId" element={<TraceDetailPage />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(await screen.findByRole("alert")).toHaveTextContent(
      /may have been removed or its data may be unavailable/i,
    );
  });
});

function importedTrace(): Trace {
  return {
    ...trace,
    input: "",
    output: null,
    retrieval: null,
    diagnosis: null,
    summary: "Imported trace is only partially mapped; diagnosis is limited.",
    spans: [],
    ingestion: {
      source: "native",
      external_trace_id: "external-otel-1",
      schema_version: "1",
      mapper_version: "1",
      mapping_status: "partially_mapped",
      privacy_mode: "full_local_only",
      service_name: "collector-demo",
      service_version: "1.2.3",
      deployment_environment: "local",
      instrumentation_scope_name: "demo.tracer",
      instrumentation_scope_version: "1.0",
      known_failure_labels: ["weak_evidence"],
      status_supplied: true,
      limitations: ["query_not_retained"],
      prompt: null,
      retrieval_mode: "hybrid",
      top_k: 1,
      model_config: null,
      evidence: [
        {
          external_chunk_id: "external-chunk-1",
          document_label: null,
          rank: 1,
          score: 0.7,
          lexical_score: 0.5,
          semantic_score: 0.7,
          citation_label: "E1",
          snippet: null,
          answer_support_status: "unassessed",
          answer_support_reason: "unassessed",
        },
      ],
      spans: [
        {
          external_span_id: "span-parent",
          parent_span_id: null,
          operation: "retrieval",
          kind: "server",
          name: "<img src=x onerror=alert(1)>",
          started_at: "2026-06-27T10:46:19Z",
          completed_at: "2026-06-27T10:46:20Z",
          latency_ms: 10,
          status: "succeeded",
          provider: "local",
          model: null,
          input_tokens: null,
          output_tokens: null,
          error_type: null,
        },
        {
          external_span_id: "span-child",
          parent_span_id: "span-parent",
          operation: "generation",
          kind: "client",
          name: "Generation",
          started_at: "2026-06-27T10:46:20Z",
          completed_at: "2026-06-27T10:46:20Z",
          latency_ms: 4,
          status: "warning",
          provider: "local",
          model: "demo",
          input_tokens: 3,
          output_tokens: 2,
          error_type: null,
        },
      ],
      evaluation_passed: null,
      evaluation_label: null,
      timestamps_supplied: true,
    },
  };
}

function selectTraceCandidate(name: string) {
  const candidateRegion = screen.getByRole("region", {
    name: "Retrieved evidence from this run",
  });
  fireEvent.click(within(candidateRegion).getByRole("button", { name }));
}

function secondTrace(): Trace {
  const hit = trace.retrieval!.hits[0];
  return {
    ...trace,
    id: secondTraceId,
    input: "second trace evidence",
    retrieval: {
      ...trace.retrieval!,
      run: {
        ...trace.retrieval!.run,
        id: "run-2",
        query: "second trace evidence",
        top_k: 7,
      },
      hits: [
        {
          ...hit,
          chunk: {
            ...hit.chunk,
            id: secondChunkId,
            document_id: secondDocumentId,
          },
          document: {
            ...hit.document,
            id: secondDocumentId,
            path: "second-guide.md",
          },
          citation: {
            ...hit.citation,
            chunk_id: secondChunkId,
            document_id: secondDocumentId,
            document_path: "second-guide.md",
          },
        },
      ],
    },
  };
}

function renderWithClient(children: React.ReactNode) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>{children}</QueryClientProvider>,
  );
}

const traceSummary = {
  id: traceId,
  query: "gpu embedding workers",
  retrieval_mode: "hybrid",
  latency_ms: 8,
  evidence_strength: "weak",
  failure_labels: ["weak_evidence"],
  span_count: 2,
  rerun_count: 0,
  created_at: "2026-06-27T10:46:19Z",
};

const retrieval = {
  run: {
    id: "run-1",
    query: traceSummary.query,
    top_k: 5,
    retrieval_mode: "hybrid",
    latency_ms: 8,
    created_at: "2026-06-27T10:46:19Z",
  },
  answer: {
    status: "answered",
    text: "GPU workers speed up embedding refreshes [1]",
    citations: [],
  },
  hits: [
    {
      rank: 1,
      score: 3.4,
      chunk: {
        id: chunkId,
        document_id: documentId,
        ordinal: 0,
        text: "GPU workers speed up embedding refreshes.",
        token_count: 6,
        byte_range: { start: 0, end: 42 },
        checksum: "1234567890abcdef",
        strategy: "structured",
        section_title: "Indexing",
        split_reason: "document_end",
        quality_flags: ["good_evidence_candidate"],
        is_duplicate: false,
        text_density: 0.9,
        evidence_score_hint: 0.8,
      },
      document: {
        id: documentId,
        source_id: "source-1",
        path: "platform-guide.md",
        mime_type: "text/markdown",
        checksum: "abcdef",
        byte_size: 64,
        profile: "technical_docs",
        extraction_quality: "high",
        warnings: [],
      },
      source: {
        id: "source-1",
        project_id: "project-1",
        name: "Corpus upload",
        kind: { FileSet: { root_hint: "browser-upload" } },
        sync_policy: "Manual",
        chunking: {
          target_tokens: 512,
          overlap_tokens: 64,
          strategy: "structured",
        },
      },
      matched_terms: [{ term: "gpu", count: 1 }],
      score_breakdown: {
        semantic: 0.9,
        lexical: 1.8,
        phrase: 0.4,
        section: 0.1,
        path: 0,
        metadata: 0.1,
      },
      normalized_score_breakdown: {
        semantic: 0.5,
        lexical: 1,
        phrase: 0.2,
        section: 0.05,
        path: 0,
        metadata: 0.05,
      },
      snippet: "GPU workers speed up embedding refreshes.",
      citation: {
        label: "[1]",
        chunk_id: chunkId,
        document_id: documentId,
        document_path: "platform-guide.md",
        chunk_ordinal: 0,
        section_title: "Indexing",
        checksum_prefix: "1234567890ab",
        snippet: "GPU workers speed up embedding refreshes.",
      },
      quality_flags: ["semantic_match"],
      evidence_strength: "weak",
      duplicate_count: 1,
      answer_support: {
        status: "unsupported",
        reason: "weak_evidence",
        matched_body_term_count: 2,
        query_term_count: 2,
        body_term_coverage: 1,
      },
    },
  ],
  embedding_status: {
    readiness: "ready",
    required: true,
    model: { provider: "local", model_name: "local-hash-v1", dimension: 384 },
    total_chunks: 1,
    indexed_chunks: 1,
    missing_chunks: 0,
    stale_chunks: 0,
  },
  diagnosis: {
    outcome: "weak",
    summary: "This run looks weak. Primary issue: Evidence is too weak",
    primary_issue: {
      code: "weak_evidence",
      severity: "critical",
      title: "Evidence is too weak",
      summary: "The returned evidence cannot support a defensible answer.",
      evidence_refs: ["E1"],
    },
    failures: [
      {
        code: "weak_evidence",
        severity: "critical",
        title: "Evidence is too weak",
        summary: "The returned evidence cannot support a defensible answer.",
        evidence_refs: ["E1"],
      },
    ],
    score_explanations: [
      {
        evidence_ref: "E1",
        chunk_id: chunkId,
        rank: 1,
        final_score: 3.4,
        score_delta_from_previous: null,
        score_delta_to_next: null,
        dominant_signal: "lexical",
        score_breakdown: {
          semantic: 0.9,
          lexical: 1.8,
          phrase: 0.4,
          section: 0.1,
          path: 0,
          metadata: 0.1,
        },
        normalized_score_breakdown: {
          semantic: 0.5,
          lexical: 1,
          phrase: 0.2,
          section: 0.05,
          path: 0,
          metadata: 0.05,
        },
        summary:
          "Ranked #1 with lexical overlap as the strongest scoring signal.",
      },
    ],
    recommendations: [
      {
        code: "broaden_candidate_pool",
        priority: "high",
        area: "top_k",
        title: "Broaden the candidate pool",
        rationale: "Evidence is weak.",
        action: "Increase top_k and inspect whether stronger evidence appears.",
        failure_codes: ["weak_evidence"],
        evidence_refs: ["E1"],
      },
    ],
  },
};

function evidenceLookup() {
  return {
    documents: [
      {
        id: documentId,
        source_id: "source-1",
        source_name: "Corpus upload",
        path: "platform-guide.md",
        profile: "technical_docs",
        extraction_quality: "high",
        warnings: [],
        chunk_count: 1,
      },
    ],
    chunks: [
      {
        id: chunkId,
        document_id: documentId,
        source_id: "source-1",
        source_name: "Corpus upload",
        document_path: "platform-guide.md",
        ordinal: 0,
        text: "GPU workers speed up embedding refreshes.",
        token_count: 6,
        checksum: "1234567890abcdef",
        section_title: "Indexing",
        quality_flags: ["good_evidence_candidate"],
        is_duplicate: false,
        text_density: 0.9,
        evidence_score_hint: 0.8,
      },
    ],
    unresolved_document_ids: [],
    unresolved_chunk_ids: [],
  };
}

const trace = {
  id: traceId,
  project_id: "project-1",
  input: traceSummary.query,
  output: retrieval.answer.text,
  started_at: "2026-06-27T10:46:19Z",
  completed_at: "2026-06-27T10:46:20Z",
  failure_labels: ["weak_evidence"],
  source_run_id: "run-1",
  summary: "Retrieved one chunk, but the evidence is weak.",
  status: "warning",
  evidence_strength: "weak",
  spans: [
    {
      id: "span-1",
      kind: "query_input",
      title: "Query input",
      description: "Captured the question.",
      started_at: "2026-06-27T10:46:19Z",
      completed_at: "2026-06-27T10:46:19Z",
      latency_ms: 0,
      status: "succeeded",
      detail: {
        type: "query_input",
        top_k: 5,
        retrieval_mode: "hybrid",
        source_filter_count: 0,
        document_filter_count: 0,
      },
    },
  ],
  retrieval,
  reruns: [],
  diagnosis: retrieval.diagnosis,
} as Trace;

const comparison = {
  id: "comparison-1",
  request: {
    query: traceSummary.query,
    top_k: 5,
    retrieval_mode: "lexical",
    source_ids: [],
    document_ids: [],
  },
  response: {
    ...retrieval,
    run: { ...retrieval.run, retrieval_mode: "lexical" },
  },
  score_delta: -0.4,
  latency_delta_ms: 2,
  overlap_count: 1,
  changed_rank_count: 0,
  diagnosis: {
    before_outcome: "weak",
    after_outcome: "mixed",
    summary:
      "The rerun changed diagnosis from weak to mixed, resolving 1 signal(s) and introducing 0 signal(s).",
    resolved_failures: ["weak_evidence"],
    introduced_failures: [],
    gained_evidence: [],
    lost_evidence: [],
    gained_citations: [],
    lost_citations: [],
  },
  created_at: "2026-06-27T10:46:20Z",
};

function responseJson(json: unknown) {
  return Promise.resolve({
    status: 200,
    json: async () => json,
  } as Response);
}
