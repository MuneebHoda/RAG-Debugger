import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";

import {
  CreateAuditReportAction,
  type AuditReportSource,
} from "./CreateAuditReportAction";

const reportId = "018f7a2a-6e2e-7000-a000-000000000901";
const sourceId = "018f7a2a-6e2e-7000-a000-000000000902";

describe("CreateAuditReportAction", () => {
  afterEach(() => vi.unstubAllGlobals());

  it.each([
    ["trace", "/api/v1/reports/from-trace", "trace_id"],
    ["experiment", "/api/v1/reports/from-experiment", "experiment_id"],
    ["ci_run", "/api/v1/reports/from-ci-run", "run_id"],
  ] as const)(
    "creates a %s report and navigates to its detail",
    async (sourceType, endpoint, idField) => {
      vi.stubGlobal(
        "fetch",
        vi.fn(async () => responseJson(report(), 201)),
      );
      renderAction({ sourceType, sourceId });

      fireEvent.click(
        screen.getByRole("button", { name: "Create audit report" }),
      );
      fireEvent.change(screen.getByLabelText("Privacy"), {
        target: { value: "snippets_allowed" },
      });
      fireEvent.click(screen.getByRole("button", { name: "Create report" }));

      expect(await screen.findByText("Opened report")).toBeInTheDocument();
      const [requestUrl, requestInit] = vi.mocked(fetch).mock.calls[0];
      expect(requestUrl.toString()).toContain(endpoint);
      expect(JSON.parse(String(requestInit?.body))).toEqual({
        [idField]: sourceId,
        privacy_mode: "snippets_allowed",
      });
    },
  );

  it("submits only once while report creation is pending", async () => {
    let resolveRequest: ((response: Response) => void) | undefined;
    const pendingRequest = new Promise<Response>((resolve) => {
      resolveRequest = resolve;
    });
    vi.stubGlobal(
      "fetch",
      vi.fn(() => pendingRequest),
    );
    renderAction({ sourceType: "trace", sourceId });

    fireEvent.click(
      screen.getByRole("button", { name: "Create audit report" }),
    );
    const confirm = screen.getByRole("button", { name: "Create report" });
    fireEvent.click(confirm);
    fireEvent.click(confirm);

    await waitFor(() => expect(fetch).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(confirm).toBeDisabled());
    resolveRequest?.(await responseJson(report(), 201));
    expect(await screen.findByText("Opened report")).toBeInTheDocument();
  });

  it("keeps structured creation errors in the action", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () =>
        responseJson(
          { error: { code: "not_found", message: "trace was removed" } },
          404,
        ),
      ),
    );
    renderAction({ sourceType: "trace", sourceId });

    fireEvent.click(
      screen.getByRole("button", { name: "Create audit report" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Create report" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "trace was removed",
    );
    expect(screen.queryByText("Opened report")).not.toBeInTheDocument();
  });
});

function renderAction(source: AuditReportSource) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={["/source"]}>
        <Routes>
          <Route
            path="/source"
            element={<CreateAuditReportAction source={source} />}
          />
          <Route path="/app/reports/:reportId" element={<p>Opened report</p>} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
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

function report() {
  return {
    id: reportId,
    workspace_id: "workspace-1",
    project_id: "project-1",
    title: "Audit report",
    subject: "",
    source: { type: "trace", trace_id: sourceId },
    privacy_mode: "snippets_allowed",
    executive_summary: "Summary",
    context: {},
    findings: [],
    recommendations: [],
    evidence: [],
    created_at: "2026-06-30T12:00:00Z",
  };
}
