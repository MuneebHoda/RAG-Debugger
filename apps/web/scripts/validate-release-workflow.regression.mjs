import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { __parsePrettierYamlConfig as parseYaml } from "prettier/plugins/yaml";

import { validateReleaseWorkflow } from "./validate-release-workflow.mjs";

const repositoryRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../../..",
);
const workflowPath = path.join(
  repositoryRoot,
  ".github/workflows/publish-artifacts.yml",
);
const verifierPath = path.join(
  repositoryRoot,
  "scripts/verify-published-release.sh",
);

async function workflow(sourceTransform = (source) => source) {
  const source = sourceTransform(await readFile(workflowPath, "utf8"));
  return {
    source,
    parsed: await parseYaml(source),
    verifier: await readFile(verifierPath, "utf8"),
  };
}

test("accepts the least-privilege trusted publication workflow", async () => {
  const { source, parsed, verifier } = await workflow();
  assert.doesNotThrow(() => validateReleaseWorkflow(source, parsed, verifier));
});

test("rejects pull-request and fork-capable publication triggers", async () => {
  const { source, parsed } = await workflow((value) =>
    value.replace("  release:\n", "  pull_request:\n  release:\n"),
  );
  assert.throws(
    () => validateReleaseWorkflow(source, parsed, "--source-digest"),
    /may trigger only/,
  );

  const forkCapable = await workflow((value) =>
    value.replace(
      "github.repository == 'MuneebHoda/RAG-Debugger'",
      "github.repository != ''",
    ),
  );
  assert.throws(
    () =>
      validateReleaseWorkflow(
        forkCapable.source,
        forkCapable.parsed,
        forkCapable.verifier,
      ),
    /reject fork/,
  );

  const lightweightTag = await workflow((value) =>
    value.replace('          [[ "$object_type" = tag ]]\n', ""),
  );
  assert.throws(
    () =>
      validateReleaseWorkflow(
        lightweightTag.source,
        lightweightTag.parsed,
        lightweightTag.verifier,
      ),
    /annotated release tag/,
  );
});

test("rejects broad package permissions and mutable selectors", async () => {
  const broad = await workflow((value) =>
    value.replace(
      "      checks: read\n      contents: read\n      pull-requests: read\n      security-events: read",
      "      packages: write\n      checks: read\n      contents: read\n      pull-requests: read",
    ),
  );
  assert.throws(
    () => validateReleaseWorkflow(broad.source, broad.parsed, broad.verifier),
    /gate-main permissions/,
  );

  const mutable = await workflow((value) =>
    value.replace(
      "CORPUSLAB_API_IMAGE: ghcr.io/muneebhoda/rag-debugger",
      "CORPUSLAB_API_IMAGE: ghcr.io/muneebhoda/rag-debugger:latest",
    ),
  );
  assert.throws(
    () =>
      validateReleaseWorkflow(mutable.source, mutable.parsed, mutable.verifier),
    /latest/,
  );

  const runtimeSecret = await workflow((value) =>
    value.replace(
      "      CORPUSLAB_API_IMAGE: ghcr.io/muneebhoda/rag-debugger\n",
      "      CORPUSLAB_API_IMAGE: ghcr.io/muneebhoda/rag-debugger\n      PRODUCTION_TOKEN: ${{ secrets.PRODUCTION_TOKEN }}\n",
    ),
  );
  assert.throws(
    () =>
      validateReleaseWorkflow(
        runtimeSecret.source,
        runtimeSecret.parsed,
        runtimeSecret.verifier,
      ),
    /runtime\/provider secrets/,
  );
});

test("rejects checkout or missing identity binding in the mutation job", async () => {
  const checkout = await workflow((value) =>
    value.replace(
      "    steps:\n      - name: Download and bind the independently verified release bundle",
      "    steps:\n      - uses: actions/checkout@d23441a48e516b6c34aea4fa41551a30e30af803\n      - name: Download and bind the independently verified release bundle",
    ),
  );
  assert.throws(
    () =>
      validateReleaseWorkflow(
        checkout.source,
        checkout.parsed,
        checkout.verifier,
      ),
    /must not checkout/,
  );

  const unbound = await workflow((value) =>
    value.replace(".head_sha == $source_sha", ".head_sha != null"),
  );
  assert.throws(
    () =>
      validateReleaseWorkflow(unbound.source, unbound.parsed, unbound.verifier),
    /bind the resolved source/,
  );
});

test("rejects unpinned actions and unbounded artifacts", async () => {
  const unpinned = await workflow((value) =>
    value.replace(/actions\/checkout@[a-f0-9]{40}/, "actions/checkout@v6"),
  );
  assert.throws(
    () =>
      validateReleaseWorkflow(
        unpinned.source,
        unpinned.parsed,
        unpinned.verifier,
      ),
    /pinned/,
  );

  const unbounded = await workflow((value) =>
    value.replace("retention-days: 30", "retention-days: 0"),
  );
  assert.throws(
    () =>
      validateReleaseWorkflow(
        unbounded.source,
        unbounded.parsed,
        unbounded.verifier,
      ),
    /bounded retention/,
  );
});
