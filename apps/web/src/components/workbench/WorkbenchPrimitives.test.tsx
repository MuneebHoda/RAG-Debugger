import { Database } from "lucide-react";
import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { WorkbenchEmptyState } from "./WorkbenchEmptyState";
import { WorkbenchPageHeader } from "./WorkbenchPageHeader";

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
});
