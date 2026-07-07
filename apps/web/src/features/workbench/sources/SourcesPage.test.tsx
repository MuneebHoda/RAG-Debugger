import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { SourcesPage } from "./SourcesPage";

describe("SourcesPage", () => {
  beforeEach(() => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        status: 200,
        json: async () => [],
      }),
    );
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("renders upload controls", async () => {
    render(
      <MemoryRouter>
        <SourcesPage />
      </MemoryRouter>,
    );

    expect(
      await screen.findByRole("heading", { name: /corpus/i }),
    ).toBeInTheDocument();
    expect(screen.getByText(/corpus is the build step/i)).toBeInTheDocument();
    expect(
      screen
        .getAllByText("Corpus")
        .find((element) => element.getAttribute("aria-current") === "step"),
    ).toBeTruthy();
    expect(screen.getByLabelText(/choose files/i)).toBeInTheDocument();
    expect(screen.getByText(/advanced chunking/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/chunking strategy/i)).toHaveValue(
      "structured",
    );
    expect(
      screen.getByRole("button", { name: /ingest files/i }),
    ).toBeDisabled();
  });
});
