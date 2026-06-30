import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import type { DemoProgress, DemoStatus } from "../../../lib/api/demo";
import { SetupChecklist } from "./SetupChecklist";

describe("SetupChecklist", () => {
  it.each([
    [progress(), "button", "Load sample corpus"],
    [
      progress({ sample_corpus_loaded: true }),
      "button",
      "Repair sample corpus",
    ],
    [
      progress({ sample_corpus_loaded: true, chunks_created: true }),
      "button",
      "Index sample",
    ],
    [
      progress({
        sample_corpus_loaded: true,
        chunks_created: true,
        embeddings_indexed: true,
      }),
      "link",
      "Test recommended query",
    ],
    [
      progress({
        sample_corpus_loaded: true,
        chunks_created: true,
        embeddings_indexed: true,
        retrieval_run_id: "run-1",
      }),
      "link",
      "Run and debug",
    ],
    [
      progress({
        sample_corpus_loaded: true,
        chunks_created: true,
        embeddings_indexed: true,
        retrieval_run_id: "run-1",
        trace_id: "trace-1",
      }),
      "link",
      "Open saved trace",
    ],
    [
      progress({
        sample_corpus_loaded: true,
        chunks_created: true,
        embeddings_indexed: true,
        retrieval_run_id: "run-1",
        trace_id: "trace-1",
        report_id: "report-1",
      }),
      "link",
      "Open completed audit report",
    ],
  ])("shows the next persisted action", (demoProgress, role, name) => {
    render(
      <MemoryRouter>
        <SetupChecklist
          error={null}
          isLoading={false}
          isMutating={false}
          status={status(demoProgress)}
          onIndex={vi.fn()}
          onLoad={vi.fn()}
          onRetry={vi.fn()}
        />
      </MemoryRouter>,
    );

    expect(screen.getByRole(role, { name })).toBeInTheDocument();
  });
});

function progress(overrides: Partial<DemoProgress> = {}): DemoProgress {
  return {
    sample_corpus_loaded: false,
    chunks_created: false,
    embeddings_indexed: false,
    document_count: 0,
    chunk_count: 0,
    indexed_chunk_count: 0,
    retrieval_run_id: null,
    trace_id: null,
    report_id: null,
    ...overrides,
  };
}

function status(demoProgress: DemoProgress): DemoStatus {
  return {
    version: "corpuslab-guided-demo-v1",
    project_id: "project-1",
    source_id: "source-1",
    progress: demoProgress,
    suggested_queries: [
      {
        id: "account_recovery",
        question: "How long is the reset link valid?",
        description: "Diagnose duplicated evidence.",
        recommended: true,
      },
    ],
  };
}
