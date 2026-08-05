import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import { constants } from "node:fs";
import { cp, mkdir, open, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { promisify } from "node:util";

import {
  classifyPaths,
  containsSecretLikeValue,
  invariant,
  normalizeRepositoryPath,
  readJson,
  validateBuilderOutput,
  validateChangeSize,
} from "./core.mjs";

const execute = promisify(execFile);

async function git(arguments_, cwd) {
  const { stdout } = await execute("git", arguments_, {
    cwd,
    maxBuffer: 20 * 1024 * 1024,
  });
  return stdout;
}

export async function changedPaths(repositoryRoot) {
  const tracked = (
    await git(
      ["diff", "--name-only", "--no-renames", "-z", "HEAD"],
      repositoryRoot,
    )
  )
    .split("\0")
    .filter(Boolean);
  const untracked = (
    await git(
      ["ls-files", "--others", "--exclude-standard", "-z"],
      repositoryRoot,
    )
  )
    .split("\0")
    .filter(Boolean);
  return [...new Set([...tracked, ...untracked])]
    .map(normalizeRepositoryPath)
    .sort();
}

async function sha256(filePath) {
  return createHash("sha256")
    .update(await readFile(filePath))
    .digest("hex");
}

function sha256Bytes(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

async function readRegularFileNoFollow(filePath, errorMessage) {
  let handle;
  try {
    handle = await open(filePath, constants.O_RDONLY | constants.O_NOFOLLOW);
  } catch (error) {
    if (error.code === "ELOOP") throw new Error(errorMessage, { cause: error });
    throw error;
  }
  try {
    const metadata = await handle.stat();
    invariant(metadata.isFile(), errorMessage);
    return { metadata, bytes: await handle.readFile() };
  } finally {
    await handle.close();
  }
}

export async function captureCandidate({
  repositoryRoot,
  outputPath,
  contextPath,
  schemaPath,
  policyPath,
  artifactDirectory,
}) {
  const [output, context, schema, policy] = await Promise.all([
    readJson(outputPath),
    readJson(contextPath),
    readJson(schemaPath),
    readJson(policyPath),
  ]);
  const paths = await changedPaths(repositoryRoot);
  invariant(paths.length > 0, "builder produced no repository changes");
  validateBuilderOutput(output, schema, context, paths);

  const classification = classifyPaths(paths, policy);
  invariant(
    classification.artifacts.length === 0,
    `candidate contains generated artifacts: ${classification.artifacts.join(", ")}`,
  );
  invariant(
    classification.protected.length === 0,
    `candidate modifies protected paths: ${classification.protected.join(", ")}`,
  );

  await rm(artifactDirectory, { recursive: true, force: true });
  await mkdir(path.join(artifactDirectory, "files"), { recursive: true });
  const entries = [];
  let totalBytes = 0;

  for (const relativePath of paths) {
    const source = path.join(repositoryRoot, relativePath);
    let sourceFile;
    try {
      sourceFile = await readRegularFileNoFollow(
        source,
        `candidate path must be a regular file: ${relativePath}`,
      );
    } catch (error) {
      if (error.code === "ENOENT") {
        entries.push({
          path: relativePath,
          operation: "delete",
          mode: "100644",
          bytes: 0,
          sha256: null,
        });
        continue;
      }
      throw error;
    }
    const { metadata, bytes: sourceBytes } = sourceFile;
    invariant(
      sourceBytes.length <= policy.limits.single_file_bytes,
      `candidate file exceeds size limit: ${relativePath}`,
    );
    invariant(
      !sourceBytes.includes(0) || policy.generated_paths.includes(relativePath),
      `binary candidate is not an allowlisted generated path: ${relativePath}`,
    );
    if (!sourceBytes.includes(0)) {
      invariant(
        !containsSecretLikeValue(sourceBytes.toString("utf8")),
        `candidate content resembles a credential or secret: ${relativePath}`,
      );
    }
    totalBytes += sourceBytes.length;
    invariant(
      totalBytes <= policy.limits.artifact_bytes,
      "candidate artifact exceeds total size limit",
    );
    const destination = path.join(artifactDirectory, "files", relativePath);
    await mkdir(path.dirname(destination), { recursive: true });
    await writeFile(destination, sourceBytes);
    entries.push({
      path: relativePath,
      operation: "upsert",
      mode: (metadata.mode & 0o111) === 0 ? "100644" : "100755",
      bytes: sourceBytes.length,
      sha256: sha256Bytes(sourceBytes),
    });
  }

  const manifest = {
    version: 1,
    base_sha: context.base_sha,
    issue_number: context.issue.number,
    entries,
  };
  await writeFile(
    path.join(artifactDirectory, "manifest.json"),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );
  await cp(outputPath, path.join(artifactDirectory, "builder-output.json"));
  await cp(contextPath, path.join(artifactDirectory, "context.json"));
  return manifest;
}

function resolvedInside(root, relativePath) {
  const resolved = path.resolve(root, normalizeRepositoryPath(relativePath));
  invariant(
    resolved.startsWith(`${path.resolve(root)}${path.sep}`),
    `path escapes repository: ${relativePath}`,
  );
  return resolved;
}

export async function applyCandidate({ repositoryRoot, artifactDirectory }) {
  const manifest = await readJson(
    path.join(artifactDirectory, "manifest.json"),
  );
  invariant(
    manifest.version === 1 && Array.isArray(manifest.entries),
    "unsupported candidate manifest",
  );
  const seen = new Set();
  for (const entry of manifest.entries) {
    invariant(!seen.has(entry.path), `duplicate candidate path: ${entry.path}`);
    seen.add(entry.path);
    const destination = resolvedInside(repositoryRoot, entry.path);
    if (entry.operation === "delete") {
      await rm(destination, { force: true });
      continue;
    }
    invariant(
      entry.operation === "upsert" && ["100644", "100755"].includes(entry.mode),
      `invalid manifest entry: ${entry.path}`,
    );
    const source = resolvedInside(
      path.join(artifactDirectory, "files"),
      entry.path,
    );
    const { bytes } = await readRegularFileNoFollow(
      source,
      `artifact path must be a regular file: ${entry.path}`,
    );
    invariant(
      bytes.length === entry.bytes && sha256Bytes(bytes) === entry.sha256,
      `artifact integrity failed: ${entry.path}`,
    );
    await mkdir(path.dirname(destination), { recursive: true });
    await rm(destination, { force: true });
    await writeFile(destination, bytes);
    await import("node:fs/promises").then(({ chmod }) =>
      chmod(destination, entry.mode === "100755" ? 0o755 : 0o644),
    );
  }
  const actual = await changedPaths(repositoryRoot);
  invariant(
    JSON.stringify(actual) === JSON.stringify([...seen].sort()),
    "applied candidate differs from its manifest",
  );
  return manifest;
}

async function meaningfulLineCount(repositoryRoot, generatedPaths) {
  const output = await git(
    ["diff", "--numstat", "--no-renames", "HEAD"],
    repositoryRoot,
  );
  let lines = 0;
  for (const row of output.trim().split("\n").filter(Boolean)) {
    const [added, deleted, filePath] = row.split("\t");
    if (generatedPaths.has(filePath)) continue;
    lines +=
      added === "-" || deleted === "-" ? 0 : Number(added) + Number(deleted);
  }
  const untracked = await git(
    ["ls-files", "--others", "--exclude-standard", "-z"],
    repositoryRoot,
  );
  for (const filePath of untracked.split("\0").filter(Boolean)) {
    if (generatedPaths.has(filePath)) continue;
    const content = await readFile(path.join(repositoryRoot, filePath));
    lines += content.includes(0)
      ? 0
      : content.toString("utf8").split(/\r?\n/).length;
  }
  return lines;
}

export async function validateCandidate({
  repositoryRoot,
  artifactDirectory,
  trustedDirectory,
  attestationPath,
}) {
  const [manifest, output, context, policy, schema] = await Promise.all([
    readJson(path.join(artifactDirectory, "manifest.json")),
    readJson(path.join(artifactDirectory, "builder-output.json")),
    readJson(path.join(artifactDirectory, "context.json")),
    readJson(path.join(trustedDirectory, ".github/autonomy/policy.json")),
    readJson(
      path.join(
        trustedDirectory,
        ".github/autonomy/schemas/builder-output.schema.json",
      ),
    ),
  ]);
  invariant(
    manifest.base_sha === context.base_sha,
    "candidate base SHA does not match context",
  );
  const actualPaths = await changedPaths(repositoryRoot);
  validateBuilderOutput(output, schema, context, actualPaths);
  const classification = classifyPaths(actualPaths, policy);
  invariant(
    classification.protected.length === 0,
    `protected paths changed: ${classification.protected.join(", ")}`,
  );
  invariant(
    classification.artifacts.length === 0,
    `artifact paths changed: ${classification.artifacts.join(", ")}`,
  );

  const authorizedLabels = context.authorized_labels ?? [];
  if (classification.sensitive.length > 0) {
    invariant(
      authorizedLabels.includes(policy.labels.sensitive_approved),
      `sensitive paths require ${policy.labels.sensitive_approved}`,
    );
  }
  const generated = new Set(output.generated_or_mechanical_paths);
  for (const filePath of generated)
    invariant(
      policy.generated_paths.includes(filePath),
      `unrecognized generated path: ${filePath}`,
    );
  const meaningfulLines = await meaningfulLineCount(repositoryRoot, generated);
  validateChangeSize({
    fileCount: actualPaths.length,
    meaningfulLines,
    atomicity: output.atomicity,
    labels: authorizedLabels,
    policy,
  });

  const productionChanged = actualPaths.some((filePath) =>
    /^(?:apps\/(?:api|web)\/src|crates\/[^/]+\/src)\//u.test(filePath),
  );
  const testsChanged = actualPaths.some((filePath) =>
    /(?:^|\/)(?:tests?\/|[^/]+\.(?:test|spec)\.[^/]+$)|(?:^|\/)tests\.rs$/u.test(
      filePath,
    ),
  );
  if (productionChanged && !testsChanged)
    invariant(
      output.test_exception.trim().length >= 20,
      "production changes require tests or a substantive test_exception",
    );

  for (const entry of manifest.entries.filter(
    (item) => item.operation === "upsert",
  )) {
    const filePath = path.join(repositoryRoot, entry.path);
    const { bytes } = await readRegularFileNoFollow(
      filePath,
      `validated candidate must be a regular file: ${entry.path}`,
    );
    if (bytes.length > 0) {
      const content = bytes.includes(0) ? "" : bytes.toString("utf8");
      invariant(
        !containsSecretLikeValue(content),
        `candidate content resembles a credential or secret: ${entry.path}`,
      );
    }
  }

  const manifestHash = await sha256(
    path.join(artifactDirectory, "manifest.json"),
  );
  const outputHash = await sha256(
    path.join(artifactDirectory, "builder-output.json"),
  );
  const attestation = {
    version: 1,
    base_sha: context.base_sha,
    issue_number: context.issue.number,
    manifest_sha256: manifestHash,
    output_sha256: outputHash,
    file_count: actualPaths.length,
    meaningful_lines: meaningfulLines,
  };
  await writeFile(attestationPath, `${JSON.stringify(attestation, null, 2)}\n`);
  return attestation;
}

export async function verifyAttestation(artifactDirectory, attestationPath) {
  const [manifest, attestation] = await Promise.all([
    readJson(path.join(artifactDirectory, "manifest.json")),
    readJson(attestationPath),
  ]);
  invariant(
    attestation.manifest_sha256 ===
      (await sha256(path.join(artifactDirectory, "manifest.json"))),
    "manifest changed after validation",
  );
  invariant(
    attestation.output_sha256 ===
      (await sha256(path.join(artifactDirectory, "builder-output.json"))),
    "builder output changed after validation",
  );
  invariant(
    attestation.base_sha === manifest.base_sha &&
      attestation.issue_number === manifest.issue_number,
    "attestation identity mismatch",
  );
  return attestation;
}
