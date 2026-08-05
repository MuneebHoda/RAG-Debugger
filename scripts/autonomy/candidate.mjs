import { createHash, randomUUID } from "node:crypto";
import { execFile } from "node:child_process";
import { constants } from "node:fs";
import { chmod, lstat, mkdir, open, rename, rm } from "node:fs/promises";
import path from "node:path";
import { promisify } from "node:util";

import {
  canonicalJsonBytes,
  classifyPaths,
  containsSecretLikeValue,
  invariant,
  normalizeRepositoryPath,
  readJson,
  validateBuilderContext,
  validateBuilderOutput,
  validateChangeSize,
  validateJsonSchema,
  validateSensitivePathAuthorization,
} from "./core.mjs";

const execute = promisify(execFile);
const controlFileBytes = 1024 * 1024;

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

function sha256Bytes(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function sameFileVersion(left, right) {
  return (
    left.dev === right.dev &&
    left.ino === right.ino &&
    left.mode === right.mode &&
    left.nlink === right.nlink &&
    left.size === right.size &&
    left.mtimeNs === right.mtimeNs &&
    left.ctimeNs === right.ctimeNs
  );
}

async function readBounded(handle, maximumBytes, errorMessage) {
  const chunks = [];
  let total = 0;
  while (total <= maximumBytes) {
    const remaining = maximumBytes + 1 - total;
    const buffer = Buffer.allocUnsafe(Math.min(64 * 1024, remaining));
    const { bytesRead } = await handle.read(buffer, 0, buffer.length, null);
    if (bytesRead === 0) break;
    chunks.push(buffer.subarray(0, bytesRead));
    total += bytesRead;
  }
  invariant(total <= maximumBytes, errorMessage);
  return Buffer.concat(chunks, total);
}

export async function readStableCandidateFile({
  filePath,
  errorMessage,
  maximumBytes,
  expectedBytes,
  expectedSha256,
  afterRead,
}) {
  invariant(
    Number.isSafeInteger(maximumBytes) && maximumBytes >= 0,
    "candidate read requires a finite byte limit",
  );
  let handle;
  try {
    handle = await open(filePath, constants.O_RDONLY | constants.O_NOFOLLOW);
  } catch (error) {
    if (error.code === "ELOOP") throw new Error(errorMessage, { cause: error });
    throw error;
  }
  try {
    const before = await handle.stat({ bigint: true });
    invariant(before.isFile(), errorMessage);
    invariant(before.size <= BigInt(maximumBytes), errorMessage);
    const bytes = await readBounded(handle, maximumBytes, errorMessage);
    if (afterRead) await afterRead();
    const after = await handle.stat({ bigint: true });
    invariant(after.isFile() && sameFileVersion(before, after), errorMessage);
    let current;
    try {
      current = await lstat(filePath, { bigint: true });
    } catch (error) {
      throw new Error(errorMessage, { cause: error });
    }
    invariant(
      current.isFile() && sameFileVersion(after, current),
      errorMessage,
    );
    invariant(BigInt(bytes.length) === after.size, errorMessage);
    if (expectedBytes !== undefined)
      invariant(bytes.length === expectedBytes, errorMessage);
    const digest = sha256Bytes(bytes);
    if (expectedSha256 !== undefined)
      invariant(digest === expectedSha256, errorMessage);
    return {
      metadata: {
        mode: Number(after.mode),
        size: Number(after.size),
      },
      bytes,
      sha256: digest,
    };
  } finally {
    await handle.close();
  }
}

async function ensureSafeParent(root, relativePath) {
  const parentParts = path.posix.dirname(relativePath).split("/");
  let current = path.resolve(root);
  for (const part of parentParts) {
    if (part === ".") continue;
    current = path.join(current, part);
    try {
      const metadata = await lstat(current);
      invariant(
        metadata.isDirectory() && !metadata.isSymbolicLink(),
        `candidate parent must be a real directory: ${relativePath}`,
      );
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
      await mkdir(current);
      const metadata = await lstat(current);
      invariant(
        metadata.isDirectory() && !metadata.isSymbolicLink(),
        `candidate parent must be a real directory: ${relativePath}`,
      );
    }
  }
}

async function safeExistingParent(root, relativePath) {
  const parentParts = path.posix.dirname(relativePath).split("/");
  let current = path.resolve(root);
  for (const part of parentParts) {
    if (part === ".") continue;
    current = path.join(current, part);
    let metadata;
    try {
      metadata = await lstat(current);
    } catch (error) {
      if (error.code === "ENOENT") return false;
      throw error;
    }
    invariant(
      metadata.isDirectory() && !metadata.isSymbolicLink(),
      `candidate parent must be a real directory: ${relativePath}`,
    );
  }
  return true;
}

export async function readStableCandidateSnapshot({
  root,
  relativePath,
  ...options
}) {
  const normalized = normalizeRepositoryPath(relativePath);
  invariant(await safeExistingParent(root, normalized), options.errorMessage);
  return readStableCandidateFile({
    ...options,
    filePath: resolvedInside(root, normalized),
  });
}

async function readStableJsonSnapshot(
  root,
  relativePath,
  { canonical = false } = {},
) {
  const { bytes } = await readStableCandidateSnapshot({
    root,
    relativePath,
    errorMessage: `candidate control file changed or is not regular: ${relativePath}`,
    maximumBytes: controlFileBytes,
  });
  const value = JSON.parse(bytes.toString("utf8"));
  if (canonical)
    invariant(
      bytes.equals(canonicalJsonBytes(value)),
      `candidate control file is not canonical JSON: ${relativePath}`,
    );
  return { value, bytes };
}

async function writeSnapshot(root, relativePath, bytes, mode = 0o644) {
  const destination = resolvedInside(root, relativePath);
  await ensureSafeParent(root, relativePath);
  const temporary = path.join(
    path.dirname(destination),
    `.${path.basename(destination)}.autonomy-${randomUUID()}`,
  );
  let handle;
  try {
    handle = await open(
      temporary,
      constants.O_CREAT |
        constants.O_EXCL |
        constants.O_WRONLY |
        constants.O_NOFOLLOW,
      mode,
    );
    await handle.writeFile(bytes);
    await handle.sync();
    await handle.close();
    handle = undefined;
    await chmod(temporary, mode);
    await rename(temporary, destination);
    return;
  } catch (error) {
    await handle?.close();
    await rm(temporary, { force: true });
    throw error;
  }
}

export async function captureCandidate({
  repositoryRoot,
  outputPath,
  contextPath,
  schemaPath,
  contextSchemaPath,
  policyPath,
  artifactDirectory,
}) {
  const [schema, contextSchema, policy] = await Promise.all([
    readJson(schemaPath),
    readJson(contextSchemaPath),
    readJson(policyPath),
  ]);
  const [outputSnapshot, contextSnapshot] = await Promise.all([
    readStableCandidateFile({
      filePath: outputPath,
      errorMessage: "builder output changed while being captured",
      maximumBytes: policy.limits.single_file_bytes,
    }),
    readStableCandidateFile({
      filePath: contextPath,
      errorMessage: "builder context changed while being captured",
      maximumBytes: policy.limits.single_file_bytes,
    }),
  ]);
  const output = JSON.parse(outputSnapshot.bytes.toString("utf8"));
  const context = JSON.parse(contextSnapshot.bytes.toString("utf8"));
  invariant(
    contextSnapshot.bytes.equals(canonicalJsonBytes(context)),
    "builder context is not canonical JSON",
  );
  validateBuilderContext(context, contextSchema, policy);
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
  validateSensitivePathAuthorization(classification.sensitive, context, policy);

  await rm(artifactDirectory, { recursive: true, force: true });
  await mkdir(path.join(artifactDirectory, "files"), { recursive: true });
  const entries = [];
  let totalBytes = 0;

  for (const relativePath of paths) {
    const parentExists = await safeExistingParent(repositoryRoot, relativePath);
    if (!parentExists) {
      entries.push({
        path: relativePath,
        operation: "delete",
        mode: "100644",
        bytes: 0,
        sha256: null,
      });
      continue;
    }
    let sourceFile;
    try {
      sourceFile = await readStableCandidateSnapshot({
        root: repositoryRoot,
        relativePath,
        errorMessage: `candidate path changed or is not a stable regular file: ${relativePath}`,
        maximumBytes: policy.limits.single_file_bytes,
      });
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
    await writeSnapshot(
      artifactDirectory,
      path.posix.join("files", relativePath),
      sourceBytes,
    );
    entries.push({
      path: relativePath,
      operation: "upsert",
      mode: (metadata.mode & 0o111) === 0 ? "100644" : "100755",
      bytes: sourceBytes.length,
      sha256: sourceFile.sha256,
    });
  }

  const manifest = {
    version: 1,
    base_sha: context.base_sha,
    issue_number: context.issue.number,
    entries,
  };
  await writeSnapshot(
    artifactDirectory,
    "manifest.json",
    Buffer.from(`${JSON.stringify(manifest, null, 2)}\n`),
  );
  await writeSnapshot(
    artifactDirectory,
    "builder-output.json",
    outputSnapshot.bytes,
  );
  await writeSnapshot(artifactDirectory, "context.json", contextSnapshot.bytes);
  return manifest;
}

export async function verifyBuilderContextFile({
  contextPath,
  trustedDirectory,
  expectedContext = {},
}) {
  const [snapshot, policy, schema] = await Promise.all([
    readStableCandidateFile({
      filePath: contextPath,
      errorMessage: "builder context changed or is not a regular file",
      maximumBytes: controlFileBytes,
    }),
    readJson(path.join(trustedDirectory, ".github/autonomy/policy.json")),
    readJson(
      path.join(
        trustedDirectory,
        ".github/autonomy/schemas/builder-context.schema.json",
      ),
    ),
  ]);
  const context = JSON.parse(snapshot.bytes.toString("utf8"));
  invariant(
    snapshot.bytes.equals(canonicalJsonBytes(context)),
    "builder context is not canonical JSON",
  );
  validateBuilderContext(context, schema, policy, expectedContext);
  if (expectedContext.contextSha256 !== undefined)
    invariant(
      snapshot.sha256 === expectedContext.contextSha256,
      "builder context digest does not match trusted preflight output",
    );
  return { context, sha256: snapshot.sha256 };
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
  const { value: manifest } = await readStableJsonSnapshot(
    artifactDirectory,
    "manifest.json",
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
    await ensureSafeParent(repositoryRoot, entry.path);
    if (entry.operation === "delete") {
      await rm(destination, { force: true });
      continue;
    }
    invariant(
      entry.operation === "upsert" && ["100644", "100755"].includes(entry.mode),
      `invalid manifest entry: ${entry.path}`,
    );
    const { bytes } = await readStableCandidateSnapshot({
      root: path.join(artifactDirectory, "files"),
      relativePath: entry.path,
      errorMessage: `artifact path is not a stable regular file or failed integrity: ${entry.path}`,
      maximumBytes: entry.bytes,
      expectedBytes: entry.bytes,
      expectedSha256: entry.sha256,
    });
    await writeSnapshot(
      repositoryRoot,
      entry.path,
      bytes,
      entry.mode === "100755" ? 0o755 : 0o644,
    );
  }
  const actual = await changedPaths(repositoryRoot);
  invariant(
    JSON.stringify(actual) === JSON.stringify([...seen].sort()),
    "applied candidate differs from its manifest",
  );
  return manifest;
}

async function meaningfulLineCount(
  repositoryRoot,
  generatedPaths,
  maximumFileBytes,
) {
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
    const { bytes: content } = await readStableCandidateSnapshot({
      root: repositoryRoot,
      relativePath: filePath,
      errorMessage: `untracked candidate changed during line counting: ${filePath}`,
      maximumBytes: maximumFileBytes,
    });
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
  expectedContext = {},
}) {
  const schemaRoot = path.join(trustedDirectory, ".github/autonomy/schemas");
  const [
    manifestSnapshot,
    outputSnapshot,
    contextSnapshot,
    policy,
    outputSchema,
    contextSchema,
    manifestSchema,
  ] = await Promise.all([
    readStableJsonSnapshot(artifactDirectory, "manifest.json", {
      canonical: true,
    }),
    readStableJsonSnapshot(artifactDirectory, "builder-output.json"),
    readStableJsonSnapshot(artifactDirectory, "context.json", {
      canonical: true,
    }),
    readJson(path.join(trustedDirectory, ".github/autonomy/policy.json")),
    readJson(path.join(schemaRoot, "builder-output.schema.json")),
    readJson(path.join(schemaRoot, "builder-context.schema.json")),
    readJson(path.join(schemaRoot, "candidate-manifest.schema.json")),
  ]);
  const manifest = manifestSnapshot.value;
  const output = outputSnapshot.value;
  const context = contextSnapshot.value;
  validateJsonSchema(manifestSchema, manifest);
  validateBuilderContext(context, contextSchema, policy, expectedContext);
  if (expectedContext.contextSha256 !== undefined)
    invariant(
      sha256Bytes(contextSnapshot.bytes) === expectedContext.contextSha256,
      "builder context digest does not match trusted preflight output",
    );
  invariant(
    manifest.base_sha === context.base_sha,
    "candidate base SHA does not match context",
  );
  invariant(
    manifest.issue_number === context.issue.number,
    "candidate issue does not match context",
  );
  invariant(
    (await git(["rev-parse", "HEAD"], repositoryRoot)).trim() ===
      context.base_sha,
    "candidate repository is not checked out at the exact base SHA",
  );
  const actualPaths = await changedPaths(repositoryRoot);
  const manifestPaths = [];
  const manifestSeen = new Set();
  for (const entry of manifest.entries) {
    const normalized = normalizeRepositoryPath(entry.path);
    invariant(
      !manifestSeen.has(normalized),
      `duplicate candidate path: ${normalized}`,
    );
    manifestSeen.add(normalized);
    manifestPaths.push(normalized);
    if (entry.operation === "delete")
      invariant(
        entry.bytes === 0 && entry.sha256 === null && entry.mode === "100644",
        `invalid deletion manifest entry: ${entry.path}`,
      );
    else
      invariant(
        entry.sha256 !== null,
        `upsert manifest entry lacks a digest: ${entry.path}`,
      );
  }
  invariant(
    JSON.stringify([...manifestPaths].sort()) === JSON.stringify(actualPaths),
    "candidate manifest does not exactly match the applied diff",
  );
  validateBuilderOutput(output, outputSchema, context, actualPaths);
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
  validateSensitivePathAuthorization(classification.sensitive, context, policy);
  const generated = new Set(output.generated_or_mechanical_paths);
  for (const filePath of generated)
    invariant(
      policy.generated_paths.includes(filePath),
      `unrecognized generated path: ${filePath}`,
    );
  const meaningfulLines = await meaningfulLineCount(
    repositoryRoot,
    generated,
    policy.limits.single_file_bytes,
  );
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
    const { bytes } = await readStableCandidateSnapshot({
      root: repositoryRoot,
      relativePath: entry.path,
      errorMessage: `validated candidate changed or is not a regular file: ${entry.path}`,
      maximumBytes: entry.bytes,
      expectedBytes: entry.bytes,
      expectedSha256: entry.sha256,
    });
    if (bytes.length > 0) {
      const content = bytes.includes(0) ? "" : bytes.toString("utf8");
      invariant(
        !containsSecretLikeValue(content),
        `candidate content resembles a credential or secret: ${entry.path}`,
      );
    }
  }

  const authorization = {
    authorized_labels: context.authorized_labels,
    bootstrap_authorization: context.bootstrap_authorization,
  };
  const attestation = {
    version: 2,
    base_sha: context.base_sha,
    issue_number: context.issue.number,
    context_sha256: sha256Bytes(contextSnapshot.bytes),
    manifest_sha256: sha256Bytes(manifestSnapshot.bytes),
    output_sha256: sha256Bytes(outputSnapshot.bytes),
    authorization_sha256: sha256Bytes(canonicalJsonBytes(authorization)),
    publication: {
      repository: context.repository,
      ...context.publication,
    },
    files: manifest.entries,
    file_count: actualPaths.length,
    meaningful_lines: meaningfulLines,
  };
  await writeSnapshot(
    path.dirname(attestationPath),
    path.basename(attestationPath),
    canonicalJsonBytes(attestation),
  );
  return attestation;
}

export async function verifyAttestation({
  artifactDirectory,
  attestationPath,
  trustedDirectory,
  expectedContext = {},
}) {
  const schemaRoot = path.join(trustedDirectory, ".github/autonomy/schemas");
  const [
    manifestSnapshot,
    outputSnapshot,
    contextSnapshot,
    attestationSnapshot,
    policy,
    contextSchema,
    manifestSchema,
    attestationSchema,
  ] = await Promise.all([
    readStableJsonSnapshot(artifactDirectory, "manifest.json", {
      canonical: true,
    }),
    readStableJsonSnapshot(artifactDirectory, "builder-output.json"),
    readStableJsonSnapshot(artifactDirectory, "context.json", {
      canonical: true,
    }),
    readStableCandidateFile({
      filePath: attestationPath,
      errorMessage: "candidate attestation changed or is not regular",
      maximumBytes: controlFileBytes,
    }),
    readJson(path.join(trustedDirectory, ".github/autonomy/policy.json")),
    readJson(path.join(schemaRoot, "builder-context.schema.json")),
    readJson(path.join(schemaRoot, "candidate-manifest.schema.json")),
    readJson(path.join(schemaRoot, "candidate-attestation.schema.json")),
  ]);
  const manifest = manifestSnapshot.value;
  const context = contextSnapshot.value;
  const attestation = JSON.parse(attestationSnapshot.bytes.toString("utf8"));
  invariant(
    attestationSnapshot.bytes.equals(canonicalJsonBytes(attestation)),
    "candidate attestation is not canonical JSON",
  );
  validateJsonSchema(manifestSchema, manifest);
  validateBuilderContext(context, contextSchema, policy, expectedContext);
  validateJsonSchema(attestationSchema, attestation);
  if (expectedContext.contextSha256 !== undefined)
    invariant(
      sha256Bytes(contextSnapshot.bytes) === expectedContext.contextSha256,
      "builder context digest does not match trusted preflight output",
    );
  invariant(
    attestation.context_sha256 === sha256Bytes(contextSnapshot.bytes),
    "builder context changed after validation",
  );
  invariant(
    attestation.manifest_sha256 === sha256Bytes(manifestSnapshot.bytes),
    "manifest changed after validation",
  );
  invariant(
    attestation.output_sha256 === sha256Bytes(outputSnapshot.bytes),
    "builder output changed after validation",
  );
  invariant(
    attestation.base_sha === manifest.base_sha &&
      attestation.issue_number === manifest.issue_number &&
      manifest.base_sha === context.base_sha &&
      manifest.issue_number === context.issue.number,
    "attestation identity mismatch",
  );
  invariant(
    attestation.authorization_sha256 ===
      sha256Bytes(
        canonicalJsonBytes({
          authorized_labels: context.authorized_labels,
          bootstrap_authorization: context.bootstrap_authorization,
        }),
      ),
    "candidate authorization changed after validation",
  );
  invariant(
    JSON.stringify(attestation.publication) ===
      JSON.stringify({
        repository: context.repository,
        ...context.publication,
      }),
    "candidate publication metadata changed after validation",
  );
  invariant(
    JSON.stringify(attestation.files) === JSON.stringify(manifest.entries),
    "attested candidate files do not match the manifest",
  );
  for (const entry of manifest.entries.filter(
    (item) => item.operation === "upsert",
  ))
    await readStableCandidateSnapshot({
      root: path.join(artifactDirectory, "files"),
      relativePath: entry.path,
      errorMessage: `attested candidate file changed: ${entry.path}`,
      maximumBytes: entry.bytes,
      expectedBytes: entry.bytes,
      expectedSha256: entry.sha256,
    });
  return {
    attestation,
    context,
    manifest,
    output: outputSnapshot.value,
  };
}

export function verifyArtifactIdentity({
  expectedId,
  observedId,
  expectedDigest,
  observedDigest,
}) {
  invariant(
    /^[1-9][0-9]*$/u.test(String(expectedId)),
    "invalid expected artifact ID",
  );
  invariant(
    String(observedId) === String(expectedId),
    "downloaded artifact ID does not match the validated artifact",
  );
  invariant(
    /^[0-9a-f]{64}$/u.test(String(expectedDigest)),
    "invalid expected artifact digest",
  );
  invariant(
    String(observedDigest) === String(expectedDigest),
    "downloaded artifact digest does not match the validated artifact",
  );
}

export async function sealCandidate({
  artifactDirectory,
  sealedPath,
  trustedDirectory,
  expectedContext = {},
}) {
  const manifestSnapshot = await readStableJsonSnapshot(
    artifactDirectory,
    "manifest.json",
    { canonical: true },
  );
  await verifyAttestation({
    artifactDirectory,
    attestationPath: path.join(artifactDirectory, "attestation.json"),
    trustedDirectory,
    expectedContext,
  });
  const paths = [
    "attestation.json",
    "builder-output.json",
    "context.json",
    "manifest.json",
    ...manifestSnapshot.value.entries
      .filter((entry) => entry.operation === "upsert")
      .map((entry) => `files/${entry.path}`),
  ].sort();
  const entries = [];
  for (const relativePath of paths) {
    const maximumBytes = relativePath.startsWith("files/")
      ? manifestSnapshot.value.entries.find(
          (entry) => `files/${entry.path}` === relativePath,
        ).bytes
      : controlFileBytes;
    const snapshot = await readStableCandidateSnapshot({
      root: artifactDirectory,
      relativePath,
      errorMessage: `sealed candidate input changed: ${relativePath}`,
      maximumBytes,
    });
    entries.push({
      path: relativePath,
      bytes: snapshot.bytes.length,
      sha256: snapshot.sha256,
      content_base64: snapshot.bytes.toString("base64"),
    });
  }
  const sealed = { version: 1, entries };
  const sealedSchema = await readJson(
    path.join(
      trustedDirectory,
      ".github/autonomy/schemas/sealed-candidate.schema.json",
    ),
  );
  validateJsonSchema(sealedSchema, sealed);
  const bytes = canonicalJsonBytes(sealed);
  await writeSnapshot(
    path.dirname(sealedPath),
    path.basename(sealedPath),
    bytes,
  );
  return { bytes: bytes.length, sha256: sha256Bytes(bytes) };
}

export async function extractSealedCandidate({
  sealedPath,
  artifactDirectory,
  trustedDirectory,
  expectedDigest,
}) {
  const policy = await readJson(
    path.join(trustedDirectory, ".github/autonomy/policy.json"),
  );
  const maximumBundleBytes =
    Math.ceil((policy.limits.artifact_bytes * 4) / 3) + 4 * 1024 * 1024;
  const sealedSnapshot = await readStableCandidateFile({
    filePath: sealedPath,
    errorMessage: "sealed candidate changed or is not a regular file",
    maximumBytes: maximumBundleBytes,
    expectedSha256: expectedDigest,
  });
  const sealed = JSON.parse(sealedSnapshot.bytes.toString("utf8"));
  invariant(
    sealedSnapshot.bytes.equals(canonicalJsonBytes(sealed)),
    "sealed candidate is not canonical JSON",
  );
  const sealedSchema = await readJson(
    path.join(
      trustedDirectory,
      ".github/autonomy/schemas/sealed-candidate.schema.json",
    ),
  );
  validateJsonSchema(sealedSchema, sealed);
  const seen = new Set();
  let decodedBytes = 0;
  await rm(artifactDirectory, { recursive: true, force: true });
  await mkdir(artifactDirectory, { recursive: true });
  for (const entry of sealed.entries) {
    invariant(!seen.has(entry.path), `duplicate sealed path: ${entry.path}`);
    seen.add(entry.path);
    const allowedControl = [
      "attestation.json",
      "builder-output.json",
      "context.json",
      "manifest.json",
    ].includes(entry.path);
    const candidatePath = entry.path.startsWith("files/")
      ? normalizeRepositoryPath(entry.path.slice("files/".length))
      : null;
    invariant(
      allowedControl || candidatePath !== null,
      `unexpected sealed candidate path: ${entry.path}`,
    );
    if (candidatePath !== null)
      invariant(
        entry.path === `files/${candidatePath}`,
        `non-canonical sealed candidate path: ${entry.path}`,
      );
    const bytes = Buffer.from(entry.content_base64, "base64");
    invariant(
      bytes.toString("base64") === entry.content_base64 &&
        bytes.length === entry.bytes &&
        sha256Bytes(bytes) === entry.sha256,
      `sealed candidate entry failed integrity: ${entry.path}`,
    );
    decodedBytes += bytes.length;
    invariant(
      decodedBytes <= policy.limits.artifact_bytes + 4 * controlFileBytes,
      "sealed candidate exceeds the decoded size limit",
    );
    await writeSnapshot(artifactDirectory, entry.path, bytes);
  }
  for (const required of [
    "attestation.json",
    "builder-output.json",
    "context.json",
    "manifest.json",
  ])
    invariant(seen.has(required), `sealed candidate is missing ${required}`);
  return sealedSnapshot.sha256;
}

export async function revalidateSealedCandidate({
  repositoryRoot,
  sealedPath,
  artifactDirectory,
  trustedDirectory,
  expectedArtifact,
  expectedContext,
}) {
  verifyArtifactIdentity(expectedArtifact);
  await extractSealedCandidate({
    sealedPath,
    artifactDirectory,
    trustedDirectory,
    expectedDigest: expectedArtifact.expectedDigest,
  });
  await verifyAttestation({
    artifactDirectory,
    attestationPath: path.join(artifactDirectory, "attestation.json"),
    trustedDirectory,
    expectedContext,
  });
  const originalAttestation = await readStableCandidateSnapshot({
    root: artifactDirectory,
    relativePath: "attestation.json",
    errorMessage: "sealed attestation changed before revalidation",
    maximumBytes: controlFileBytes,
  });
  await applyCandidate({ repositoryRoot, artifactDirectory });
  const revalidatedPath = path.join(
    path.dirname(artifactDirectory),
    `revalidated-${randomUUID()}.json`,
  );
  await validateCandidate({
    repositoryRoot,
    artifactDirectory,
    trustedDirectory,
    attestationPath: revalidatedPath,
    expectedContext,
  });
  const revalidated = await readStableCandidateFile({
    filePath: revalidatedPath,
    errorMessage: "revalidated attestation changed",
    maximumBytes: controlFileBytes,
  });
  invariant(
    originalAttestation.bytes.equals(revalidated.bytes),
    "publisher revalidation does not match the sealed attestation",
  );
  await verifyAttestation({
    artifactDirectory,
    attestationPath: path.join(artifactDirectory, "attestation.json"),
    trustedDirectory,
    expectedContext,
  });
  await rm(revalidatedPath, { force: true });
  return JSON.parse(originalAttestation.bytes.toString("utf8"));
}
