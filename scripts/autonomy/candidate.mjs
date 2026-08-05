import { createHash, randomUUID } from "node:crypto";
import { execFile } from "node:child_process";
import { constants } from "node:fs";
import {
  chmod,
  lstat,
  mkdir,
  open,
  rename,
  rm,
  writeFile,
} from "node:fs/promises";
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
  invariant(
    await safeExistingParent(root, normalized),
    options.errorMessage,
  );
  return readStableCandidateFile({
    ...options,
    filePath: resolvedInside(root, normalized),
  });
}

async function readStableJsonSnapshot(root, relativePath) {
  const { bytes } = await readStableCandidateSnapshot({
    root,
    relativePath,
    errorMessage: `candidate control file changed or is not regular: ${relativePath}`,
    maximumBytes: controlFileBytes,
  });
  return { value: JSON.parse(bytes.toString("utf8")), bytes };
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
  policyPath,
  artifactDirectory,
}) {
  const [schema, policy] = await Promise.all([
    readJson(schemaPath),
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
  validateSensitivePathAuthorization(
    classification.sensitive,
    context,
    policy,
  );

  await rm(artifactDirectory, { recursive: true, force: true });
  await mkdir(path.join(artifactDirectory, "files"), { recursive: true });
  const entries = [];
  let totalBytes = 0;

  for (const relativePath of paths) {
    const parentExists = await safeExistingParent(
      repositoryRoot,
      relativePath,
    );
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
  await writeSnapshot(
    artifactDirectory,
    "context.json",
    contextSnapshot.bytes,
  );
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
}) {
  const [manifestSnapshot, outputSnapshot, contextSnapshot, policy, schema] =
    await Promise.all([
      readStableJsonSnapshot(artifactDirectory, "manifest.json"),
      readStableJsonSnapshot(artifactDirectory, "builder-output.json"),
      readStableJsonSnapshot(artifactDirectory, "context.json"),
      readJson(path.join(trustedDirectory, ".github/autonomy/policy.json")),
      readJson(
        path.join(
          trustedDirectory,
          ".github/autonomy/schemas/builder-output.schema.json",
        ),
      ),
    ]);
  const manifest = manifestSnapshot.value;
  const output = outputSnapshot.value;
  const context = contextSnapshot.value;
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
  validateSensitivePathAuthorization(
    classification.sensitive,
    context,
    policy,
  );
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

  const attestation = {
    version: 1,
    base_sha: context.base_sha,
    issue_number: context.issue.number,
    manifest_sha256: sha256Bytes(manifestSnapshot.bytes),
    output_sha256: sha256Bytes(outputSnapshot.bytes),
    file_count: actualPaths.length,
    meaningful_lines: meaningfulLines,
  };
  await writeFile(attestationPath, `${JSON.stringify(attestation, null, 2)}\n`);
  return attestation;
}

export async function verifyAttestation(artifactDirectory, attestationPath) {
  const [manifestSnapshot, outputSnapshot, attestationSnapshot] =
    await Promise.all([
      readStableJsonSnapshot(artifactDirectory, "manifest.json"),
      readStableJsonSnapshot(artifactDirectory, "builder-output.json"),
      readStableCandidateFile({
        filePath: attestationPath,
        errorMessage: "candidate attestation changed or is not regular",
        maximumBytes: controlFileBytes,
      }),
    ]);
  const manifest = manifestSnapshot.value;
  const attestation = JSON.parse(attestationSnapshot.bytes.toString("utf8"));
  invariant(
    attestation.manifest_sha256 ===
      sha256Bytes(manifestSnapshot.bytes),
    "manifest changed after validation",
  );
  invariant(
    attestation.output_sha256 ===
      sha256Bytes(outputSnapshot.bytes),
    "builder output changed after validation",
  );
  invariant(
    attestation.base_sha === manifest.base_sha &&
      attestation.issue_number === manifest.issue_number,
    "attestation identity mismatch",
  );
  return attestation;
}
