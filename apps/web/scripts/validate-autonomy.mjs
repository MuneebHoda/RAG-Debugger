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
  validateSchemaDefinition,
  validateJsonSchema,
} from "../../../scripts/autonomy/core.mjs";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDirectory, "../../..");
const workflowPaths = [
  ".github/workflows/autonomy-planner.yml",
  ".github/workflows/autonomy-builder.yml",
];
const immutableAction = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+@[0-9a-f]{40}$/u;
const checkoutAction =
  "actions/checkout@d23441a48e516b6c34aea4fa41551a30e30af803";
const appTokenAction =
  "actions/create-github-app-token@bcd2ba49218906704ab6c1aa796996da409d3eb1";
const uploadArtifactAction =
  "actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a";
const downloadArtifactAction =
  "actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c";
const exactRepository = "MuneebHoda/RAG-Debugger";
const trustedWorkflowPermissions = { contents: "read" };
const trustedPreflightPermissions = {
  contents: "read",
  issues: "read",
  "pull-requests": "read",
};
const trustedPublisherPermissions = { contents: "read" };
const preserveTrustedAutomation =
  'mkdir -p "$RUNNER_TEMP/trusted/.github" "$RUNNER_TEMP/trusted/scripts"\n' +
  'cp -R .github/autonomy "$RUNNER_TEMP/trusted/.github/autonomy"\n' +
  'cp -R scripts/autonomy "$RUNNER_TEMP/trusted/scripts/autonomy"\n';
const trustedCheckout = {
  name: "Checkout exact trusted base",
  uses: checkoutAction,
  with: {
    ref: "${{ github.sha }}",
    "persist-credentials": false,
  },
};

const trustedValidationSteps = [
  trustedCheckout,
  { name: "Preserve trusted automation", run: preserveTrustedAutomation },
  {
    name: "Download candidate",
    uses: downloadArtifactAction,
    with: {
      name: "builder-candidate",
      path: "${{ runner.temp }}/candidate",
    },
  },
  {
    name: "Recheck pause before validation",
    run: 'node "$RUNNER_TEMP/trusted/scripts/autonomy/autonomy.mjs" assert-unpaused validation',
  },
  {
    name: "Apply candidate safely",
    env: {
      AUTONOMY_REPOSITORY_ROOT: "${{ github.workspace }}",
      AUTONOMY_TRUSTED_ROOT: "${{ runner.temp }}/trusted",
    },
    run: 'node "$RUNNER_TEMP/trusted/scripts/autonomy/autonomy.mjs" apply-candidate "$RUNNER_TEMP/candidate"',
  },
  {
    name: "Validate and seal candidate before code execution",
    id: "seal",
    env: {
      AUTONOMY_EXPECTED_BASE_SHA: "${{ github.sha }}",
      AUTONOMY_EXPECTED_CONTEXT_SHA256:
        "${{ needs.preflight.outputs.context_sha256 }}",
      AUTONOMY_EXPECTED_ISSUE_NUMBER:
        "${{ needs.preflight.outputs.issue_number }}",
      AUTONOMY_EXPECTED_REPOSITORY: exactRepository,
      AUTONOMY_EXPECTED_RUN_URL:
        "https://github.com/MuneebHoda/RAG-Debugger/actions/runs/${{ github.run_id }}",
      AUTONOMY_EXPECTED_TRIGGER: "${{ needs.preflight.outputs.trigger }}",
      AUTONOMY_REPOSITORY_ROOT: "${{ github.workspace }}",
      AUTONOMY_TRUSTED_ROOT: "${{ runner.temp }}/trusted",
    },
    run: 'node "$RUNNER_TEMP/trusted/scripts/autonomy/autonomy.mjs" seal-candidate "$RUNNER_TEMP/candidate" "$RUNNER_TEMP/candidate/attestation.json" "$RUNNER_TEMP/builder-sealed.json"',
  },
  {
    name: "Upload immutable validated candidate",
    id: "upload",
    uses: uploadArtifactAction,
    with: {
      path: "${{ runner.temp }}/builder-sealed.json",
      archive: false,
      "retention-days": 1,
      "if-no-files-found": "error",
      overwrite: false,
    },
  },
  {
    name: "Verify immutable upload digest",
    env: {
      LOCAL_DIGEST: "${{ steps.seal.outputs.bundle_sha256 }}",
      UPLOAD_DIGEST: "${{ steps.upload.outputs.artifact-digest }}",
    },
    run: 'test "$LOCAL_DIGEST" = "$UPLOAD_DIGEST"',
  },
];

const trustedPublisherSteps = [
  trustedCheckout,
  { name: "Preserve trusted automation", run: preserveTrustedAutomation },
  {
    name: "Download original immutable candidate by artifact ID",
    uses: downloadArtifactAction,
    with: {
      "artifact-ids": "${{ needs.validate.outputs.artifact_id }}",
      path: "${{ runner.temp }}/sealed",
      "skip-decompress": true,
      "digest-mismatch": "error",
    },
  },
  {
    name: "Recheck pause before publisher validation",
    env: {
      AUTONOMY_TRUSTED_ROOT: "${{ runner.temp }}/trusted",
    },
    run: 'node "$RUNNER_TEMP/trusted/scripts/autonomy/autonomy.mjs" assert-unpaused validation',
  },
  {
    name: "Revalidate and apply candidate without executing it",
    env: {
      AUTONOMY_DOWNLOADED_ARTIFACT_DIGEST:
        "${{ needs.validate.outputs.artifact_digest }}",
      AUTONOMY_DOWNLOADED_ARTIFACT_ID:
        "${{ needs.validate.outputs.artifact_id }}",
      AUTONOMY_EXPECTED_ARTIFACT_DIGEST:
        "${{ needs.validate.outputs.bundle_sha256 }}",
      AUTONOMY_EXPECTED_ARTIFACT_ID:
        "${{ needs.validate.outputs.artifact_id }}",
      AUTONOMY_EXPECTED_BASE_SHA: "${{ github.sha }}",
      AUTONOMY_EXPECTED_CONTEXT_SHA256:
        "${{ needs.preflight.outputs.context_sha256 }}",
      AUTONOMY_EXPECTED_ISSUE_NUMBER:
        "${{ needs.preflight.outputs.issue_number }}",
      AUTONOMY_EXPECTED_REPOSITORY: exactRepository,
      AUTONOMY_EXPECTED_RUN_URL:
        "https://github.com/MuneebHoda/RAG-Debugger/actions/runs/${{ github.run_id }}",
      AUTONOMY_EXPECTED_TRIGGER: "${{ needs.preflight.outputs.trigger }}",
      AUTONOMY_REPOSITORY_ROOT: "${{ github.workspace }}",
      AUTONOMY_TRUSTED_ROOT: "${{ runner.temp }}/trusted",
    },
    run: 'node "$RUNNER_TEMP/trusted/scripts/autonomy/autonomy.mjs" revalidate-sealed-candidate "$RUNNER_TEMP/sealed/builder-sealed.json" "$RUNNER_TEMP/publisher-candidate"',
  },
  {
    name: "Recheck pause before publication",
    env: {
      AUTONOMY_TRUSTED_ROOT: "${{ runner.temp }}/trusted",
    },
    run: 'node "$RUNNER_TEMP/trusted/scripts/autonomy/autonomy.mjs" assert-unpaused publication',
  },
  {
    name: "Create publication-only GitHub App token",
    id: "app-token",
    uses: appTokenAction,
    with: {
      "app-id": "${{ secrets.AUTONOMY_APP_ID }}",
      "private-key": "${{ secrets.AUTONOMY_APP_PRIVATE_KEY }}",
      owner: "MuneebHoda",
      repositories: exactRepository,
      "permission-contents": "write",
      "permission-issues": "write",
      "permission-pull-requests": "write",
    },
  },
  {
    name: "Publish validated draft",
    env: {
      AUTONOMY_GITHUB_TOKEN: "${{ steps.app-token.outputs.token }}",
      AUTONOMY_EXPECTED_BASE_SHA: "${{ github.sha }}",
      AUTONOMY_EXPECTED_CONTEXT_SHA256:
        "${{ needs.preflight.outputs.context_sha256 }}",
      AUTONOMY_EXPECTED_ISSUE_NUMBER:
        "${{ needs.preflight.outputs.issue_number }}",
      AUTONOMY_EXPECTED_REPOSITORY: exactRepository,
      AUTONOMY_EXPECTED_RUN_URL:
        "https://github.com/MuneebHoda/RAG-Debugger/actions/runs/${{ github.run_id }}",
      AUTONOMY_EXPECTED_TRIGGER: "${{ needs.preflight.outputs.trigger }}",
      AUTONOMY_REPOSITORY_ROOT: "${{ github.workspace }}",
      AUTONOMY_TRUSTED_ROOT: "${{ runner.temp }}/trusted",
    },
    run: 'node "$RUNNER_TEMP/trusted/scripts/autonomy/autonomy.mjs" publish-builder "$RUNNER_TEMP/publisher-candidate" "$RUNNER_TEMP/publisher-candidate/attestation.json"',
  },
];

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

function actionSteps(workflow) {
  return Object.entries(workflow.jobs ?? {}).flatMap(([jobName, job]) =>
    (job.steps ?? [])
      .map((step, index) => ({ job, jobName, index, step }))
      .filter(({ step }) => typeof step?.uses === "string"),
  );
}

export function validateAppTokenSteps(relativePath, workflow) {
  for (const { job, jobName, step } of actionSteps(workflow).filter(
    ({ step }) => {
      const actionName = step.uses.split("@", 1)[0].toLocaleLowerCase("en-US");
      return actionName === "actions/create-github-app-token";
    },
  )) {
    invariant(
      step.uses === appTokenAction,
      `${relativePath} ${jobName} uses an unapproved App-token action pin`,
    );
    invariant(
      step.with?.owner === "MuneebHoda" &&
        step.with?.repositories === exactRepository,
      `${relativePath} ${jobName} App token must target exactly ${exactRepository}`,
    );
    invariant(
      step.with?.["app-id"] === "${{ secrets.AUTONOMY_APP_ID }}" &&
        step.with?.["private-key"] ===
          "${{ secrets.AUTONOMY_APP_PRIVATE_KEY }}",
      `${relativePath} ${jobName} App token must use the dedicated repository secrets`,
    );
    invariant(
      step.with?.["skip-token-revoke"] === undefined,
      `${relativePath} ${jobName} App token must preserve automatic token revocation`,
    );
    invariant(
      step.with?.enterprise === undefined &&
        step.with?.["github-api-url"] === undefined &&
        !String(step.with.owner).includes("${{") &&
        !String(step.with.repositories).includes("${{") &&
        !/[\r\n,]/u.test(step.with.repositories),
      `${relativePath} ${jobName} App token target must be one static repository`,
    );
    validateExactPermissions(
      `${relativePath} ${jobName} ordinary GITHUB_TOKEN`,
      job.permissions,
      trustedPublisherPermissions,
    );
  }
}

function canonicalValue(value) {
  if (Array.isArray(value)) return value.map(canonicalValue);
  if (value && typeof value === "object")
    return Object.fromEntries(
      Object.entries(value)
        .sort(([left], [right]) => left.localeCompare(right, "en-US"))
        .map(([key, item]) => [key, canonicalValue(item)]),
    );
  return value;
}

function validateTrustedStepAllowlist(jobName, steps, expected) {
  invariant(
    JSON.stringify(canonicalValue(steps ?? [])) ===
      JSON.stringify(canonicalValue(expected)),
    `${jobName} must match the exact trusted step allowlist`,
  );
}

function validateExactPermissions(scope, actual, expected) {
  invariant(
    JSON.stringify(canonicalValue(actual)) ===
      JSON.stringify(canonicalValue(expected)),
    `${scope} permissions must match the exact least-privilege map`,
  );
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
  validateExactPermissions(
    `${relativePath} workflow`,
    workflow.permissions,
    trustedWorkflowPermissions,
  );
  validateExactPermissions(
    `${relativePath} preflight`,
    workflow.jobs?.preflight?.permissions,
    trustedPreflightPermissions,
  );
  validateExactPermissions(
    `${relativePath} publisher`,
    workflow.jobs?.publish?.permissions,
    trustedPublisherPermissions,
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
  validateAppTokenSteps(relativePath, workflow);
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

export function validateBuilder(workflow) {
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
  const validate = serialized(workflow.jobs.validate);
  const quality = serialized(workflow.jobs.quality);
  const publish = serialized(workflow.jobs.publish);
  invariant(
    publish.includes('permission-contents":"write') &&
      publish.includes('permission-issues":"write') &&
      publish.includes('permission-pull-requests":"write'),
    "builder publisher permissions are incomplete",
  );
  validateTrustedStepAllowlist(
    "builder validation",
    workflow.jobs.validate?.steps,
    trustedValidationSteps,
  );
  validateTrustedStepAllowlist(
    "builder publisher",
    workflow.jobs.publish?.steps,
    trustedPublisherSteps,
  );
  invariant(
    validate.includes("seal-candidate") &&
      validate.includes(uploadArtifactAction) &&
      validate.includes('"archive":false') &&
      validate.includes('"overwrite":false') &&
      !/(?:run-quality|cargo |npm |playwright|sqlx)/u.test(validate),
    "validation must seal and immutably upload before candidate execution",
  );
  invariant(
    quality.includes(downloadArtifactAction) &&
      quality.includes("needs.validate.outputs.artifact_id") &&
      quality.includes("revalidate-sealed-candidate") &&
      quality.includes("scripts/autonomy/run-quality.sh") &&
      quality.includes("sqlx-cli --version 0.8.6") &&
      !quality.includes(uploadArtifactAction) &&
      !quality.includes("create-github-app-token") &&
      !quality.includes("secrets.") &&
      !quality.includes("id-token") &&
      quality.includes('"persist-credentials":false'),
    "quality must be disposable, credential-free, and unable to promote artifacts",
  );
  invariant(
    Array.isArray(workflow.jobs.publish?.needs) &&
      workflow.jobs.publish.needs.includes("validate") &&
      workflow.jobs.publish.needs.includes("quality") &&
      publish.includes("needs.validate.outputs.artifact_id") &&
      publish.includes("revalidate-sealed-candidate") &&
      !publish.includes(uploadArtifactAction) &&
      publish.includes('"persist-credentials":false'),
    "draft publication must use the original artifact after validation and quality",
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

async function validateQualityScript() {
  const script = await readRepositoryFile("scripts/autonomy/run-quality.sh");
  invariant(
    script.includes("pg_isready") &&
      script.includes("seq 1 30") &&
      script.includes("PostgreSQL did not become ready within 60 seconds"),
    "autonomous quality must use a bounded PostgreSQL readiness check",
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
    "apps/web/scripts/autonomy.artifact.regression.mjs",
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

  const schemaDirectory = path.join(repositoryRoot, ".github/autonomy/schemas");
  const schemaNames = [
    "planner-output.schema.json",
    "builder-output.schema.json",
    "builder-context.schema.json",
    "candidate-manifest.schema.json",
    "candidate-attestation.schema.json",
    "sealed-candidate.schema.json",
  ];
  const schemas = await Promise.all(
    schemaNames.map((name) => readJson(path.join(schemaDirectory, name))),
  );
  for (const [index, schema] of schemas.entries())
    validateSchemaDefinition(schema, schemaNames[index]);
  const [plannerSchema, builderSchema] = schemas;
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
  await validateQualityScript();
  console.log("Autonomy workflows, policy, labels, and schemas are valid.");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await validateAutonomyRepository();
}
