import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { WorkbenchWorkflowGuide } from "./WorkbenchWorkflowGuide";

describe("WorkbenchWorkflowGuide", () => {
  it("shows purpose, quality impact, loop position, and linked next action", () => {
    render(
      <MemoryRouter>
        <WorkbenchWorkflowGuide
          currentStep="retrieval"
          impact="Separates broad candidate ranking from answerable evidence."
          nextAction={{ label: "Open Trace Debugger", to: "/app/traces" }}
          purpose="Ask one diagnostic question against the current corpus."
        />
      </MemoryRouter>,
    );

    expect(
      screen.getByText(/ask one diagnostic question/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/separates broad candidate ranking/i),
    ).toBeInTheDocument();
    expect(activeStep("Retrieval")).toBeTruthy();
    expect(
      screen.getByRole("link", { name: /open trace debugger/i }),
    ).toHaveAttribute("href", "/app/traces");
  });

  it("supports button actions for page-local workflow operations", () => {
    const onClick = vi.fn();

    render(
      <MemoryRouter>
        <WorkbenchWorkflowGuide
          currentStep="trace"
          impact="Turns one run into reusable eval and report evidence."
          nextAction={{ label: "Save run", onClick }}
          purpose="Diagnose the saved retrieval run."
        />
      </MemoryRouter>,
    );

    fireEvent.click(screen.getByRole("button", { name: /save run/i }));

    expect(onClick).toHaveBeenCalledTimes(1);
  });
});

function activeStep(label: string) {
  return screen
    .getAllByText(label)
    .find((element) => element.getAttribute("aria-current") === "step");
}
