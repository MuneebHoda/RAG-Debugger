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
      screen.getAllByRole("link", { name: /run the guided demo/i }),
    ).toEqual(
      expect.arrayContaining([expect.objectContaining({ pathname: "/app" })]),
    );
    expect(screen.getAllByRole("link", { name: /view the debugger/i })).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ pathname: "/app/traces" }),
      ]),
    );
    expect(
      screen.getByRole("heading", {
        name: /retrieval found candidates.*refused to answer/i,
      }),
    ).toBeInTheDocument();
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

  it("renders the connected product story in narrative order", () => {
    render(
      <MemoryRouter>
        <LandingPage />
      </MemoryRouter>,
    );

    const headings = [
      /retrieval found candidates.*refused to answer/i,
      /find the exact evidence failure/i,
      /turn a bad run into a regression test/i,
      /fail the pr before weak evidence ships/i,
      /share the diagnosis, not the documents/i,
      /raw evidence stays local until you approve what leaves/i,
      /debug the answer.*gate the fix.*share the evidence/i,
    ].map((name) => screen.getByRole("heading", { name }));

    for (let index = 1; index < headings.length; index += 1) {
      expect(
        headings[index - 1].compareDocumentPosition(headings[index]) &
          Node.DOCUMENT_POSITION_FOLLOWING,
      ).toBeTruthy();
    }
  });

  it("updates evidence, labels, diagnosis, and repair through keyboard tabs", async () => {
    render(
      <MemoryRouter>
        <LandingPage />
      </MemoryRouter>,
    );

    const story = screen
      .getByRole("heading", { name: /find the exact evidence failure/i })
      .closest("section");
    expect(story).not.toBeNull();
    const storyQueries = within(story as HTMLElement);

    expect(storyQueries.getByText("answerability_gap")).toBeInTheDocument();
    expect(storyQueries.getAllByText(/candidate only/i)).toHaveLength(3);

    const unsupported = storyQueries.getByRole("tab", {
      name: "Unsupported answer",
    });
    unsupported.focus();
    fireEvent.keyDown(unsupported, { key: "ArrowLeft" });

    await waitFor(() =>
      expect(
        storyQueries.getByRole("tab", { name: "Ranking drift" }),
      ).toHaveAttribute("aria-selected", "true"),
    );
    await waitFor(() =>
      expect(
        storyQueries.getByText("vector_lexical_disagreement"),
      ).toBeInTheDocument(),
    );
    expect(storyQueries.getByText(/supports answer/i)).toBeInTheDocument();
    expect(
      storyQueries.getByText(/compare lexical, vector, and hybrid runs/i),
    ).toBeInTheDocument();
  });

  it("shows Eval Lab, CI gate, audit report, and privacy boundaries", () => {
    render(
      <MemoryRouter>
        <LandingPage />
      </MemoryRouter>,
    );

    expect(screen.getByText("Example experiment")).toBeInTheDocument();
    expect(screen.getByText(/gate threshold: recall@5/i)).toBeInTheDocument();
    expect(screen.getByText("Merge blocked")).toBeInTheDocument();
    expect(screen.getByText("metadata_only")).toBeInTheDocument();
    expect(
      screen.getByText("No external model call is required."),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Detailed local reports cannot be exported."),
    ).toBeInTheDocument();

    const finalCta = screen
      .getByRole("heading", {
        name: /debug the answer.*gate the fix.*share the evidence/i,
      })
      .closest("section");
    expect(finalCta).not.toBeNull();
    expect(
      within(finalCta as HTMLElement).getByRole("link", {
        name: /run the guided demo/i,
      }),
    ).toHaveAttribute("href", "/app");
    expect(
      within(finalCta as HTMLElement).getByRole("link", {
        name: /view the debugger/i,
      }),
    ).toHaveAttribute("href", "/app/traces");
  });

  it("renders reveal sections immediately when reduced motion is requested", () => {
    globalThis.__setReducedMotionForTests(true);

    render(
      <MemoryRouter>
        <LandingPage />
      </MemoryRouter>,
    );

    const qualitySection = screen
      .getByRole("heading", {
        name: /turn a bad run into a regression test/i,
      })
      .closest("section");
    expect(qualitySection).not.toHaveStyle({ opacity: "0" });
    expect(
      screen.getByRole("heading", {
        name: /fail the pr before weak evidence ships/i,
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
