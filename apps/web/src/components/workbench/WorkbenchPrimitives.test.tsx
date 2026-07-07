import { Database } from "lucide-react";
import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { WorkbenchEmptyState } from "./WorkbenchEmptyState";
import { WorkbenchMetricCard } from "./WorkbenchMetricCard";
import { WorkbenchPageHeader } from "./WorkbenchPageHeader";
import { WorkbenchPanel } from "./WorkbenchPanel";
import { WorkbenchStatusPill } from "./WorkbenchStatusPill";
import { WorkbenchToolbar } from "./WorkbenchToolbar";

describe("workbench primitives", () => {
  it("renders one explanatory page heading with actions and metadata", () => {
    render(
      <MemoryRouter>
        <WorkbenchPageHeader
          actions={<a href="#content">Primary action</a>}
          description="A concise explanation of this workspace area."
          metadata={<span>Ready</span>}
          section="Debug"
          title="Retrieval"
          titleId="retrieval-title"
        />
      </MemoryRouter>,
    );

    expect(
      screen.getByRole("heading", { level: 1, name: "Retrieval" }),
    ).toHaveAttribute("id", "retrieval-title");
    expect(screen.getByText("Debug")).toBeInTheDocument();
    expect(screen.getByText("Ready")).toBeInTheDocument();
    expect(
      screen.getByRole("link", { name: "Primary action" }),
    ).toHaveAttribute("href", "#content");
  });

  it("provides explicit next actions from empty states", () => {
    const onPrimary = vi.fn();
    render(
      <MemoryRouter>
        <WorkbenchEmptyState
          description="Add evidence before continuing."
          icon={Database}
          primaryAction={{ label: "Add evidence", onClick: onPrimary }}
          secondaryAction={{ label: "Open Home", to: "/app" }}
          title="No evidence"
        />
      </MemoryRouter>,
    );

    fireEvent.click(screen.getByRole("button", { name: /Add evidence/ }));
    expect(onPrimary).toHaveBeenCalledOnce();
    expect(screen.getByRole("link", { name: /Open Home/ })).toHaveAttribute(
      "href",
      "/app",
    );
    expect(
      screen.getByText("No evidence").closest("[data-workbench-empty-state]"),
    ).toContainElement(screen.getByRole("button", { name: /Add evidence/ }));
  });

  it("renders a semantic panel with heading, description, actions, and content", () => {
    render(
      <WorkbenchPanel
        actions={<button type="button">Refresh</button>}
        description="A shared surface for dense workbench data."
        icon={Database}
        title="Evidence panel"
        titleId="evidence-panel-title"
      >
        <p>Panel content</p>
      </WorkbenchPanel>,
    );

    const panel = screen
      .getByRole("heading", { level: 2, name: "Evidence panel" })
      .closest("[data-workbench-panel]");
    expect(panel).toHaveAttribute("aria-labelledby", "evidence-panel-title");
    expect(
      screen.getByText("A shared surface for dense workbench data."),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Refresh" })).toBeVisible();
    expect(screen.getByText("Panel content")).toBeVisible();
  });

  it("exposes status tones as stable attributes", () => {
    render(
      <WorkbenchStatusPill tone="warning">Needs review</WorkbenchStatusPill>,
    );

    expect(screen.getByText("Needs review")).toHaveAttribute(
      "data-tone",
      "warning",
    );
  });

  it("renders dense metric cards with optional details", () => {
    render(
      <WorkbenchMetricCard
        detail="18 weak runs"
        icon={Database}
        label="Traces"
        tone="info"
        value="42"
      />,
    );

    expect(screen.getByText("Traces")).toBeVisible();
    expect(screen.getByText("42")).toBeVisible();
    expect(screen.getByText("18 weak runs")).toBeVisible();
  });

  it("keeps toolbar controls keyboard reachable", () => {
    render(
      <WorkbenchToolbar label="Run filters">
        <button type="button">Search</button>
        <select aria-label="Filter">
          <option>All</option>
        </select>
      </WorkbenchToolbar>,
    );

    expect(screen.getByLabelText("Run filters")).toContainElement(
      screen.getByRole("button", { name: "Search" }),
    );
    expect(screen.getByRole("combobox", { name: "Filter" })).toBeVisible();
  });
});
