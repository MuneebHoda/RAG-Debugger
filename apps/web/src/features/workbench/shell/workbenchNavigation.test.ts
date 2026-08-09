import { describe, expect, it } from "vitest";

import {
  isWorkbenchNavItemActive,
  resolveWorkbenchBreadcrumbs,
  WORKBENCH_FLOW_LABELS,
} from "./workbenchNavigation";

describe("workbench navigation", () => {
  it("keeps the product loop in one canonical order", () => {
    expect(WORKBENCH_FLOW_LABELS).toEqual([
      "Corpus",
      "Retrieval",
      "Trace Debugger",
      "Eval Lab",
      "CI Runs",
      "Audit Reports",
    ]);
  });

  it("activates parent areas for detail routes", () => {
    expect(
      isWorkbenchNavItemActive("corpus", "/app/sources/document-1", ""),
    ).toBe(true);
    expect(isWorkbenchNavItemActive("traces", "/app/traces/trace-1", "")).toBe(
      true,
    );
    expect(
      isWorkbenchNavItemActive(
        "eval_lab",
        "/app/evals/experiments/experiment-1",
        "",
      ),
    ).toBe(true);
  });

  it("treats the CI query view as a distinct navigation item", () => {
    expect(
      isWorkbenchNavItemActive("ci_runs", "/app/evals", "?view=ci-runs"),
    ).toBe(true);
    expect(
      isWorkbenchNavItemActive("eval_lab", "/app/evals", "?view=ci-runs"),
    ).toBe(false);
    expect(
      isWorkbenchNavItemActive("ci_runs", "/app/evals/ci-runs/run-1", ""),
    ).toBe(true);
    expect(
      isWorkbenchNavItemActive("eval_lab", "/app/evals/ci-runs/run-1", ""),
    ).toBe(false);
  });

  it("builds stable breadcrumbs for list and detail routes", () => {
    expect(resolveWorkbenchBreadcrumbs("/app/sources/document-1", "")).toEqual([
      { label: "Home", to: "/app" },
      { label: "Corpus", to: "/app/sources" },
      { label: "Document" },
    ]);
    expect(resolveWorkbenchBreadcrumbs("/app/evals", "?view=ci-runs")).toEqual([
      { label: "Home", to: "/app" },
      { label: "Eval Lab", to: "/app/evals" },
      { label: "CI Runs" },
    ]);
    expect(resolveWorkbenchBreadcrumbs("/app/evals/ci-runs/run-1", "")).toEqual(
      [
        { label: "Home", to: "/app" },
        { label: "Eval Lab", to: "/app/evals" },
        { label: "CI Runs", to: "/app/evals?view=ci-runs" },
        { label: "Run detail" },
      ],
    );
  });
});
