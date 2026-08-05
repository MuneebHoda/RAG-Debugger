import assert from "node:assert/strict";
import { Buffer } from "node:buffer";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import {
  cp,
  lstat,
  mkdir,
  mkdtemp,
  readFile,
  rm,
  symlink,
  writeFile,
} from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  canonicalJsonBytes,
  createBuilderContext,
} from "../../../scripts/autonomy/core.mjs";
import {
  applyCandidate,
  captureCandidate,
  extractSealedCandidate,
  revalidateSealedCandidate,
  sealCandidate,
  validateCandidate,
  verifyArtifactIdentity,
} from "../../../scripts/autonomy/candidate.mjs";

const repositoryRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../../..",
);
const schemaRoot = path.join(repositoryRoot, ".github/autonomy/schemas");
const policy = JSON.parse(
  await readFile(
    path.join(repositoryRoot, ".github/autonomy/policy.json"),
    "utf8",
  ),
);

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function git(root, arguments_) {
  return execFileSync("git", arguments_, { cwd: root, encoding: "utf8" });
}

async function initializeRepository() {
  const root = await mkdtemp(path.join(os.tmpdir(), "corpuslab-sealed-base-"));
  git(root, ["init", "-q"]);
  git(root, ["config", "user.email", "fixture@example.invalid"]);
  git(root, ["config", "user.name", "Autonomy Fixture"]);
  await writeFile(path.join(root, "README.md"), "sealed fixture\n");
  git(root, ["add", "README.md"]);
  git(root, ["commit", "-qm", "test: create sealed base"]);
  return root;
}

function builderOutput(filesChanged) {
  return {
    issue_number: 99,
    plan: ["Add one deterministic artifact-boundary fixture and validate it."],
    summary:
      "Added a deterministic artifact-boundary fixture without changing product runtime behavior.",
    files_changed: filesChanged,
    tests_added: ["Covered sealed artifact publication revalidation."],
    validation_commands: ["just autonomy-check"],
    documentation_updated: [],
    risks: [
      "The workflow contract must remain synchronized with its fixtures.",
    ],
    rollback:
      "Revert the focused automation fixture and trust-boundary change.",
    follow_ups: [],
    atomicity: { justification: "", testing: "", rollback: "" },
    generated_or_mechanical_paths: [],
    test_exception: "The candidate changes test documentation only.",
  };
}

async function buildSealedFixture({ deleteOnly = false } = {}) {
  const root = await initializeRepository();
  const baseSha = git(root, ["rev-parse", "HEAD"]).trim();
  const control = await mkdtemp(
    path.join(os.tmpdir(), "corpuslab-sealed-control-"),
  );
  const context = createBuilderContext({
    baseSha,
    trigger: "approval_label",
    repository: "MuneebHoda/RAG-Debugger",
    runUrl: "https://github.com/MuneebHoda/RAG-Debugger/actions/runs/100",
    authorizedLabels: [policy.labels.approved],
    issue: {
      number: 99,
      title: "test: validate sealed candidate",
      body: "Exercise the immutable candidate publication boundary.",
      labels: ["type/test"],
    },
    policy,
  });
  const contextBytes = canonicalJsonBytes(context);
  const expectedContext = {
    baseSha,
    trigger: "approval_label",
    repository: "MuneebHoda/RAG-Debugger",
    runUrl: "https://github.com/MuneebHoda/RAG-Debugger/actions/runs/100",
    issueNumber: 99,
    contextSha256: sha256(contextBytes),
  };
  const changedPath = deleteOnly ? "README.md" : "docs/example.md";
  if (deleteOnly) await rm(path.join(root, changedPath));
  else {
    await mkdir(path.join(root, "docs"), { recursive: true });
    await writeFile(path.join(root, changedPath), "validated bytes\n");
  }
  const outputPath = path.join(control, "builder-output.json");
  const contextPath = path.join(control, "builder-context.json");
  await writeFile(outputPath, JSON.stringify(builderOutput([changedPath])));
  await writeFile(contextPath, contextBytes);
  const candidate = path.join(control, "candidate");
  await captureCandidate({
    repositoryRoot: root,
    outputPath,
    contextPath,
    schemaPath: path.join(schemaRoot, "builder-output.schema.json"),
    contextSchemaPath: path.join(schemaRoot, "builder-context.schema.json"),
    policyPath: path.join(repositoryRoot, ".github/autonomy/policy.json"),
    artifactDirectory: candidate,
  });
  git(root, ["reset", "--hard", "-q", baseSha]);
  git(root, ["clean", "-fdq"]);
  await applyCandidate({ repositoryRoot: root, artifactDirectory: candidate });
  await validateCandidate({
    repositoryRoot: root,
    artifactDirectory: candidate,
    trustedDirectory: repositoryRoot,
    attestationPath: path.join(candidate, "attestation.json"),
    expectedContext,
  });
  const sealedPath = path.join(control, "builder-sealed.json");
  const sealed = await sealCandidate({
    artifactDirectory: candidate,
    sealedPath,
    trustedDirectory: repositoryRoot,
    expectedContext,
  });
  git(root, ["reset", "--hard", "-q", baseSha]);
  git(root, ["clean", "-fdq"]);
  return { baseSha, control, expectedContext, root, sealed, sealedPath };
}

async function cloneFresh(source, suffix) {
  const parent = await mkdtemp(path.join(os.tmpdir(), `corpuslab-${suffix}-`));
  const destination = path.join(parent, "repository");
  execFileSync("git", ["clone", "-q", "--no-hardlinks", source, destination]);
  return destination;
}

async function mutateSealed(source, destination, entryPath, mutate) {
  const sealed = JSON.parse(await readFile(source, "utf8"));
  const entry = sealed.entries.find(
    (candidate) => candidate.path === entryPath,
  );
  assert.ok(entry, `missing sealed entry ${entryPath}`);
  const current = Buffer.from(entry.content_base64, "base64");
  const next = await mutate(current, sealed);
  entry.content_base64 = next.toString("base64");
  entry.bytes = next.length;
  entry.sha256 = sha256(next);
  const bytes = canonicalJsonBytes(sealed);
  await writeFile(destination, bytes);
  return sha256(bytes);
}

async function forgeSealed(source, destination, modify) {
  const sealed = JSON.parse(await readFile(source, "utf8"));
  const values = new Map(
    sealed.entries.map((entry) => [
      entry.path,
      Buffer.from(entry.content_base64, "base64"),
    ]),
  );
  const state = {
    attestation: JSON.parse(values.get("attestation.json").toString("utf8")),
    context: JSON.parse(values.get("context.json").toString("utf8")),
    manifest: JSON.parse(values.get("manifest.json").toString("utf8")),
    output: JSON.parse(values.get("builder-output.json").toString("utf8")),
    files: values,
  };
  await modify(state);
  const manifestBytes = canonicalJsonBytes(state.manifest);
  const outputBytes = canonicalJsonBytes(state.output);
  const contextBytes = canonicalJsonBytes(state.context);
  state.attestation.context_sha256 = sha256(contextBytes);
  state.attestation.manifest_sha256 = sha256(manifestBytes);
  state.attestation.output_sha256 = sha256(outputBytes);
  state.attestation.authorization_sha256 = sha256(
    canonicalJsonBytes({
      authorized_labels: state.context.authorized_labels,
      bootstrap_authorization: state.context.bootstrap_authorization,
    }),
  );
  state.attestation.publication = {
    repository: state.context.repository,
    ...state.context.publication,
  };
  state.attestation.files = state.manifest.entries;
  state.attestation.file_count = state.manifest.entries.length;
  values.set("manifest.json", manifestBytes);
  values.set("builder-output.json", outputBytes);
  values.set("context.json", contextBytes);
  values.set("attestation.json", canonicalJsonBytes(state.attestation));
  const requiredPaths = [
    "attestation.json",
    "builder-output.json",
    "context.json",
    "manifest.json",
    ...state.manifest.entries
      .filter((entry) => entry.operation === "upsert")
      .map((entry) => `files/${entry.path}`),
  ].sort();
  const entries = requiredPaths.map((entryPath) => {
    const bytes = values.get(entryPath);
    assert.ok(bytes, `forged bundle lacks ${entryPath}`);
    return {
      path: entryPath,
      bytes: bytes.length,
      sha256: sha256(bytes),
      content_base64: bytes.toString("base64"),
    };
  });
  const bytes = canonicalJsonBytes({ version: 1, entries });
  await writeFile(destination, bytes);
  return sha256(bytes);
}

async function expectPublicationRejection(
  fixture,
  sealedPath,
  digest,
  pattern,
) {
  const publisher = await cloneFresh(fixture.root, "publisher-reject");
  await assert.rejects(
    () =>
      revalidateSealedCandidate({
        repositoryRoot: publisher,
        sealedPath,
        artifactDirectory: path.join(fixture.control, `rejected-${Date.now()}`),
        trustedDirectory: repositoryRoot,
        expectedArtifact: {
          expectedId: "100",
          observedId: "100",
          expectedDigest: digest,
          observedDigest: digest,
        },
        expectedContext: fixture.expectedContext,
      }),
    pattern,
  );
}

test("quality mutations cannot alter the immutable publisher artifact", async () => {
  const fixture = await buildSealedFixture();
  const qualitySealed = path.join(fixture.control, "quality-download.json");
  const publisherSealed = path.join(fixture.control, "publisher-download.json");
  await Promise.all([
    cp(fixture.sealedPath, qualitySealed),
    cp(fixture.sealedPath, publisherSealed),
  ]);
  const qualityRepository = await cloneFresh(fixture.root, "quality");
  const qualityCandidate = path.join(fixture.control, "quality-candidate");
  await revalidateSealedCandidate({
    repositoryRoot: qualityRepository,
    sealedPath: qualitySealed,
    artifactDirectory: qualityCandidate,
    trustedDirectory: repositoryRoot,
    expectedArtifact: {
      expectedId: "100",
      observedId: "100",
      expectedDigest: fixture.sealed.sha256,
      observedDigest: fixture.sealed.sha256,
    },
    expectedContext: fixture.expectedContext,
  });
  for (const relativePath of [
    "attestation.json",
    "builder-output.json",
    "context.json",
    "manifest.json",
    "files/docs/example.md",
  ])
    await writeFile(
      path.join(qualityCandidate, relativePath),
      "mutated locally\n",
    );
  await writeFile(qualitySealed, "mutated local download\n");

  const publisherRepository = await cloneFresh(fixture.root, "publisher");
  await revalidateSealedCandidate({
    repositoryRoot: publisherRepository,
    sealedPath: publisherSealed,
    artifactDirectory: path.join(fixture.control, "publisher-candidate"),
    trustedDirectory: repositoryRoot,
    expectedArtifact: {
      expectedId: "100",
      observedId: "100",
      expectedDigest: fixture.sealed.sha256,
      observedDigest: fixture.sealed.sha256,
    },
    expectedContext: fixture.expectedContext,
  });
  assert.equal(
    await readFile(path.join(publisherRepository, "docs/example.md"), "utf8"),
    "validated bytes\n",
  );
});

test("delete-only candidates seal the four required control files", async () => {
  const fixture = await buildSealedFixture({ deleteOnly: true });
  const sealed = JSON.parse(await readFile(fixture.sealedPath, "utf8"));
  assert.deepEqual(
    sealed.entries.map((entry) => entry.path),
    [
      "attestation.json",
      "builder-output.json",
      "context.json",
      "manifest.json",
    ],
  );
});

test("artifact identity rejects ID and digest mismatches", () => {
  const valid = {
    expectedId: "100",
    observedId: "100",
    expectedDigest: "a".repeat(64),
    observedDigest: "a".repeat(64),
  };
  assert.doesNotThrow(() => verifyArtifactIdentity(valid));
  assert.throws(
    () => verifyArtifactIdentity({ ...valid, observedId: "101" }),
    /artifact ID/u,
  );
  assert.throws(
    () => verifyArtifactIdentity({ ...valid, observedDigest: "b".repeat(64) }),
    /artifact digest/u,
  );
});

test("publisher rejects a checkout that is not the attested base SHA", async () => {
  const fixture = await buildSealedFixture();
  const publisher = await cloneFresh(fixture.root, "publisher-wrong-base");
  await writeFile(path.join(publisher, "later.txt"), "later commit\n");
  git(publisher, ["config", "user.email", "fixture@example.invalid"]);
  git(publisher, ["config", "user.name", "Autonomy Fixture"]);
  git(publisher, ["add", "later.txt"]);
  git(publisher, ["commit", "-qm", "test: move base"]);
  await assert.rejects(
    () =>
      revalidateSealedCandidate({
        repositoryRoot: publisher,
        sealedPath: fixture.sealedPath,
        artifactDirectory: path.join(fixture.control, "wrong-base-candidate"),
        trustedDirectory: repositoryRoot,
        expectedArtifact: {
          expectedId: "100",
          observedId: "100",
          expectedDigest: fixture.sealed.sha256,
          observedDigest: fixture.sealed.sha256,
        },
        expectedContext: fixture.expectedContext,
      }),
    /exact base SHA|applied candidate differs/iu,
  );
});

test("publisher rejects modified control files and candidate bytes", async () => {
  const fixture = await buildSealedFixture();
  const mutations = [
    [
      "context.json",
      (bytes) => {
        const context = JSON.parse(bytes.toString("utf8"));
        context.issue.number = 98;
        return canonicalJsonBytes(context);
      },
    ],
    [
      "manifest.json",
      (bytes) =>
        Buffer.from(
          bytes.toString("utf8").replace(fixture.baseSha, "f".repeat(40)),
        ),
    ],
    [
      "builder-output.json",
      (bytes) =>
        Buffer.from(
          bytes
            .toString("utf8")
            .replace("artifact-boundary", "artifact boundary"),
        ),
    ],
    ["files/docs/example.md", () => Buffer.from("different candidate bytes\n")],
    [
      "attestation.json",
      (bytes) =>
        Buffer.from(
          bytes.toString("utf8").replace('"version": 2', '"version": 1'),
        ),
    ],
  ];
  for (const [index, [entryPath, mutate]] of mutations.entries()) {
    const changed = path.join(fixture.control, `modified-${index}.json`);
    const digest = await mutateSealed(
      fixture.sealedPath,
      changed,
      entryPath,
      mutate,
    );
    await expectPublicationRejection(
      fixture,
      changed,
      digest,
      /context|manifest|output|integrity|attestation|candidate file|base SHA|const/iu,
    );
  }
});

test("sealed extraction rejects missing, duplicate, malformed, and noncanonical context", async () => {
  const fixture = await buildSealedFixture();
  const source = JSON.parse(await readFile(fixture.sealedPath, "utf8"));
  const cases = [
    source.entries.filter((entry) => entry.path !== "context.json"),
    [
      ...source.entries,
      source.entries.find((entry) => entry.path === "context.json"),
    ],
  ];
  for (const [index, entries] of cases.entries()) {
    const bytes = canonicalJsonBytes({ ...source, entries });
    const changed = path.join(fixture.control, `context-shape-${index}.json`);
    await writeFile(changed, bytes);
    await expectPublicationRejection(
      fixture,
      changed,
      sha256(bytes),
      /missing context|duplicate sealed path|too few items/iu,
    );
  }

  for (const [index, contextBytes] of [
    Buffer.from("{not-json}\n"),
    Buffer.from(JSON.stringify({ version: 1 })),
  ].entries()) {
    const changed = path.join(fixture.control, `context-content-${index}.json`);
    const digest = await mutateSealed(
      fixture.sealedPath,
      changed,
      "context.json",
      () => contextBytes,
    );
    await expectPublicationRejection(
      fixture,
      changed,
      digest,
      /JSON|canonical|missing required/iu,
    );
  }
});

test("sealed format cannot preserve a preexisting symlink destination", async () => {
  const fixture = await buildSealedFixture();
  const destination = path.join(fixture.control, "symlink-destination");
  const outside = path.join(fixture.control, "outside-files");
  await mkdir(destination, { recursive: true });
  await mkdir(outside, { recursive: true });
  await writeFile(path.join(outside, "sentinel"), "outside\n");
  await symlink(outside, path.join(destination, "files"));
  await extractSealedCandidate({
    sealedPath: fixture.sealedPath,
    artifactDirectory: destination,
    trustedDirectory: repositoryRoot,
    expectedDigest: fixture.sealed.sha256,
  });
  const metadata = await lstat(path.join(destination, "files/docs/example.md"));
  assert.equal(metadata.isFile(), true);
  assert.equal(metadata.isSymbolicLink(), false);
  assert.equal(
    await readFile(path.join(outside, "sentinel"), "utf8"),
    "outside\n",
  );
});

test("publisher reruns policy checks even when a forged attestation claims validation", async () => {
  const fixture = await buildSealedFixture();
  const cases = [
    {
      name: "protected",
      pattern: /protected paths changed/u,
      modify(state) {
        const filePath = ".github/workflows/evil.yml";
        const bytes = Buffer.from("name: hostile\n");
        state.files.set(`files/${filePath}`, bytes);
        state.manifest.entries.push({
          path: filePath,
          operation: "upsert",
          mode: "100644",
          bytes: bytes.length,
          sha256: sha256(bytes),
        });
        state.output.files_changed.push(filePath);
      },
    },
    {
      name: "sensitive",
      pattern: /sensitive paths require/u,
      modify(state) {
        const filePath = "apps/web/package.json";
        const bytes = Buffer.from('{"private":true}\n');
        state.files.set(`files/${filePath}`, bytes);
        state.manifest.entries.push({
          path: filePath,
          operation: "upsert",
          mode: "100644",
          bytes: bytes.length,
          sha256: sha256(bytes),
        });
        state.output.files_changed.push(filePath);
      },
    },
    {
      name: "secret",
      pattern: /credential or secret/u,
      modify(state) {
        const filePath = "docs/credential.md";
        const bytes = Buffer.from(
          "token=github_pat_abcdefghijklmnopqrstuvwxyz\n",
        );
        state.files.set(`files/${filePath}`, bytes);
        state.manifest.entries.push({
          path: filePath,
          operation: "upsert",
          mode: "100644",
          bytes: bytes.length,
          sha256: sha256(bytes),
        });
        state.output.files_changed.push(filePath);
      },
    },
    {
      name: "traversal",
      pattern: /escapes repository/u,
      modify(state) {
        state.manifest.entries.push({
          path: "../escape",
          operation: "delete",
          mode: "100644",
          bytes: 0,
          sha256: null,
        });
        state.output.files_changed.push("../escape");
      },
    },
    {
      name: "oversized",
      pattern: /too many items/u,
      modify(state) {
        for (let index = 0; index < 100; index += 1) {
          const filePath = `docs/deleted-${index}.md`;
          state.manifest.entries.push({
            path: filePath,
            operation: "delete",
            mode: "100644",
            bytes: 0,
            sha256: null,
          });
          state.output.files_changed.push(filePath);
        }
      },
    },
  ];
  for (const candidate of cases) {
    const changed = path.join(fixture.control, `forged-${candidate.name}.json`);
    const digest = await forgeSealed(
      fixture.sealedPath,
      changed,
      candidate.modify,
    );
    await expectPublicationRejection(
      fixture,
      changed,
      digest,
      candidate.pattern,
    );
  }
});
