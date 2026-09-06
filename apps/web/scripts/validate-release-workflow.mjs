import { readFile, readdir } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

import console from "node:console";

import { __parsePrettierYamlConfig as parseYaml } from "prettier/plugins/yaml";

const repositoryRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../../..",
);
const workflowPath = ".github/workflows/publish-artifacts.yml";
const immutableAction =
  /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+(?:\/[A-Za-z0-9_.-]+)?@[a-f0-9]{40}$/;

function invariant(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function permissionsExactly(actual, expected, label) {
  invariant(
    actual && typeof actual === "object" && !Array.isArray(actual),
    `${label} must define explicit permissions`,
  );
  invariant(
    JSON.stringify(actual) === JSON.stringify(expected),
    `${label} permissions exceed the release contract`,
  );
}

function collectUses(value, uses = []) {
  if (Array.isArray(value)) {
    for (const item of value) collectUses(item, uses);
  } else if (value && typeof value === "object") {
    for (const [key, item] of Object.entries(value)) {
      if (key === "uses") uses.push(item);
      collectUses(item, uses);
    }
  }
  return uses;
}

export function validateActionPins(workflow, label) {
  for (const action of collectUses(workflow)) {
    invariant(
      typeof action === "string" &&
        (action.startsWith("./") || immutableAction.test(action)),
      `${label} action ${action} must be pinned to a full commit SHA`,
    );
  }
}

export function validateReleaseWorkflow(source, workflow, verifierSource = "") {
  invariant(
    workflow?.name === "Publish deployment artifacts",
    `${workflowPath} has the wrong workflow name`,
  );
  invariant(
    workflow.on && Object.keys(workflow.on).sort().join(",") === "push,release",
    `${workflowPath} may trigger only from trusted main pushes and published releases`,
  );
  invariant(
    JSON.stringify(workflow.on.push?.branches) === JSON.stringify(["main"]),
    `${workflowPath} may publish main only`,
  );
  invariant(
    JSON.stringify(workflow.on.release?.types) ===
      JSON.stringify(["published"]),
    `${workflowPath} may consume only approved published releases`,
  );
  invariant(
    workflow.permissions && Object.keys(workflow.permissions).length === 0,
    `${workflowPath} must deny workflow-level permissions`,
  );

  const jobs = workflow.jobs ?? {};
  permissionsExactly(
    jobs["gate-main"]?.permissions,
    { checks: "read", contents: "read" },
    "gate-main",
  );
  permissionsExactly(
    jobs["publish-main"]?.permissions,
    {
      "artifact-metadata": "write",
      attestations: "write",
      contents: "read",
      "id-token": "write",
      packages: "write",
    },
    "publish-main",
  );
  permissionsExactly(
    jobs["resolve-release"]?.permissions,
    { contents: "read" },
    "resolve-release",
  );
  permissionsExactly(
    jobs["version-alias"]?.permissions,
    { actions: "read", contents: "write", packages: "write" },
    "version-alias",
  );

  invariant(
    jobs["publish-main"]?.needs === "gate-main",
    "publication must depend on the trusted-main gate",
  );
  invariant(
    jobs["version-alias"]?.needs === "resolve-release",
    "version aliases must depend on release resolution",
  );
  const mainGate = jobs["gate-main"]?.if ?? "";
  const releaseGate = jobs["resolve-release"]?.if ?? "";
  invariant(
    mainGate.includes("github.event_name == 'push'") &&
      mainGate.includes("github.ref == 'refs/heads/main'"),
    "publication must require an exact main push",
  );
  invariant(
    mainGate.includes("github.repository == 'MuneebHoda/RAG-Debugger'") &&
      releaseGate.includes("github.repository == 'MuneebHoda/RAG-Debugger'"),
    "publication must reject fork repositories",
  );
  invariant(
    !/\b(?:pull_request_target|pull_request|issue_comment|issues|workflow_dispatch):/.test(
      source,
    ),
    "untrusted events must not trigger publication",
  );
  invariant(
    !/\b(?:staging|production)_(?:secret|token|password)|secrets\.(?!GITHUB_TOKEN)/i.test(
      source,
    ),
    "runtime/provider secrets must not enter the publication workflow",
  );
  invariant(
    !/:latest(?:[\s"']|$)/i.test(source),
    "latest must not be a publication or deployment selector",
  );
  invariant(
    source.includes("retention-days: 30"),
    "release bundles must have bounded retention",
  );
  invariant(
    source.includes("verify-publish-gates.mjs"),
    "publication must verify required quality/security checks",
  );
  invariant(
    /^\s+\[\[ "\$object_type" = tag \]\]\s*$/m.test(source),
    "version publication must require an annotated release tag",
  );
  invariant(
    source.includes("verify-manifest") &&
      source.includes("verify-published-release.sh"),
    "publication must verify manifests and attestations",
  );
  invariant(
    verifierSource.includes("--source-digest"),
    "attestation verification must bind the source commit",
  );
  invariant(
    source.includes("CORPUSLAB_API_IMAGE: ghcr.io/muneebhoda/rag-debugger"),
    "publication must use the approved GHCR package",
  );

  validateActionPins(workflow, workflowPath);
}

async function main() {
  const source = await readFile(
    path.join(repositoryRoot, workflowPath),
    "utf8",
  );
  const verifierSource = await readFile(
    path.join(repositoryRoot, "scripts/verify-published-release.sh"),
    "utf8",
  );
  validateReleaseWorkflow(source, await parseYaml(source), verifierSource);
  const workflowDirectory = path.join(repositoryRoot, ".github/workflows");
  for (const name of (await readdir(workflowDirectory)).filter((entry) =>
    /\.ya?ml$/.test(entry),
  )) {
    validateActionPins(
      await parseYaml(
        await readFile(path.join(workflowDirectory, name), "utf8"),
      ),
      `.github/workflows/${name}`,
    );
  }
  console.log("Release workflow governance validation passed.");
}

if (path.resolve(process.argv[1] ?? "") === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(
      `Release workflow governance validation failed: ${error.message}`,
    );
    process.exitCode = 1;
  });
}
