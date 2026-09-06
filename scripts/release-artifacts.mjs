#!/usr/bin/env node

import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import {
  readFile,
  readdir,
  mkdir,
  rm,
  copyFile,
  chmod,
  utimes,
  writeFile,
} from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const outputDirectory = path.join(repositoryRoot, "target/release-artifacts");
const webStageDirectory = path.join(outputDirectory, "web-application");
const fixedArchiveDate = new Date("1980-01-01T00:00:00Z");
const sha256Pattern = /^[a-f0-9]{64}$/;
const digestPattern = /^sha256:[a-f0-9]{64}$/;
const sourceShaPattern = /^[a-f0-9]{40}$/;
const forbiddenManifestKeys =
  /database_url|password|credential|cookie|customer|runtime_config_value/i;

function invariant(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

async function sha256File(filePath) {
  return sha256(await readFile(filePath));
}

async function listFiles(directory, prefix = "") {
  const entries = await readdir(path.join(directory, prefix), {
    withFileTypes: true,
  });
  const files = [];
  for (const entry of entries.sort((left, right) =>
    left.name.localeCompare(right.name, "en"),
  )) {
    const relativePath = path.posix.join(
      prefix.split(path.sep).join(path.posix.sep),
      entry.name,
    );
    if (entry.isDirectory()) {
      files.push(...(await listFiles(directory, relativePath)));
    } else if (entry.isFile()) {
      files.push(relativePath);
    }
  }
  return files;
}

export async function checksumManifest(directory, files) {
  const lines = [];
  for (const relativePath of [...files].sort()) {
    const digest = await sha256File(path.join(directory, relativePath));
    lines.push(`${digest}  ${relativePath}`);
  }
  return `${lines.join("\n")}\n`;
}

export function validateLockfile(relativePath, contents) {
  invariant(contents.length > 0, `${relativePath} is missing or empty`);
}

async function requireLockfiles() {
  for (const relativePath of ["Cargo.lock", "apps/web/package-lock.json"]) {
    const contents = await readFile(path.join(repositoryRoot, relativePath));
    validateLockfile(relativePath, contents);
  }
}

async function prepare() {
  const sourceSha = process.env.CORPUSLAB_RELEASE_SHA ?? "";
  const version = process.env.CORPUSLAB_VERSION ?? "";
  const apiImage = process.env.CORPUSLAB_API_IMAGE ?? "";
  invariant(
    sourceShaPattern.test(sourceSha),
    "CORPUSLAB_RELEASE_SHA must be a full lowercase commit SHA",
  );
  invariant(
    /^0\.[0-9]+\.[0-9]+(?:-rc\.[1-9][0-9]*)?$/.test(version),
    "CORPUSLAB_VERSION must follow the pre-release version policy",
  );
  invariant(
    apiImage.length > 0 &&
      !apiImage.includes("@") &&
      !apiImage.endsWith(":latest"),
    "CORPUSLAB_API_IMAGE must be an untagged immutable-publication image name",
  );
  await requireLockfiles();

  await rm(outputDirectory, { recursive: true, force: true });
  await mkdir(webStageDirectory, { recursive: true });

  const webDistDirectory = path.join(repositoryRoot, "apps/web/dist");
  const webFiles = (await listFiles(webDistDirectory)).filter(
    (file) => file !== "runtime-config.js",
  );
  invariant(webFiles.length > 0, "the web application build is empty");
  for (const relativePath of webFiles) {
    const destination = path.join(webStageDirectory, relativePath);
    await mkdir(path.dirname(destination), { recursive: true });
    await copyFile(path.join(webDistDirectory, relativePath), destination);
    await chmod(destination, 0o644);
    await utimes(destination, fixedArchiveDate, fixedArchiveDate);
  }

  const webManifest = await checksumManifest(webStageDirectory, webFiles);
  await writeFile(
    path.join(outputDirectory, "web-application.files.sha256"),
    webManifest,
  );

  const migrationsDirectory = path.join(repositoryRoot, "migrations");
  const migrationFiles = (await listFiles(migrationsDirectory)).filter((file) =>
    file.endsWith(".sql"),
  );
  invariant(
    migrationFiles.length > 0,
    "the embedded SQLx migration set is empty",
  );
  const migrationManifest = await checksumManifest(
    migrationsDirectory,
    migrationFiles,
  );
  await writeFile(
    path.join(outputDirectory, "migrations.files.sha256"),
    migrationManifest,
  );

  const archive = `corpuslab-web-${sourceSha}.zip`;
  const identities = {
    schema_version: 1,
    source_commit: sourceSha,
    application_version: version,
    api_tag: `${apiImage}:${sourceSha}`,
    web: {
      artifact: archive,
      application_sha256: sha256(webManifest),
    },
    migrations: {
      manifest: "migrations.files.sha256",
      sha256: sha256(migrationManifest),
    },
  };
  await writeFile(
    path.join(outputDirectory, "build-identities.json"),
    `${JSON.stringify(identities, null, 2)}\n`,
  );
}

async function finalize() {
  const identitiesPath = path.join(outputDirectory, "build-identities.json");
  const identities = JSON.parse(await readFile(identitiesPath, "utf8"));
  const archivePath = path.join(outputDirectory, identities.web.artifact);
  identities.web.archive_sha256 = await sha256File(archivePath);
  await writeFile(identitiesPath, `${JSON.stringify(identities, null, 2)}\n`);
}

function parseChecksumManifest(contents, label) {
  const entries = new Map();
  for (const line of contents.trimEnd().split("\n")) {
    const match = /^([a-f0-9]{64})  ([^\r\n]+)$/.exec(line);
    invariant(match, `${label} contains an invalid checksum line`);
    invariant(
      !path.posix.isAbsolute(match[2]) && !match[2].split("/").includes(".."),
      `${label} contains an unsafe path`,
    );
    invariant(!entries.has(match[2]), `${label} repeats ${match[2]}`);
    entries.set(match[2], match[1]);
  }
  invariant(entries.size > 0, `${label} is empty`);
  return entries;
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    encoding: null,
    maxBuffer: 128 * 1024 * 1024,
    ...options,
  });
  invariant(result.status === 0, `${command} ${args.join(" ")} failed`);
  return result.stdout;
}

export async function verifyWebArtifact(directory, web) {
  invariant(
    path.basename(web.artifact) === web.artifact,
    "web artifact name must be a basename",
  );
  invariant(
    sha256Pattern.test(web.application_sha256),
    "web application checksum is invalid",
  );
  invariant(
    sha256Pattern.test(web.archive_sha256),
    "web archive checksum is invalid",
  );
  const archivePath = path.join(directory, web.artifact);
  invariant(
    (await sha256File(archivePath)) === web.archive_sha256,
    "web archive checksum mismatch",
  );

  const manifestPath = path.join(directory, "web-application.files.sha256");
  const manifestContents = await readFile(manifestPath, "utf8");
  invariant(
    sha256(manifestContents) === web.application_sha256,
    "web application checksum mismatch",
  );
  const expected = parseChecksumManifest(
    manifestContents,
    "web application checksum manifest",
  );
  invariant(
    !expected.has("runtime-config.js"),
    "runtime-config.js must not define the web application identity",
  );

  const archiveEntries = run("unzip", ["-Z1", archivePath])
    .toString("utf8")
    .trimEnd()
    .split("\n")
    .filter((entry) => entry && !entry.endsWith("/"));
  invariant(
    archiveEntries.every(
      (entry) =>
        !path.posix.isAbsolute(entry) && !entry.split("/").includes(".."),
    ),
    "web archive contains an unsafe path",
  );
  invariant(
    !archiveEntries.includes("runtime-config.js"),
    "web archive must exclude runtime-config.js",
  );
  assert.deepEqual(
    [...archiveEntries].sort(),
    [...expected.keys()].sort(),
    "web archive file list mismatch",
  );
  for (const entry of archiveEntries) {
    invariant(
      sha256(run("unzip", ["-p", archivePath, entry])) === expected.get(entry),
      `web archive checksum mismatch for ${entry}`,
    );
  }
}

async function verifyMigrationSet(directory, migrations) {
  invariant(
    migrations.manifest === "migrations.files.sha256",
    "migration manifest name is invalid",
  );
  invariant(
    sha256Pattern.test(migrations.sha256),
    "migration-set checksum is invalid",
  );
  const published = await readFile(
    path.join(directory, migrations.manifest),
    "utf8",
  );
  parseChecksumManifest(published, "migration checksum manifest");
  invariant(
    sha256(published) === migrations.sha256,
    "migration-set checksum mismatch",
  );
  const migrationFiles = (
    await listFiles(path.join(repositoryRoot, "migrations"))
  ).filter((file) => file.endsWith(".sql"));
  const actual = await checksumManifest(
    path.join(repositoryRoot, "migrations"),
    migrationFiles,
  );
  invariant(
    actual === published,
    "release migration set does not match the checked-out source",
  );
}

async function verifySpdx(directory, entry) {
  invariant(
    path.basename(entry.artifact) === entry.artifact,
    "SBOM artifact name must be a basename",
  );
  invariant(sha256Pattern.test(entry.sha256), "SBOM checksum is invalid");
  const filePath = path.join(directory, entry.artifact);
  invariant(
    (await sha256File(filePath)) === entry.sha256,
    `${entry.artifact} checksum mismatch`,
  );
  const document = JSON.parse(await readFile(filePath, "utf8"));
  invariant(
    document.spdxVersion === "SPDX-2.3",
    `${entry.artifact} must be SPDX 2.3`,
  );
  invariant(
    document.SPDXID === "SPDXRef-DOCUMENT",
    `${entry.artifact} has an invalid SPDX document ID`,
  );
  invariant(
    typeof document.documentNamespace === "string" &&
      document.documentNamespace.length > 0,
    `${entry.artifact} has no SPDX namespace`,
  );
  invariant(
    Array.isArray(document.packages) && Array.isArray(document.files),
    `${entry.artifact} has no SPDX package/file inventory`,
  );
  invariant(
    document.packages.length + document.files.length > 0,
    `${entry.artifact} has an empty SPDX inventory`,
  );
}

export function verifyAttestation(attestation, label, published) {
  if (!published) {
    invariant(
      attestation?.status === "not-created-dry-run",
      `${label} must identify the dry-run attestation boundary`,
    );
    return;
  }
  invariant(
    /^[0-9]+$/.test(attestation?.id ?? ""),
    `${label} attestation ID is missing or invalid`,
  );
  invariant(
    /^https:\/\/github\.com\/MuneebHoda\/RAG-Debugger\/attestations\/[0-9]+$/.test(
      attestation?.url ?? "",
    ),
    `${label} attestation URL is missing or invalid`,
  );
}

export function evaluateTrivyReport(report) {
  invariant(
    report?.SchemaVersion === 2 &&
      (report.Results == null || Array.isArray(report.Results)),
    "Trivy report has an invalid schema",
  );
  const vulnerabilities = [];
  const secrets = [];
  for (const result of report.Results ?? []) {
    for (const vulnerability of result.Vulnerabilities ?? []) {
      if (["HIGH", "CRITICAL"].includes(vulnerability.Severity)) {
        vulnerabilities.push({
          target: result.Target,
          id: vulnerability.VulnerabilityID,
          package: vulnerability.PkgName,
          severity: vulnerability.Severity,
        });
      }
    }
    for (const secret of result.Secrets ?? []) {
      secrets.push({
        target: result.Target,
        rule: secret.RuleID,
        category: secret.Category,
      });
    }
  }
  return { vulnerabilities, secrets };
}

async function verifyScan(reportPath) {
  const findings = evaluateTrivyReport(
    JSON.parse(await readFile(reportPath, "utf8")),
  );
  if (findings.vulnerabilities.length || findings.secrets.length) {
    for (const finding of findings.vulnerabilities) {
      console.error(
        `forbidden vulnerability: ${finding.severity} ${finding.id} ${finding.package} (${finding.target})`,
      );
    }
    for (const finding of findings.secrets) {
      console.error(
        `forbidden secret finding: ${finding.rule} ${finding.category} (${finding.target})`,
      );
    }
    throw new Error(
      "Trivy policy rejects High/Critical vulnerabilities and every secret finding",
    );
  }
}

function requireEnvironment(name, pattern) {
  const value = process.env[name] ?? "";
  invariant(pattern.test(value), `${name} is missing or invalid`);
  return value;
}

async function createManifest() {
  const identities = JSON.parse(
    await readFile(path.join(outputDirectory, "build-identities.json"), "utf8"),
  );
  const mode = process.env.CORPUSLAB_RELEASE_MODE ?? "";
  invariant(
    mode === "published" || mode === "dry-run",
    "CORPUSLAB_RELEASE_MODE must be published or dry-run",
  );
  const published = mode === "published";
  const imageDigest = requireEnvironment("CORPUSLAB_API_DIGEST", digestPattern);
  const repository = requireEnvironment(
    "GITHUB_REPOSITORY",
    /^MuneebHoda\/RAG-Debugger$/,
  );
  const runId = requireEnvironment("GITHUB_RUN_ID", /^[0-9]+$/);
  const runAttempt = requireEnvironment("GITHUB_RUN_ATTEMPT", /^[1-9][0-9]*$/);
  const image = process.env.CORPUSLAB_API_IMAGE ?? "";
  invariant(
    image === "ghcr.io/muneebhoda/rag-debugger" ||
      (!published && image === "corpuslab-api"),
    "API image name is outside the release contract",
  );

  const attestation = (name) =>
    published
      ? {
          id: requireEnvironment(
            `CORPUSLAB_${name}_ATTESTATION_ID`,
            /^[0-9]+$/,
          ),
          url: requireEnvironment(
            `CORPUSLAB_${name}_ATTESTATION_URL`,
            /^https:\/\/github\.com\/MuneebHoda\/RAG-Debugger\/attestations\/[0-9]+$/,
          ),
        }
      : { status: "not-created-dry-run" };

  const artifactEntry = async (artifact, subject, attestationName) => ({
    artifact,
    sha256: await sha256File(path.join(outputDirectory, artifact)),
    subject,
    attestation: attestation(attestationName),
  });
  const fileEntry = async (artifact, subject) => ({
    artifact,
    sha256: await sha256File(path.join(outputDirectory, artifact)),
    subject,
  });
  const apiReference = published ? `${image}@${imageDigest}` : null;
  const webSubject = `${identities.web.artifact}@sha256:${identities.web.archive_sha256}`;
  const manifest = {
    schema_version: 1,
    publication: { mode },
    source: {
      repository: `https://github.com/${repository}`,
      commit: identities.source_commit,
    },
    release: {
      application_version: identities.application_version,
      version_tag: null,
    },
    api: {
      image,
      digest: imageDigest,
      reference: apiReference,
      full_commit_tag: `${image}:${identities.source_commit}`,
    },
    web: {
      artifact: identities.web.artifact,
      application_sha256: identities.web.application_sha256,
      archive_sha256: identities.web.archive_sha256,
      runtime_config_identity:
        "excluded; generated and checksummed at deployment time",
    },
    migrations: identities.migrations,
    sboms: {
      format: "SPDX-2.3",
      api: await artifactEntry(
        "api.spdx.json",
        published ? apiReference : identities.api_tag,
        "API_SBOM",
      ),
      web: await artifactEntry("web.spdx.json", webSubject, "WEB_SBOM"),
    },
    provenance: {
      api: {
        subject: published ? apiReference : identities.api_tag,
        attestation: attestation("API_PROVENANCE"),
      },
      web: { subject: webSubject, attestation: attestation("WEB_PROVENANCE") },
    },
    scans: {
      policy:
        "Trivy: any secret or known High/Critical vulnerability fails publication; no implicit exceptions",
      api: await fileEntry(
        "api-trivy.json",
        published ? apiReference : identities.api_tag,
      ),
      web: await fileEntry("web-trivy.json", webSubject),
    },
    workflow: {
      name: "Publish deployment artifacts",
      run_id: runId,
      run_attempt: Number(runAttempt),
      url: `https://github.com/${repository}/actions/runs/${runId}/attempts/${runAttempt}`,
    },
  };
  await writeFile(
    path.join(
      outputDirectory,
      mode === "published"
        ? "release-manifest.json"
        : "release-manifest.dry-run.json",
    ),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );
}

export function validateArtifactSubjects(manifest, published) {
  const apiSubject = published
    ? manifest.api.reference
    : `${manifest.api.image}:${manifest.source.commit}`;
  const webSubject = `${manifest.web.artifact}@sha256:${manifest.web.archive_sha256}`;
  for (const [label, actual, expected] of [
    ["API SBOM", manifest.sboms?.api?.subject, apiSubject],
    ["web SBOM", manifest.sboms?.web?.subject, webSubject],
    ["API provenance", manifest.provenance?.api?.subject, apiSubject],
    ["web provenance", manifest.provenance?.web?.subject, webSubject],
    ["API scan", manifest.scans?.api?.subject, apiSubject],
    ["web scan", manifest.scans?.web?.subject, webSubject],
  ]) {
    invariant(actual === expected, `${label} subject mismatch`);
  }
}

async function verifyManifest(manifestPath, expectedMode) {
  const directory = path.dirname(path.resolve(manifestPath));
  const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
  const published = expectedMode === "published";
  invariant(
    expectedMode === "published" || expectedMode === "dry-run",
    "expected manifest mode is invalid",
  );
  invariant(
    manifest.schema_version === 1,
    "release manifest schema version is unsupported",
  );
  invariant(
    manifest.publication?.mode === expectedMode,
    "release manifest publication mode mismatch",
  );
  invariant(
    manifest.source?.repository ===
      "https://github.com/MuneebHoda/RAG-Debugger",
    "release manifest source repository mismatch",
  );
  invariant(
    sourceShaPattern.test(manifest.source?.commit ?? ""),
    "release manifest source commit is invalid",
  );
  invariant(
    manifest.source.commit ===
      (process.env.CORPUSLAB_RELEASE_SHA ?? manifest.source.commit),
    "release manifest source commit mismatch",
  );
  invariant(
    /^0\.[0-9]+\.[0-9]+(?:-rc\.[1-9][0-9]*)?$/.test(
      manifest.release?.application_version ?? "",
    ),
    "release manifest application version is invalid",
  );
  invariant(
    manifest.release.application_version ===
      (process.env.CORPUSLAB_VERSION ?? manifest.release.application_version),
    "release manifest application version mismatch",
  );
  invariant(
    manifest.release.version_tag === null,
    "base release manifest must not claim a mutable version selector",
  );
  invariant(
    digestPattern.test(manifest.api?.digest ?? ""),
    "release manifest API digest is invalid",
  );
  invariant(
    !manifest.api.full_commit_tag.endsWith(":latest"),
    "latest is not a permitted API selector",
  );
  if (published) {
    invariant(
      manifest.api.image === "ghcr.io/muneebhoda/rag-debugger",
      "published API image name mismatch",
    );
    invariant(
      manifest.api.reference === `${manifest.api.image}@${manifest.api.digest}`,
      "published API reference must use its digest",
    );
  } else {
    invariant(
      manifest.api.image === "corpuslab-api" && manifest.api.reference === null,
      "dry-run API identity must remain local",
    );
  }
  invariant(
    manifest.api.full_commit_tag ===
      `${manifest.api.image}:${manifest.source.commit}`,
    "API full-commit tag mismatch",
  );
  await verifyWebArtifact(directory, manifest.web);
  await verifyMigrationSet(directory, manifest.migrations);
  invariant(
    manifest.web.runtime_config_identity ===
      "excluded; generated and checksummed at deployment time",
    "runtime-config identity contract mismatch",
  );
  invariant(
    manifest.sboms?.format === "SPDX-2.3",
    "release manifest SBOM format mismatch",
  );
  validateArtifactSubjects(manifest, published);
  await verifySpdx(directory, manifest.sboms.api);
  await verifySpdx(directory, manifest.sboms.web);
  verifyAttestation(manifest.sboms.api.attestation, "API SBOM", published);
  verifyAttestation(manifest.sboms.web.attestation, "web SBOM", published);
  verifyAttestation(
    manifest.provenance?.api?.attestation,
    "API provenance",
    published,
  );
  verifyAttestation(
    manifest.provenance?.web?.attestation,
    "web provenance",
    published,
  );
  for (const scan of [manifest.scans?.api, manifest.scans?.web]) {
    invariant(
      path.basename(scan?.artifact ?? "") === scan?.artifact,
      "scan artifact name is invalid",
    );
    invariant(
      (await sha256File(path.join(directory, scan.artifact))) === scan.sha256,
      `${scan.artifact} checksum mismatch`,
    );
    await verifyScan(path.join(directory, scan.artifact));
  }
  invariant(
    manifest.scans.policy.includes("High/Critical") &&
      manifest.scans.policy.includes("any secret"),
    "release manifest scan policy is incomplete",
  );
  invariant(
    manifest.workflow?.name === "Publish deployment artifacts",
    "release manifest workflow identity mismatch",
  );
  invariant(
    /^[0-9]+$/.test(manifest.workflow?.run_id ?? ""),
    "release manifest workflow run ID is invalid",
  );
  invariant(
    Number.isInteger(manifest.workflow?.run_attempt) &&
      manifest.workflow.run_attempt > 0,
    "release manifest workflow attempt is invalid",
  );
  invariant(
    manifest.workflow.url ===
      `https://github.com/MuneebHoda/RAG-Debugger/actions/runs/${manifest.workflow.run_id}/attempts/${manifest.workflow.run_attempt}`,
    "release manifest workflow URL mismatch",
  );
  invariant(
    !forbiddenManifestKeys.test(JSON.stringify(manifest)),
    "release manifest contains a forbidden field",
  );
}

async function main() {
  const [command, ...args] = process.argv.slice(2);
  if (command === "prepare") {
    await prepare();
  } else if (command === "finalize") {
    await finalize();
  } else if (command === "create-manifest") {
    await createManifest();
  } else if (command === "verify-manifest") {
    invariant(
      args.length === 2,
      "verify-manifest requires a manifest path and mode",
    );
    await verifyManifest(args[0], args[1]);
  } else if (command === "verify-scan") {
    invariant(args.length === 1, "verify-scan requires a Trivy JSON report");
    await verifyScan(args[0]);
  } else {
    throw new Error(
      "usage: release-artifacts.mjs prepare|finalize|create-manifest|verify-manifest <path> <mode>|verify-scan <path>",
    );
  }
}

const invokedPath = process.argv[1] && path.resolve(process.argv[1]);
if (invokedPath === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(`Release artifact validation failed: ${error.message}`);
    process.exitCode = 1;
  });
}
