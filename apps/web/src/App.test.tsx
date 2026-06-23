import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it } from "vitest";

import { App } from "./App";

describe("App", () => {
  it("renders the product shell", () => {
    render(
      <MemoryRouter>
        <App />
      </MemoryRouter>,
    );

    expect(
      screen.getByRole("heading", { name: /rag debugger/i }),
    ).toBeInTheDocument();
    expect(screen.getByText(/retrieval diagnosis/i)).toBeInTheDocument();
  });
});
