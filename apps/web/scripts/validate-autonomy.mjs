import { readFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

import console from "node:console";

import { parse as parseYaml } from "yaml";

import {
  createTrustedBootstrapAuthorization,
  invariant,
  readJson,
  validateJsonSchema,
} from "../../../scripts/autonomy/core.mjs";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDirectory, "../../..");
const workflowPaths = [
  ".github/workflows/autonomy-planner.yml",
  ".github/workflows/autonomy-builder.yml",
];
const immutableAction = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+@[0-9a-f]{40}$/u;

async function readRepositoryFile(relativePath) {
  return readFile(path.join(repositoryRoot, relativePath), "utf8");
}

async function readYaml(relativePath) {
  try {
    return await parseYaml(await readRepositoryFile(relativePath));
  } catch (error) {
    throw new Error(`${relativePath} is not valid YAML: ${error.message}`, {
      cause: error,
    });
  }
}

function walk(value, visit) {
  visit(value);
  if (Array.isArray(value)) value.forEach((item) => walk(item, visit));
  else if (value && typeof value === "object")
    Object.values(value).forEach((item) => walk(item, visit));
}

function collectUses(workflow) {
  const uses = [];
  walk(workflow, (value) => {
    if (value && typeof value === "object" && typeof value.uses === "string")
      uses.push(value.uses);
  });
  return uses;
}

function serialized(value) {
  return JSON.stringify(value);
}

function workflowTriggers(relativePath, workflow) {
  const triggers = workflow?.on;
  invariant(
    triggers && typeof triggers === "object" && !Array.isArray(triggers),
    `${relativePath} must define object-shaped workflow triggers`,
  );
  return triggers;
}

export function validateWorkflow(relativePath, workflow) {
  invariant(
    workflow && typeof workflow === "object",
    `${relativePath} must contain an object`,
  );
  const triggers = workflowTriggers(relativePath, workflow);
  invariant(
    workflow.permissions?.contents === "read",
    `${relativePath} must default to read-only contents`,
  );
  invariant(
    workflow.concurrency?.["cancel-in-progress"] === false,
    `${relativePath} must not cancel an active autonomous run`,
  );
  invariant(
    !triggers.pull_request_target && !triggers.issue_comment,
    `${relativePath} contains an untrusted trigger`,
  );
  for (const action of collectUses(workflow))
    invariant(
      immutableAction.test(action),
      `${relativePath} action is not pinned to a full SHA: ${action}`,
    );
  const text = serialized(workflow);
  if (text.includes("create-github-app-token")) {
    invariant(
      text.includes("github.repository") &&
        !text.includes("github.event.repository.name"),
      `${relativePath} GitHub App tokens must target the current repository`,
    );
  }
  for (const forbidden of [
    "gh pr merge",
    "gh pr ready",
    "enable-auto-merge",
    "issues/close",
    "workflow_run",
  ]) {
    invariant(
      !text.includes(forbidden),
      `${relativePath} contains forbidden automatic operation: ${forbidden}`,
    );
  }
  invariant(
    text.includes("AUTONOMY_PAUSED"),
    `${relativePath} must honor the pause switch`,
  );
  invariant(
    workflow.env?.AUTONOMY_PAUSED?.includes("|| 'true'"),
    `${relativePath} must fail closed when the pause variable is absent`,
  );
  invariant(
    text.includes("HAS_OPENAI_KEY") && text.includes("HAS_AUTONOMY_APP"),
    `${relativePath} preflight must verify model and publisher configuration`,
  );
  if (triggers.workflow_dispatch) {
    invariant(
      text.includes("Diagnostic mode"),
      `${relativePath} dispatch must be diagnostic-only`,
    );
  }
  for (const stage of ["generation", "validation", "publication"])
    invariant(
      text.includes(`assert-unpaused ${stage}`),
      `${relativePath} must recheck pause before ${stage}`,
    );
  invariant(
    collectUses(workflow).filter((action) =>
      action.startsWith("openai/codex-action@"),
    ).length === 1,
    `${relativePath} must contain exactly one model invocation`,
  );
}

function validatePlanner(workflow) {
  const triggers = workflowTriggers(workflowPaths[0], workflow);
  invariant(
    triggers.schedule?.length === 1,
    "planner must define one weekly schedule",
  );
  invariant(workflow.jobs.generate?.steps, "planner generate job is missing");
  const generate = serialized(workflow.jobs.generate);
  invariant(
    generate.includes("gpt-5.6-sol") && generate.includes("xhigh"),
    "planner must use GPT-5.6 Sol with xhigh reasoning",
  );
  invariant(
    generate.includes("workspace-write") && generate.includes("drop-sudo"),
    "planner sandbox policy is incomplete",
  );
  invariant(
    !generate.includes("AUTONOMY_GITHUB_TOKEN") &&
      !generate.includes("create-github-app-token"),
    "planner generation must not receive publication credentials",
  );
  invariant(
    serialized(workflow.jobs.publish).includes('permission-issues":"write'),
    "planner publisher must request issue-only writes",
  );
}

function validateBuilder(workflow) {
  const triggers = workflowTriggers(workflowPaths[1], workflow);
  invariant(
    triggers.issues?.types?.includes("labeled"),
    "builder must listen for label events",
  );
  invariant(
    triggers.push?.branches?.includes("main"),
    "builder must support the reviewed main bootstrap",
  );
  invariant(
    triggers.schedule?.length === 1,
    "builder must define one reconciliation schedule",
  );
  invariant(
    serialized(workflow.jobs.preflight).includes("github.event.before"),
    "builder bootstrap must inspect the exact pre-push commit",
  );
  const generate = serialized(workflow.jobs.generate);
  invariant(
    generate.includes("gpt-5.6-sol") && generate.includes("high"),
    "builder must use GPT-5.6 Sol with high reasoning",
  );
  invariant(
    !generate.includes("xhigh"),
    "builder reasoning must remain high, not xhigh",
  );
  invariant(
    generate.includes("workspace-write") && generate.includes("drop-sudo"),
    "builder sandbox policy is incomplete",
  );
  invariant(
    !generate.includes("AUTONOMY_GITHUB_TOKEN") &&
      !generate.includes("create-github-app-token"),
    "builder generation must not receive publication credentials",
  );
  const publish = serialized(workflow.jobs.publish);
  invariant(
    publish.includes('permission-contents":"write') &&
      publish.includes('permission-issues":"write') &&
      publish.includes('permission-pull-requests":"write'),
    "builder publisher permissions are incomplete",
  );
  invariant(
    serialized(workflow.jobs.validate).includes(
      "scripts/autonomy/run-quality.sh",
    ) && workflow.jobs.publish?.needs === "validate",
    "draft publication must depend on the trusted complete quality gate",
  );
  invariant(
    serialized(workflow.jobs.block).includes(
      "needs.publish.result != 'success'",
    ),
    "builder failures must stop without retry",
  );
  invariant(
    collectUses(workflow).filter((action) =>
      action.startsWith("openai/codex-action@"),
    ).length === 1 &&
      !/(?:nick-invision\/retry|retry-action|max-attempts|workflow_run)/iu.test(
        serialized(workflow),
      ),
    "builder must invoke the model once without automatic retry",
  );
}

async function validatePolicyAndSchemas() {
  const policy = await readJson(
    path.join(repositoryRoot, ".github/autonomy/policy.json"),
  );
  invariant(
    policy.models.planner.id === "gpt-5.6-sol" &&
      policy.models.planner.effort === "xhigh",
    "planner model policy drifted",
  );
  invariant(
    policy.models.builder.id === "gpt-5.6-sol" &&
      policy.models.builder.effort === "high",
    "builder model policy drifted",
  );
  invariant(
    policy.limits.planner_proposals <= 3 && policy.limits.open_proposals <= 3,
    "planner proposal cap exceeds three",
  );
  invariant(
    policy.bootstrap.issue_number === 27 &&
      policy.bootstrap.authorization.id === "issue-27-ci-eval-bootstrap-v1",
    "reviewed bootstrap identity drifted",
  );
  const bootstrap = createTrustedBootstrapAuthorization({
    policy,
    issueNumber: 27,
    baseSha: "a".repeat(40),
    beforeSha: "b".repeat(40),
    eventName: "push",
    ref: "refs/heads/main",
    beforePolicyPresent: false,
  });
  invariant(
    bootstrap.sensitive_paths.length > 0 &&
      bootstrap.sensitive_paths.every(
        (filePath) =>
          !filePath.startsWith(".github/") &&
          !filePath.startsWith("migrations/") &&
          !filePath.startsWith("crates/storage/") &&
          !filePath.endsWith("Cargo.toml") &&
          !filePath.endsWith("package.json") &&
          !filePath.endsWith("package-lock.json") &&
          !filePath.endsWith(".env.example"),
      ),
    "bootstrap authorization exceeds the reviewed CI-eval file boundary",
  );
  for (const required of [
    ".github/",
    "AGENTS.md",
    "SECURITY.md",
    "justfile",
    "scripts/autonomy/",
    "apps/web/scripts/autonomy.bootstrap.regression.mjs",
    "apps/web/scripts/autonomy.security.regression.mjs",
    "apps/web/scripts/fixtures/autonomy/",
  ]) {
    invariant(
      policy.protected_paths.includes(required),
      `protected path is missing from policy: ${required}`,
    );
  }
  const labels = await readYaml(".github/labels.yml");
  const configured = new Set(labels.map((label) => label.name));
  for (const label of Object.values(policy.labels))
    invariant(
      configured.has(label),
      `autonomy label is not configured: ${label}`,
    );

  const plannerSchema = await readJson(
    path.join(
      repositoryRoot,
      ".github/autonomy/schemas/planner-output.schema.json",
    ),
  );
  const builderSchema = await readJson(
    path.join(
      repositoryRoot,
      ".github/autonomy/schemas/builder-output.schema.json",
    ),
  );
  validateJsonSchema(plannerSchema, {
    proposals: [
      {
        title: "test: verify autonomous schema",
        summary:
          "A bounded fixture proving planner output remains structurally valid.",
        evidence: ["scripts/autonomy/core.mjs owns schema validation."],
        scope: ["Validate one deterministic structured-output fixture."],
        non_goals: ["No runtime changes."],
        implementation: ["Run the repository-owned schema validator."],
        architecture:
          "Keep schema validation inside the trusted automation boundary without changing product runtime architecture.",
        acceptance_criteria: [
          "The valid fixture passes deterministic validation.",
        ],
        tests: ["Exercise required and additional-property behavior."],
        security: "No credentials or external content enter this fixture.",
        performance: "The fixture is constant-sized and completes immediately.",
        risks: ["Schema drift could make this fixture stale."],
        rollback: "Revert the schema and matching fixture together.",
        labels: ["type/test", "area/docs"],
      },
    ],
  });
  invariant(
    builderSchema.properties.issue_number.minimum === 1,
    "builder schema lost its issue identity bound",
  );
}

export async function validateAutonomyRepository() {
  const [planner, builder] = await Promise.all(workflowPaths.map(readYaml));
  validateWorkflow(workflowPaths[0], planner);
  validateWorkflow(workflowPaths[1], builder);
  validatePlanner(planner);
  validateBuilder(builder);
  await validatePolicyAndSchemas();
  console.log("Autonomy workflows, policy, labels, and schemas are valid.");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await validateAutonomyRepository();
}
