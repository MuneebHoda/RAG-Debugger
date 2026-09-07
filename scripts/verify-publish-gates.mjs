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

export function evaluatePublicationGate(checkRuns, codeScanningAlerts) {
  if (!Array.isArray(codeScanningAlerts)) {
    throw new Error("CodeQL alert results are required");
  }
  const checks = evaluateChecks(checkRuns);
  if (checks.some((check) => check.state === "failure")) {
    return { state: "failure", checks, codeScanningAlerts: [] };
  }
  if (checks.some((check) => check.state === "pending")) {
    return { state: "pending", checks, codeScanningAlerts: [] };
  }
  return {
    state: codeScanningAlerts.length ? "failure" : "success",
    checks,
    codeScanningAlerts,
  };
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

async function loadGithubPages(url, token, label, fetchImpl) {
  const values = [];
  while (url) {
    const response = await fetchImpl(url, {
      headers: {
        Accept: "application/vnd.github+json",
        Authorization: `Bearer ${token}`,
        "X-GitHub-Api-Version": "2022-11-28",
      },
    });
    if (!response.ok) {
      throw new Error(`${label} API returned ${response.status}`);
    }
    const page = await response.json();
    if (!Array.isArray(page)) {
      throw new Error(`${label} API returned an invalid response`);
    }
    values.push(...page);
    const next = response.headers
      .get("link")
      ?.split(",")
      .find((link) => link.includes('rel="next"'));
    url = next?.match(/<([^>]+)>/)?.[1] ?? "";
  }
  return values;
}

export async function loadCodeScanningAlerts(
  repository,
  sourceSha,
  token,
  fetchImpl = fetch,
) {
  const pulls = await loadGithubPages(
    `https://api.github.com/repos/${repository}/commits/${sourceSha}/pulls?per_page=100`,
    token,
    "Associated pull requests",
    fetchImpl,
  );
  const trustedPulls = pulls.filter(
    (pull) =>
      pull.merged_at &&
      pull.base?.ref === "main" &&
      pull.base?.repo?.full_name === repository &&
      (pull.merge_commit_sha === sourceSha || pull.head?.sha === sourceSha),
  );
  if (!trustedPulls.length) {
    throw new Error(
      `no merged main pull request is bound to release SHA ${sourceSha}`,
    );
  }

  const alerts = [];
  for (const pull of trustedPulls) {
    const pullAlerts = await loadGithubPages(
      `https://api.github.com/repos/${repository}/code-scanning/alerts?state=open&pr=${pull.number}&per_page=100`,
      token,
      `Code scanning alerts for pull request #${pull.number}`,
      fetchImpl,
    );
    alerts.push(
      ...pullAlerts.map((alert) => ({
        ...alert,
        pull_request_number: pull.number,
      })),
    );
  }
  return alerts;
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
    const checkRuns = await loadChecks(repository, sourceSha, token);
    const states = evaluateChecks(checkRuns);
    const failures = states.filter((check) => check.state === "failure");
    if (failures.length) {
      throw new Error(
        `required checks failed: ${failures.map((check) => `${check.name} (${check.conclusion})`).join(", ")}`,
      );
    }
    const pending = states.filter((check) => check.state === "pending");
    if (!pending.length) {
      const gate = evaluatePublicationGate(
        checkRuns,
        await loadCodeScanningAlerts(repository, sourceSha, token),
      );
      if (gate.codeScanningAlerts.length) {
        throw new Error(
          `open CodeQL alerts block publication: ${gate.codeScanningAlerts
            .map(
              (alert) =>
                `#${alert.number} ${alert.rule?.id ?? "unknown-rule"} (PR #${alert.pull_request_number})`,
            )
            .join(", ")}`,
        );
      }
      console.log(
        `Verified ${states.length} required checks and zero open CodeQL alerts for ${sourceSha}.`,
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
