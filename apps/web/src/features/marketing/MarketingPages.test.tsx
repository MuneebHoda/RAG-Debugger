import { render, screen, within } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it } from "vitest";

import { LoginPage } from "../auth/LoginPage";
import { SignupPage } from "../auth/SignupPage";
import { FeaturesPage } from "./FeaturesPage";
import { LandingPage } from "./LandingPage";
import { PricingPage } from "./PricingPage";

const blockedLaunchCopy = ["coming soon", "future", "planned", "roadmap"];

describe("marketing pages", () => {
  it("shows the CorpusLab landing story with product imagery", () => {
    render(
      <MemoryRouter>
        <LandingPage />
      </MemoryRouter>,
    );

    expect(
      screen.getByRole("heading", {
        name: /turn every corpus into trusted retrieval/i,
      }),
    ).toBeInTheDocument();
    expect(
      screen.getByAltText(/abstract corpuslab evidence intelligence map/i),
    ).toHaveAttribute("src", "/product/corpuslab-hero-theme.png");
    expect(
      screen.getByAltText(/dashboard showing corpus health/i),
    ).toHaveAttribute("src", "/product/corpuslab-dashboard.png");
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
