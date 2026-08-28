import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type {
  GoldenDataset,
  GoldenDatasetImportSummary,
} from "../../../../lib/api/evalLab";
import {
  GoldenDatasetExportPanel,
  GoldenDatasetImportPanel,
} from "./GoldenDatasetTransfer";

describe("golden dataset transfer", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("warns before full export and requires dry-run validation before import", async () => {
    const requests: string[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
        const url = input.toString();
        requests.push(`${init?.method ?? "GET"} ${url}`);
        const dryRun = url.includes("dry_run=true");
        return new Response(JSON.stringify(importSummary(dryRun, !dryRun)), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        });
      }),
    );

    renderTransfer();

    const warning = screen.getByText(/full JSON exposes queries, notes/i);
    const disclosure = warning.parentElement as HTMLDetailsElement;
    expect(disclosure.open).toBe(false);
    fireEvent.click(warning);
    expect(disclosure.open).toBe(true);
    expect(
      screen.getByRole("link", { name: "Export full dataset" }),
    ).toHaveAttribute("href", expect.stringContaining("content_mode=full"));

    const file = new File([JSON.stringify(portableDataset())], "golden.json", {
      type: "application/json",
    });
    Object.defineProperty(file, "text", {
      value: () => Promise.resolve(JSON.stringify(portableDataset())),
    });
    fireEvent.change(screen.getByLabelText("Dataset JSON"), {
      target: { files: [file] },
    });

    const validate = screen.getByRole("button", { name: "Validate dry run" });
    await waitFor(() => expect(validate).toBeEnabled());
    expect(screen.getByRole("button", { name: "Apply import" })).toBeDisabled();
    fireEvent.click(validate);
    expect(await screen.findByText(/"valid":true/)).toBeInTheDocument();

    const apply = screen.getByRole("button", { name: "Apply import" });
    expect(apply).toBeEnabled();
    fireEvent.click(apply);
    expect(await screen.findByText(/"applied":true/)).toBeInTheDocument();

    expect(requests.some((request) => request.includes("dry_run=true"))).toBe(
      true,
    );
    expect(
      requests.some(
        (request) =>
          request.includes("dry_run=false") &&
          request.includes("validation_token=validation-token"),
      ),
    ).toBe(true);
  });
});

function renderTransfer() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  render(
    <QueryClientProvider client={client}>
      <GoldenDatasetExportPanel datasetId="dataset-1" />
      <GoldenDatasetImportPanel datasets={[]} onApplied={() => {}} />
    </QueryClientProvider>,
  );
}

function portableDataset(): GoldenDataset {
  return {
    schema_version: 1,
    content_mode: "full",
    dataset: {
      key: "release-gate",
      name: "Release gate",
      description: null,
    },
    cases: [
      {
        case_key: "gpu-indexing",
        name: "GPU indexing",
        query: "How does GPU indexing work?",
        top_k: 5,
        expected_documents: [{ document_checksum: "doc-checksum" }],
        expected_chunks: [],
        notes: null,
      },
    ],
  };
}

function importSummary(
  dryRun: boolean,
  applied: boolean,
): GoldenDatasetImportSummary {
  return {
    schema_version: 1,
    mode: "create_new",
    dry_run: dryRun,
    valid: true,
    applied,
    action: "create_dataset",
    dataset_id: applied ? "dataset-2" : null,
    cases_total: 1,
    cases_added: 1,
    cases_changed: 0,
    cases_skipped: 0,
    cases_removed: 0,
    invalid_cases: [],
    unresolved_evidence: [],
    privacy_sensitive_fields: ["queries", "evidence_references"],
    validation_token: dryRun ? "validation-token" : null,
  };
}
