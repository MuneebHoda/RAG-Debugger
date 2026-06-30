import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { DebugReport } from "../../../lib/api/reports";
import { ReportDetailPage } from "./ReportDetailPage";
import { ReportsPage } from "./ReportsPage";

const reportId = "018f7a2a-6e2e-7000-a000-000000000801";
const traceId = "018f7a2a-6e2e-7000-a000-000000000802";
const experimentId = "018f7a2a-6e2e-7000-a000-000000000803";

describe("audit reports workbench", () => {
  beforeEach(() => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
        const url = input.toString();
        if (url.endsWith("/api/v1/reports/from-trace")) {
          expect(init?.method).toBe("POST");
          return responseJson(report, 201);
        }
        if (url.endsWith(`/api/v1/reports/${reportId}/export.md`)) {
          return new Response("# Retrieval audit\n", { status: 200 });
        }
        if (url.endsWith(`/api/v1/reports/${reportId}`)) {
          return responseJson(report);
        }
        if (url.endsWith("/api/v1/reports")) {
          return responseJson([report]);
        }
        if (url.endsWith("/api/v1/traces")) {
          return responseJson([traceSummary]);
        }
        if (url.endsWith("/api/v1/eval-lab/experiments")) {
          return responseJson([experimentSummary]);
        }
        if (url.endsWith("/api/v1/eval-lab/ci/runs")) {
          return responseJson([]);
        }
        if (url.endsWith("/api/v1/sources")) {
          return responseJson([]);
        }
        return responseJson({ error: { message: "not found" } }, 404);
      }),
    );
  });

  afterEach(() => vi.unstubAllGlobals());

  it("leads with saved reports and retains diagnostic candidates", async () => {
    renderWithClient(
      <MemoryRouter>
        <ReportsPage />
      </MemoryRouter>,
    );

    expect(await screen.findByText(report.title)).toBeInTheDocument();
    expect(screen.getByText("Run diagnoses")).toBeInTheDocument();
    expect(screen.getAllByText(traceSummary.query)).toHaveLength(2);
    expect(screen.getByText("Corpus findings")).toBeInTheDocument();
  });

  it("creates a metadata-only report and opens its detail route", async () => {
    renderWithClient(
      <MemoryRouter initialEntries={["/app/reports"]}>
        <Routes>
          <Route path="/app/reports" element={<ReportsPage />} />
          <Route
            path="/app/reports/:reportId"
            element={<div>Created report detail</div>}
          />
        </Routes>
      </MemoryRouter>,
    );

    await screen.findByRole("option", { name: traceSummary.query });
    fireEvent.change(screen.getByLabelText("Run"), {
      target: { value: traceId },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create report" }));

    expect(
      await screen.findByText("Created report detail"),
    ).toBeInTheDocument();
    const fetchMock = vi.mocked(fetch);
    const createCall = fetchMock.mock.calls.find(([input]) =>
      input.toString().endsWith("/api/v1/reports/from-trace"),
    );
    expect(JSON.parse(String(createCall?.[1]?.body))).toEqual({
      trace_id: traceId,
      privacy_mode: "metadata_only",
    });
  });

  it("renders report details and copies Markdown", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("navigator", {
      ...navigator,
      clipboard: { writeText },
    });

    renderWithClient(
      <MemoryRouter initialEntries={[`/app/reports/${reportId}`]}>
        <Routes>
          <Route path="/app/reports/:reportId" element={<ReportDetailPage />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(
      await screen.findByText(report.executive_summary),
    ).toBeInTheDocument();
    expect(screen.getByText("Weak evidence")).toBeInTheDocument();
    expect(screen.getByText("Increase retrieval depth")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Copy Markdown" }));

    await waitFor(() =>
      expect(writeText).toHaveBeenCalledWith("# Retrieval audit\n"),
    );
    expect(screen.getByRole("button", { name: "Copied" })).toBeInTheDocument();
  });

  it("blocks Markdown export for full-local reports", async () => {
    vi.mocked(fetch).mockImplementation(async (input) => {
      if (input.toString().endsWith(`/api/v1/reports/${reportId}`)) {
        return responseJson({ ...report, privacy_mode: "full_local_only" });
      }
      return responseJson([]);
    });

    renderWithClient(
      <MemoryRouter initialEntries={[`/app/reports/${reportId}`]}>
        <Routes>
          <Route path="/app/reports/:reportId" element={<ReportDetailPage />} />
        </Routes>
      </MemoryRouter>,
    );

    const copyButton = await screen.findByRole("button", {
      name: "Copy Markdown",
    });
    expect(copyButton).toBeDisabled();
    expect(
      screen.getByText(/export is blocked until it is redacted/i),
    ).toBeInTheDocument();
  });
});

function renderWithClient(children: React.ReactNode) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>{children}</QueryClientProvider>,
  );
}

function responseJson(value: unknown, status = 200) {
  return Promise.resolve(
    new Response(JSON.stringify(value), {
      status,
      headers: { "Content-Type": "application/json" },
    }),
  );
}

const report: DebugReport = {
  id: reportId,
  workspace_id: "018f7a2a-6e2e-7000-a000-000000000810",
  project_id: "018f7a2a-6e2e-7000-a000-000000000811",
  title: "Retrieval audit",
  subject: "GPU worker question",
  source: { type: "trace", trace_id: traceId },
  privacy_mode: "metadata_only",
  executive_summary: "The run returned weak evidence for the requested topic.",
  context: { retrieval_mode: "hybrid", top_k: "5", latency_ms: "8" },
  findings: [
    {
      code: "weak-evidence",
      severity: "warning",
      title: "Weak evidence",
      summary:
        "The strongest retrieved chunk did not clear the evidence threshold.",
      failure_labels: ["weak_evidence"],
      evidence_refs: ["E1"],
    },
  ],
  evidence: [
    {
      label: "E1",
      role: "retrieved",
      source_id: null,
      document_id: null,
      chunk_id: "018f7a2a-6e2e-7000-a000-000000000820",
      rank: 1,
      document_path: null,
      section_title: null,
      checksum_prefix: "abcdef123456",
      citation_label: "1",
      snippet: null,
      evidence_strength: "weak",
      chunk_quality_flags: [],
      retrieval_quality_flags: ["low_score"],
    },
  ],
  recommendations: [
    {
      code: "increase-top-k",
      priority: "high",
      area: "top_k",
      title: "Increase retrieval depth",
      rationale: "Relevant evidence may be ranked below the current cutoff.",
      action: "Rerun with a higher top_k and compare precision.",
      finding_codes: ["weak-evidence"],
    },
  ],
  created_at: "2026-06-30T12:00:00Z",
};

const traceSummary = {
  id: traceId,
  query: "How are GPU workers configured?",
  retrieval_mode: "hybrid",
  latency_ms: 8,
  evidence_strength: "weak",
  failure_labels: ["weak_evidence"],
  span_count: 4,
  rerun_count: 0,
  created_at: "2026-06-30T11:00:00Z",
};

const experimentSummary = {
  id: experimentId,
  dataset_id: "dataset-1",
  dataset_name: "Release questions",
  name: "Hybrid baseline",
  modes: ["hybrid"],
  top_k: 5,
  config_snapshot: {},
  mode_results: [],
  comparison: {},
  gate: { status: "failed" },
  failures: [],
  created_at: "2026-06-30T10:00:00Z",
};
