import { fireEvent, render, screen, within } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";

import { MarketingLayout } from "./MarketingLayout";

describe("MarketingLayout", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("uses the landing hero state without affecting other public routes", () => {
    const { container } = renderLayout();

    expect(
      container.querySelector("[data-landing-header-state='hero']"),
    ).toBeInTheDocument();

    fireEvent.click(
      within(
        screen.getByRole("navigation", { name: "Public navigation" }),
      ).getByRole("link", { name: "Features" }),
    );
    expect(
      container.querySelector("[data-landing-header-state]"),
    ).not.toBeInTheDocument();
  });

  it("removes landing scroll listeners when the layout unmounts", () => {
    const removeEventListener = vi.spyOn(window, "removeEventListener");
    const { unmount } = renderLayout();

    unmount();

    expect(removeEventListener).toHaveBeenCalledWith(
      "scroll",
      expect.any(Function),
    );
    expect(removeEventListener).toHaveBeenCalledWith(
      "resize",
      expect.any(Function),
    );
  });

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
