import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, describe, expect, it } from "vitest";

import { LoginPage } from "../auth/LoginPage";
import { SignupPage } from "../auth/SignupPage";
import { FeaturesPage } from "./FeaturesPage";
import { LandingPage } from "./LandingPage";
import { PricingPage } from "./PricingPage";

const blockedLaunchCopy = ["coming soon", "future", "planned", "roadmap"];

afterEach(() => {
  globalThis.__setReducedMotionForTests(false);
});

describe("marketing pages", () => {
  it("shows the CorpusLab command center and guided workflow entry points", () => {
    render(
      <MemoryRouter>
        <LandingPage />
      </MemoryRouter>,
    );

    expect(
      screen.getByRole("heading", {
        name: /see why your rag answer failed/i,
      }),
    ).toBeInTheDocument();
    expect(
      screen.getByLabelText(/interactive rag diagnosis simulation/i),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("link", { name: /run the guided demo/i }),
    ).toHaveAttribute("href", "/app");
    expect(
      screen.getByRole("link", { name: /view the debugger/i }),
    ).toHaveAttribute("href", "/app/traces");
    expect(screen.getByAltText(/mission control dashboard/i)).toHaveAttribute(
      "src",
      "/product/corpuslab-dashboard.png",
    );
  });

  it("changes evidence, diagnosis, gate, and report state by scenario", async () => {
    render(
      <MemoryRouter>
        <LandingPage />
      </MemoryRouter>,
    );

    expect(screen.getByText("Answerability failed")).toBeInTheDocument();
    expect(screen.getByText("answerability gap")).toBeInTheDocument();
    expect(screen.getAllByText("Failed").length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "Strong" }));

    await waitFor(() =>
      expect(
        screen.getByText("Direct evidence, release ready"),
      ).toBeInTheDocument(),
    );
    expect(screen.getByText("No blocking labels")).toBeInTheDocument();
    expect(screen.getByText("Audit ready")).toBeInTheDocument();
    expect(screen.getByText("97", { selector: "strong" })).toBeInTheDocument();
  });

  it("provides explicit playback control and disables autoplay for reduced motion", () => {
    const { unmount } = render(
      <MemoryRouter>
        <LandingPage />
      </MemoryRouter>,
    );

    const pause = screen.getByRole("button", { name: "Pause simulation" });
    fireEvent.click(pause);
    expect(
      screen.getByRole("button", { name: "Play simulation" }),
    ).toBeInTheDocument();
    unmount();

    globalThis.__setReducedMotionForTests(true);
    render(
      <MemoryRouter>
        <LandingPage />
      </MemoryRouter>,
    );
    expect(
      screen.getByRole("button", { name: "Play simulation" }),
    ).toBeInTheDocument();
  });

  it("updates failure diagnosis through accessible stage tabs", async () => {
    render(
      <MemoryRouter>
        <LandingPage />
      </MemoryRouter>,
    );

    expect(
      screen.getByText(/bad text creates invisible evidence/i),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("tab", { name: "Retrieve" }));
    await waitFor(() =>
      expect(
        screen.getByText(/relevant evidence can exist and still rank too low/i),
      ).toBeInTheDocument(),
    );
  });

  it("updates the retrieval demo query and mode", async () => {
    render(
      <MemoryRouter>
        <LandingPage />
      </MemoryRouter>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Vector" }));
    await waitFor(() =>
      expect(screen.getByText("86% evidence strength")).toBeInTheDocument(),
    );
    fireEvent.click(screen.getByRole("button", { name: "Support escalation" }));
    await waitFor(() =>
      expect(screen.getByText(/support-operations\.md/i)).toBeInTheDocument(),
    );
  });

  it("switches the product tour without changing layout ownership", async () => {
    render(
      <MemoryRouter>
        <LandingPage />
      </MemoryRouter>,
    );

    fireEvent.click(screen.getByRole("tab", { name: "Quality" }));
    await waitFor(() =>
      expect(screen.getByAltText(/quality experiment/i)).toHaveAttribute(
        "src",
        "/product/corpuslab-evals.png",
      ),
    );
  });

  it("renders reveal sections immediately when reduced motion is requested", () => {
    globalThis.__setReducedMotionForTests(true);

    render(
      <MemoryRouter>
        <LandingPage />
      </MemoryRouter>,
    );

    const capabilitySection = screen
      .getByRole("heading", {
        name: /build, test, debug, measure, and share/i,
      })
      .closest("section");
    expect(capabilitySection).not.toHaveStyle({ opacity: "0" });
    expect(
      screen.getByRole("heading", {
        name: /one evidence system. every quality decision/i,
      }),
    ).toBeInTheDocument();
  });

  it("describes platform features in present-tense product language", () => {
    render(
      <MemoryRouter>
        <FeaturesPage />
      </MemoryRouter>,
    );

    expect(screen.getByText(/GPU and HPC workers/i)).toBeInTheDocument();
    expect(screen.getByText(/SSO\/SAML/i)).toBeInTheDocument();
    expect(screen.getByText(/API keys and SDKs/i)).toBeInTheDocument();
    expect(screen.getByText(/Evidence lineage/i)).toBeInTheDocument();
  });

  it("shows subscription plus usage pricing", () => {
    render(
      <MemoryRouter>
        <PricingPage />
      </MemoryRouter>,
    );

    const pricing = screen.getByLabelText(/pricing tiers/i);
    expect(within(pricing).getByText("Developer")).toBeInTheDocument();
    expect(within(pricing).getByText("Team")).toBeInTheDocument();
    expect(within(pricing).getByText("Scale")).toBeInTheDocument();
    expect(within(pricing).getByText("Enterprise")).toBeInTheDocument();
    expect(screen.getAllByText(/platform units/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/\$299\/mo/i)).toBeInTheDocument();
    expect(screen.getByText(/\$999\/mo/i)).toBeInTheDocument();
  });

  it("does not show placeholder launch copy on public pages", () => {
    const { rerender, container } = render(
      <MemoryRouter>
        <LandingPage />
      </MemoryRouter>,
    );

    for (const term of blockedLaunchCopy) {
      expect(container.textContent?.toLowerCase()).not.toContain(term);
    }

    rerender(
      <MemoryRouter>
        <FeaturesPage />
      </MemoryRouter>,
    );

    for (const term of blockedLaunchCopy) {
      expect(container.textContent?.toLowerCase()).not.toContain(term);
    }
  });
});

describe("auth pages", () => {
  it("renders login and signup entry points", () => {
    const { rerender } = render(
      <MemoryRouter>
        <LoginPage />
      </MemoryRouter>,
    );

    expect(
      screen.getByRole("heading", { name: /sign in/i }),
    ).toBeInTheDocument();
    expect(screen.getByText("demo@corpuslab.ai")).toBeInTheDocument();
    expect(screen.getByText(/SSO/i)).toBeInTheDocument();

    rerender(
      <MemoryRouter>
        <SignupPage />
      </MemoryRouter>,
    );

    expect(
      screen.getByRole("heading", { name: /create your corpuslab workspace/i }),
    ).toBeInTheDocument();
    expect(screen.getByText(/Run evals/i)).toBeInTheDocument();
  });
});
