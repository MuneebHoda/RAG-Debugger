#!/usr/bin/env node

import process from "node:process";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const REQUIRED_PUBLISH_CHECKS = [
  "Analyze (actions)",
  "Analyze (javascript-typescript)",
  "Analyze (rust)",
  "Cargo Deny",
  "Database migrations",
  "Documentation",
  "Production artifacts",
  "Release dry run",
  "Rust",
  "Rust Coverage",
  "Web",
  "Web Coverage",
];

export function evaluateChecks(checkRuns) {
  return REQUIRED_PUBLISH_CHECKS.map((name) => {
    const candidates = checkRuns
      .filter(
        (check) => check.name === name && check.app?.slug === "github-actions",
      )
      .sort((left, right) => (right.id ?? 0) - (left.id ?? 0));
    const latest = candidates[0];
    if (latest?.conclusion === "success") {
      return { name, state: "success" };
    }
    return {
      name,
      state:
        latest?.status === "completed" && latest.conclusion !== "success"
          ? "failure"
          : "pending",
      conclusion: latest?.conclusion,
    };
  });
}

async function loadChecks(repository, sourceSha, token) {
  const response = await fetch(
    `https://api.github.com/repos/${repository}/commits/${sourceSha}/check-runs?per_page=100`,
    {
      headers: {
        Accept: "application/vnd.github+json",
        Authorization: `Bearer ${token}`,
        "X-GitHub-Api-Version": "2022-11-28",
      },
    },
  );
  if (!response.ok) {
    throw new Error(`GitHub Checks API returned ${response.status}`);
  }
  return (await response.json()).check_runs;
}

async function main() {
  const repository = process.env.GITHUB_REPOSITORY ?? "";
  const sourceSha = process.env.CORPUSLAB_RELEASE_SHA ?? "";
  const token = process.env.GH_TOKEN ?? "";
  if (repository !== "MuneebHoda/RAG-Debugger") {
    throw new Error("publication is restricted to MuneebHoda/RAG-Debugger");
  }
  if (!/^[a-f0-9]{40}$/.test(sourceSha)) {
    throw new Error(
      "CORPUSLAB_RELEASE_SHA must be a full lowercase commit SHA",
    );
  }
  if (!token) {
    throw new Error("GH_TOKEN is required to verify trusted checks");
  }

  const attempts = Number(process.env.CORPUSLAB_GATE_ATTEMPTS ?? 40);
  const interval = Number(process.env.CORPUSLAB_GATE_INTERVAL_MS ?? 15_000);
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    const states = evaluateChecks(
      await loadChecks(repository, sourceSha, token),
    );
    const failures = states.filter((check) => check.state === "failure");
    if (failures.length) {
      throw new Error(
        `required checks failed: ${failures.map((check) => `${check.name} (${check.conclusion})`).join(", ")}`,
      );
    }
    const pending = states.filter((check) => check.state === "pending");
    if (!pending.length) {
      console.log(
        `Verified ${states.length} required quality/security checks for ${sourceSha}.`,
      );
      return;
    }
    if (attempt === attempts) {
      throw new Error(
        `required checks did not complete successfully: ${pending.map((check) => check.name).join(", ")}`,
      );
    }
    await new Promise((resolve) => setTimeout(resolve, interval));
  }
}

if (path.resolve(process.argv[1] ?? "") === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(`Publication gate failed: ${error.message}`);
    process.exitCode = 1;
  });
}
