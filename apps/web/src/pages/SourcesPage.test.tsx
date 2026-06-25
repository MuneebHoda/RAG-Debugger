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
      await screen.findByRole("heading", { name: /sources/i }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText(/choose files/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/chunking strategy/i)).toHaveValue(
      "structured",
    );
    expect(
      screen.getByRole("button", { name: /ingest files/i }),
    ).toBeDisabled();
  });
});
