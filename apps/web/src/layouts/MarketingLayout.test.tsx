import { fireEvent, render, screen, within } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { describe, expect, it } from "vitest";

import { MarketingLayout } from "./MarketingLayout";

describe("MarketingLayout", () => {
  it("opens and closes mobile navigation with Escape", () => {
    renderLayout();

    const menuButton = screen.getByRole("button", { name: "Open menu" });
    fireEvent.click(menuButton);
    expect(screen.getByRole("button", { name: "Close menu" })).toHaveAttribute(
      "aria-expanded",
      "true",
    );

    fireEvent.keyDown(window, { key: "Escape" });
    const closedMenuButton = screen.getByRole("button", { name: "Open menu" });
    expect(closedMenuButton).toHaveAttribute("aria-expanded", "false");
    expect(closedMenuButton).toHaveFocus();
  });

  it("closes mobile navigation after route selection", () => {
    renderLayout();
    fireEvent.click(screen.getByRole("button", { name: "Open menu" }));
    fireEvent.click(
      within(
        screen.getByRole("navigation", {
          name: "Mobile public navigation",
        }),
      ).getByRole("link", {
        name: "Features",
      }),
    );
    expect(screen.getByRole("button", { name: "Open menu" })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
    expect(
      screen.getByRole("heading", { name: "Features page" }),
    ).toBeInTheDocument();
  });
});

function renderLayout() {
  return render(
    <MemoryRouter initialEntries={["/"]}>
      <Routes>
        <Route element={<MarketingLayout />}>
          <Route index element={<h1>Landing page</h1>} />
          <Route path="features" element={<h1>Features page</h1>} />
        </Route>
      </Routes>
    </MemoryRouter>,
  );
}
