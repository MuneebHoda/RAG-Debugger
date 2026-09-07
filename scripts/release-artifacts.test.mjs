import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  checksumManifest,
  evaluateTrivyReport,
  validateLockfile,
  validateArtifactSubjects,
  verifyAttestation,
  verifyWebArtifact,
} from "./release-artifacts.mjs";
import {
  evaluateChecks,
  evaluatePublicationGate,
  loadCodeScanningAlerts,
  REQUIRED_PUBLISH_CHECKS,
} from "./verify-publish-gates.mjs";

const sha256 = (value) => createHash("sha256").update(value).digest("hex");

test("checksum manifests are deterministic and runtime config stays separate", async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "corpuslab-release-"));
  try {
    await mkdir(path.join(directory, "assets"));
    await writeFile(path.join(directory, "index.html"), "immutable html\n");
    await writeFile(path.join(directory, "assets/app.js"), "immutable js\n");
    await writeFile(path.join(directory, "runtime-config.js"), "staging\n");
    const first = await checksumManifest(directory, [
      "index.html",
      "assets/app.js",
    ]);
    await writeFile(path.join(directory, "runtime-config.js"), "production\n");
    const second = await checksumManifest(directory, [
      "assets/app.js",
      "index.html",
    ]);
    assert.equal(first, second);
    await writeFile(path.join(directory, "assets/app.js"), "changed js\n");
    assert.notEqual(
      await checksumManifest(directory, ["index.html", "assets/app.js"]),
      first,
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("web archive verification rejects digest mismatches and runtime config", async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "corpuslab-release-"));
  try {
    const stage = path.join(directory, "stage");
    await mkdir(stage);
    await writeFile(path.join(stage, "index.html"), "immutable html\n");
    const manifest = await checksumManifest(stage, ["index.html"]);
    await writeFile(
      path.join(directory, "web-application.files.sha256"),
      manifest,
    );
    execFileSync(
      "zip",
      ["-X", "-q", path.join(directory, "web.zip"), "index.html"],
      { cwd: stage },
    );
    const archive = await readFile(path.join(directory, "web.zip"));
    const web = {
      artifact: "web.zip",
      application_sha256: sha256(manifest),
      archive_sha256: sha256(archive),
    };
    await assert.doesNotReject(verifyWebArtifact(directory, web));
    await assert.rejects(
      verifyWebArtifact(directory, { ...web, archive_sha256: "0".repeat(64) }),
      /archive checksum mismatch/,
    );

    await writeFile(
      path.join(stage, "runtime-config.js"),
      "public runtime config\n",
    );
    const unsafeManifest = await checksumManifest(stage, [
      "index.html",
      "runtime-config.js",
    ]);
    await writeFile(
      path.join(directory, "web-application.files.sha256"),
      unsafeManifest,
    );
    execFileSync(
      "zip",
      [
        "-X",
        "-q",
        path.join(directory, "unsafe.zip"),
        "index.html",
        "runtime-config.js",
      ],
      { cwd: stage },
    );
    const unsafeArchive = await readFile(path.join(directory, "unsafe.zip"));
    await assert.rejects(
      verifyWebArtifact(directory, {
        artifact: "unsafe.zip",
        application_sha256: sha256(unsafeManifest),
        archive_sha256: sha256(unsafeArchive),
      }),
      /must not define the web application identity/,
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("release policy fails on missing locks, attestations, vulnerabilities, and secrets", () => {
  assert.throws(
    () => validateLockfile("Cargo.lock", Buffer.alloc(0)),
    /missing or empty/,
  );
  assert.throws(
    () => verifyAttestation(undefined, "API provenance", true),
    /missing or invalid/,
  );
  assert.doesNotThrow(() =>
    verifyAttestation(
      { status: "not-created-dry-run" },
      "API provenance",
      false,
    ),
  );

  const vulnerabilityReport = {
    SchemaVersion: 2,
    Results: [
      {
        Target: "api",
        Vulnerabilities: [
          { VulnerabilityID: "CVE-TEST", PkgName: "fixture", Severity: "HIGH" },
        ],
      },
    ],
  };
  assert.equal(
    evaluateTrivyReport(vulnerabilityReport).vulnerabilities.length,
    1,
  );
  const secretReport = {
    SchemaVersion: 2,
    Results: [
      {
        Target: "web",
        Secrets: [{ RuleID: "private-key", Category: "AsymmetricPrivateKey" }],
      },
    ],
  };
  assert.equal(evaluateTrivyReport(secretReport).secrets.length, 1);
  assert.deepEqual(
    evaluateTrivyReport({
      SchemaVersion: 2,
      Results: [{ Vulnerabilities: [{ Severity: "MEDIUM" }] }],
    }),
    { vulnerabilities: [], secrets: [] },
  );

  const manifest = {
    source: { commit: "a".repeat(40) },
    api: { image: "corpuslab-api", reference: null },
    web: { artifact: "web.zip", archive_sha256: "b".repeat(64) },
    sboms: { api: {}, web: {} },
    provenance: { api: {}, web: {} },
    scans: { api: {}, web: {} },
  };
  const apiSubject = `corpuslab-api:${manifest.source.commit}`;
  const webSubject = `web.zip@sha256:${manifest.web.archive_sha256}`;
  for (const group of [manifest.sboms, manifest.provenance, manifest.scans]) {
    group.api.subject = apiSubject;
    group.web.subject = webSubject;
  }
  assert.doesNotThrow(() => validateArtifactSubjects(manifest, false));
  manifest.sboms.web.subject = "web.zip@sha256:wrong";
  assert.throws(
    () => validateArtifactSubjects(manifest, false),
    /web SBOM subject mismatch/,
  );
});

function passingChecks() {
  return REQUIRED_PUBLISH_CHECKS.map((name) => ({
    name,
    status: "completed",
    conclusion: "success",
    app: { slug: "github-actions" },
  }));
}

test("publication gates require every named GitHub Actions check", () => {
  const passing = passingChecks();
  assert.ok(
    evaluateChecks(passing).every((check) => check.state === "success"),
  );
  assert.equal(
    evaluateChecks(passing.filter((check) => check.name !== "Cargo Deny")).find(
      (check) => check.name === "Cargo Deny",
    ).state,
    "pending",
  );
  passing[0].conclusion = "failure";
  assert.equal(evaluateChecks(passing)[0].state, "failure");
});

test("successful CodeQL execution with no open alerts passes", () => {
  assert.equal(evaluatePublicationGate(passingChecks(), []).state, "success");
});

test("successful CodeQL execution with an open alert fails", () => {
  const result = evaluatePublicationGate(passingChecks(), [
    { number: 49, rule: { id: "actions/untrusted-checkout/medium" } },
  ]);
  assert.equal(result.state, "failure");
  assert.equal(result.codeScanningAlerts.length, 1);
});

test("failed CodeQL execution fails publication", () => {
  const checks = passingChecks();
  checks.find((check) => check.name === "Analyze (rust)").conclusion =
    "failure";
  assert.equal(evaluatePublicationGate(checks, []).state, "failure");
});

test("pending CodeQL execution remains pending", () => {
  const checks = passingChecks();
  const codeql = checks.find((check) => check.name === "Analyze (actions)");
  codeql.status = "in_progress";
  codeql.conclusion = null;
  assert.equal(evaluatePublicationGate(checks, []).state, "pending");
});

test("CodeQL alert API failures cannot be ignored", async () => {
  const responses = [
    {
      ok: true,
      headers: { get: () => null },
      json: async () => [
        {
          number: 120,
          merged_at: "2026-09-07T00:00:00Z",
          merge_commit_sha: "a".repeat(40),
          head: { sha: "b".repeat(40) },
          base: {
            ref: "main",
            repo: { full_name: "MuneebHoda/RAG-Debugger" },
          },
        },
      ],
    },
    {
      ok: false,
      status: 403,
      headers: { get: () => null },
    },
  ];
  await assert.rejects(
    loadCodeScanningAlerts(
      "MuneebHoda/RAG-Debugger",
      "a".repeat(40),
      "token",
      async () => responses.shift(),
    ),
    /Code scanning alerts for pull request #120 API returned 403/,
  );
});

test("published verification rejects manifest identity self-assertion", async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "corpuslab-release-"));
  try {
    const manifest = path.join(directory, "release-manifest.json");
    await writeFile(
      manifest,
      JSON.stringify({
        source: { commit: "a".repeat(40) },
        release: { application_version: "0.1.0" },
      }),
    );
    const verify = (releaseSha, version) =>
      spawnSync("./scripts/verify-published-release.sh", [manifest], {
        cwd: path.resolve(import.meta.dirname, ".."),
        encoding: "utf8",
        env: {
          ...process.env,
          GITHUB_REPOSITORY: "MuneebHoda/RAG-Debugger",
          CORPUSLAB_RELEASE_SHA: releaseSha,
          CORPUSLAB_VERSION: version,
        },
      });

    const missingIdentity = verify("", "");
    assert.notEqual(missingIdentity.status, 0);
    assert.match(missingIdentity.stderr, /independently identify/);

    const sourceMismatch = verify("b".repeat(40), "0.1.0");
    assert.notEqual(sourceMismatch.status, 0);
    assert.match(sourceMismatch.stderr, /independently resolved source/);

    const versionMismatch = verify("a".repeat(40), "0.2.0");
    assert.notEqual(versionMismatch.status, 0);
    assert.match(versionMismatch.stderr, /independently resolved version/);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
