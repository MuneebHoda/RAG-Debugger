#!/usr/bin/env node

import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

import {
  builderPullRequestBody,
  chooseEligibleIssue,
  containsSecretLikeValue,
  deriveBranchName,
  invariant,
  isReviewedBootstrap,
  normalizeTitle,
  proposalBody,
  readJson,
  sanitizeInventoryIssue,
  sanitizeIssueText,
  validatePlannerOutput,
} from "./core.mjs";
import {
  GitHubClient,
  actorCanAuthorize,
  authorizedLabelEvent,
  authorizedUsers,
  labelNames,
} from "./github.mjs";
import {
  applyCandidate,
  captureCandidate,
  validateCandidate,
  verifyAttestation,
} from "./candidate.mjs";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(
  process.env.AUTONOMY_REPOSITORY_ROOT ??
    path.resolve(scriptDirectory, "../.."),
);
const trustedRoot = path.resolve(
  process.env.AUTONOMY_TRUSTED_ROOT ?? repositoryRoot,
);
const policyPath = path.join(trustedRoot, ".github/autonomy/policy.json");
const plannerSchemaPath = path.join(
  trustedRoot,
  ".github/autonomy/schemas/planner-output.schema.json",
);
const builderSchemaPath = path.join(
  trustedRoot,
  ".github/autonomy/schemas/builder-output.schema.json",
);

function env(name, fallback = "") {
  return process.env[name] ?? fallback;
}

function enabled(value) {
  return String(value).toLocaleLowerCase("en-US") === "true";
}

function paused() {
  return enabled(env("AUTONOMY_PAUSED", "true"));
}

function assertRuntimeConfigured() {
  invariant(enabled(env("HAS_OPENAI_KEY")), "OPENAI_API_KEY is not configured");
  invariant(
    enabled(env("HAS_AUTONOMY_APP")),
    "AUTONOMY_APP_ID or AUTONOMY_APP_PRIVATE_KEY is not configured",
  );
}

function githubClient(tokenName = "GITHUB_TOKEN") {
  return new GitHubClient({
    repository: env("GITHUB_REPOSITORY"),
    token: env(tokenName),
    apiUrl: env("GITHUB_API_URL", "https://api.github.com"),
  });
}

async function writeOutputs(values) {
  const outputPath = env("GITHUB_OUTPUT");
  if (!outputPath) return;
  const lines = Object.entries(values).map(
    ([key, value]) => `${key}=${String(value).replace(/[\r\n]/g, " ")}`,
  );
  await import("node:fs/promises").then(({ appendFile }) =>
    appendFile(outputPath, `${lines.join("\n")}\n`),
  );
}

async function eventPayload() {
  const eventPath = env("GITHUB_EVENT_PATH");
  return eventPath ? readJson(eventPath) : {};
}

function isOpenIssue(issue) {
  return issue?.state === "open" && !issue.pull_request;
}

function authorizationSet() {
  return authorizedUsers(env("AUTONOMY_APPROVERS", "MuneebHoda"));
}

async function controlLabelAuthorizations(
  client,
  issue,
  policy,
  allowlist,
  approvedEvent,
) {
  const names = labelNames(issue);
  const authorized = [];
  for (const label of [
    policy.labels.approved,
    policy.labels.sensitive_approved,
    policy.labels.large_approved,
  ]) {
    if (!names.includes(label)) continue;
    const event =
      label === policy.labels.approved && approvedEvent
        ? approvedEvent
        : await authorizedLabelEvent(client, issue.number, label, allowlist);
    if (event) authorized.push(label);
  }
  return authorized;
}

async function issueContext(client, issue, policy, trigger, approvedEvent) {
  const allowlist = authorizationSet();
  const authorizedLabels =
    trigger === "bootstrap"
      ? []
      : await controlLabelAuthorizations(
          client,
          issue,
          policy,
          allowlist,
          approvedEvent,
        );
  if (trigger !== "bootstrap") {
    invariant(
      authorizedLabels.includes(policy.labels.approved),
      "latest approval label was not applied by an authorized maintainer",
    );
  }
  return {
    version: 1,
    base_sha: env("GITHUB_SHA"),
    trigger,
    repository: env("GITHUB_REPOSITORY"),
    run_url: `${env("GITHUB_SERVER_URL", "https://github.com")}/${env("GITHUB_REPOSITORY")}/actions/runs/${env("GITHUB_RUN_ID")}`,
    authorized_labels: authorizedLabels,
    issue: {
      number: issue.number,
      title: sanitizeIssueText(issue.title, 200),
      body: sanitizeIssueText(
        issue.body ?? "",
        policy.limits.issue_body_characters,
      ),
      labels: labelNames(issue).filter((label) => !label.startsWith("agent/")),
    },
  };
}

async function assertNoGeneratedPullRequest(client, policy) {
  invariant(
    !(await client.hasOpenGeneratedPullRequest(policy.labels.generated)),
    "an agent-generated pull request is already open",
  );
}

function policyIntroducedByPush(policy) {
  const before = env("GITHUB_EVENT_BEFORE");
  if (!before || /^0+$/u.test(before)) return false;
  try {
    execFileSync("git", ["show", `${before}:.github/autonomy/policy.json`], {
      cwd: repositoryRoot,
      stdio: "ignore",
    });
    return isReviewedBootstrap({
      eventName: env("GITHUB_EVENT_NAME"),
      ref: env("GITHUB_REF"),
      beforePolicyPresent: true,
      authorizationMarker: policy.authorization_marker,
    });
  } catch {
    return isReviewedBootstrap({
      eventName: env("GITHUB_EVENT_NAME"),
      ref: env("GITHUB_REF"),
      beforePolicyPresent: false,
      authorizationMarker: policy.authorization_marker,
    });
  }
}

async function plannerPreflight(outputDirectory) {
  const policy = await readJson(policyPath);
  const client = githubClient();
  await mkdir(outputDirectory, { recursive: true });
  const eventName = env("GITHUB_EVENT_NAME");
  const diagnostic = eventName === "workflow_dispatch";
  let reason = "eligible";
  let executeModel = true;
  if (paused()) {
    reason = "autonomy is paused";
    executeModel = false;
  } else if (diagnostic) {
    reason = "diagnostic dry run";
    executeModel = false;
  } else if (
    eventName !== "schedule" ||
    !enabled(env("AUTONOMY_PLANNER_SCHEDULE_ENABLED"))
  ) {
    reason = "planner schedule is disabled";
    executeModel = false;
  }

  const [open, closed] = await Promise.all([
    client.listIssues("open"),
    client.listIssues("closed"),
  ]);
  const openIssues = open.filter(isOpenIssue);
  const proposedCount = openIssues.filter((issue) =>
    labelNames(issue).includes(policy.labels.proposed),
  ).length;
  const availableSlots = Math.max(
    0,
    policy.limits.open_proposals - proposedCount,
  );
  if (availableSlots === 0) {
    reason = "open proposal cap reached";
    executeModel = false;
  }
  if (executeModel) assertRuntimeConfigured();
  const inventory = {
    version: 1,
    repository: env("GITHUB_REPOSITORY"),
    base_sha: env("GITHUB_SHA"),
    available_slots: Math.min(availableSlots, policy.limits.planner_proposals),
    allowed_labels: policy.allowed_issue_labels,
    open_issues: openIssues.map((issue) =>
      sanitizeInventoryIssue(issue, policy),
    ),
    recently_closed: [],
  };
  // Closed issues are not open by definition; retain bounded issue records without PRs.
  inventory.recently_closed = closed
    .filter((issue) => !issue.pull_request)
    .slice(0, 50)
    .map((issue) => sanitizeInventoryIssue(issue, policy));
  await writeFile(
    path.join(outputDirectory, "planner-inventory.json"),
    `${JSON.stringify(inventory, null, 2)}\n`,
  );
  await writeOutputs({
    execute_model: executeModel,
    diagnostic,
    reason,
    available_slots: inventory.available_slots,
  });
  console.log(
    `Planner preflight: ${reason}; available slots=${inventory.available_slots}`,
  );
}

async function builderPreflight(outputDirectory) {
  const policy = await readJson(policyPath);
  const client = githubClient();
  const payload = await eventPayload();
  const eventName = env("GITHUB_EVENT_NAME");
  await mkdir(outputDirectory, { recursive: true });
  if (paused()) {
    await writeOutputs({ execute_model: false, reason: "autonomy is paused" });
    return;
  }
  if (eventName === "workflow_dispatch") {
    await writeOutputs({
      execute_model: false,
      diagnostic: true,
      reason: "diagnostic dry run",
    });
    return;
  }
  if (await client.hasOpenGeneratedPullRequest(policy.labels.generated)) {
    await writeOutputs({
      execute_model: false,
      reason: "an agent-generated pull request is already open",
    });
    return;
  }

  let selected;
  let trigger;
  let approvedEvent;
  const allowlist = authorizationSet();
  if (eventName === "issues") {
    if (
      payload.action !== "labeled" ||
      payload.label?.name !== policy.labels.approved ||
      !isOpenIssue(payload.issue)
    ) {
      await writeOutputs({
        execute_model: false,
        reason: "label event is not an eligible agent/approved issue",
      });
      return;
    }
    invariant(
      await actorCanAuthorize(client, payload.sender?.login, allowlist),
      "label actor is not an authorized maintainer",
    );
    selected = await client.getIssue(payload.issue.number);
    approvedEvent = {
      actor: payload.sender,
      created_at: new Date().toISOString(),
    };
    trigger = "approval_label";
  } else if (eventName === "schedule") {
    if (!enabled(env("AUTONOMY_BUILDER_RECONCILE_ENABLED"))) {
      await writeOutputs({
        execute_model: false,
        reason: "builder reconciliation is disabled",
      });
      return;
    }
    const approvedIssues = (
      await client.listIssues("open", [policy.labels.approved])
    ).filter(isOpenIssue);
    const authorized = [];
    for (const issue of approvedIssues) {
      const event = await authorizedLabelEvent(
        client,
        issue.number,
        policy.labels.approved,
        allowlist,
      );
      if (event)
        authorized.push({
          ...issue,
          approved_at: event.created_at,
          approved_event: event,
        });
    }
    selected = chooseEligibleIssue(authorized, policy);
    if (!selected) {
      await writeOutputs({
        execute_model: false,
        reason: "no eligible approved issue is available",
      });
      return;
    }
    approvedEvent = selected.approved_event;
    trigger = "reconciliation";
  } else if (eventName === "push") {
    if (!policyIntroducedByPush(policy)) {
      await writeOutputs({
        execute_model: false,
        reason: "push is not the one-time reviewed bootstrap",
      });
      return;
    }
    selected = await client.getIssue(policy.bootstrap.issue_number);
    if (!isOpenIssue(selected)) {
      await writeOutputs({
        execute_model: false,
        reason: "bootstrap issue is not open",
      });
      return;
    }
    trigger = "bootstrap";
  } else {
    await writeOutputs({
      execute_model: false,
      reason: `unsupported builder event: ${eventName}`,
    });
    return;
  }

  const names = labelNames(selected);
  invariant(
    ![
      policy.labels.claimed,
      policy.labels.blocked,
      policy.labels.generated,
    ].some((label) => names.includes(label)),
    "issue is already claimed, blocked, or generated",
  );
  assertRuntimeConfigured();
  const context = await issueContext(
    client,
    selected,
    policy,
    trigger,
    approvedEvent,
  );
  await writeFile(
    path.join(outputDirectory, "builder-context.json"),
    `${JSON.stringify(context, null, 2)}\n`,
  );
  await writeOutputs({
    execute_model: true,
    diagnostic: false,
    issue_number: selected.number,
    trigger,
  });
  console.log(`Builder selected issue #${selected.number} through ${trigger}`);
}

const labelDefinitions = {
  proposed: [
    "c5def5",
    "Proposed by the bounded autonomous planner; maintainer approval required.",
  ],
  approved: [
    "0e8a16",
    "Authorized for one bounded autonomous implementation attempt.",
  ],
  claimed: [
    "fbca04",
    "Claimed by the autonomous builder; automatic retries are disabled.",
  ],
  generated: [
    "1d76db",
    "Has an open draft pull request generated by the bounded builder.",
  ],
  blocked: [
    "d73a4a",
    "Autonomous generation, validation, or publication stopped and needs a human.",
  ],
  sensitive_approved: [
    "b60205",
    "Maintainer explicitly approved sensitive-path changes.",
  ],
  large_approved: [
    "5319e7",
    "Maintainer explicitly approved a change above the normal size bound.",
  ],
};

async function upsertAgentLabels(client, policy) {
  for (const [key, [color, description]] of Object.entries(labelDefinitions)) {
    await client.upsertLabel(policy.labels[key], color, description);
  }
}

async function claimIssue(contextPath) {
  invariant(!paused(), "autonomy is paused");
  const [context, policy] = await Promise.all([
    readJson(contextPath),
    readJson(policyPath),
  ]);
  const client = githubClient("AUTONOMY_GITHUB_TOKEN");
  await upsertAgentLabels(client, policy);
  await assertNoGeneratedPullRequest(client, policy);
  const issue = await client.getIssue(context.issue.number);
  invariant(isOpenIssue(issue), "selected issue is no longer open");
  const names = labelNames(issue);
  invariant(
    ![
      policy.labels.claimed,
      policy.labels.blocked,
      policy.labels.generated,
    ].some((label) => names.includes(label)),
    "selected issue is no longer claimable",
  );
  if (context.trigger === "bootstrap") {
    invariant(
      issue.number === policy.bootstrap.issue_number &&
        policy.authorization_marker === "issue-99-reviewed-bootstrap-v1",
      "invalid bootstrap authorization",
    );
  } else {
    invariant(
      names.includes(policy.labels.approved),
      "selected issue is no longer approved",
    );
  }
  await client.addLabels(issue.number, [policy.labels.claimed]);
  await client.addComment(
    issue.number,
    `Bounded autonomous builder claimed this issue for one attempt. Progress: ${context.run_url}`,
  );
}

async function preparePrompt(role, contextPath, outputPath) {
  invariant(
    ["planner", "builder"].includes(role),
    "prompt role must be planner or builder",
  );
  const prompt = await readFile(
    path.join(trustedRoot, `.github/autonomy/prompts/${role}.md`),
    "utf8",
  );
  const context = await readJson(contextPath);
  const serialized = JSON.stringify(context, null, 2);
  invariant(
    !containsSecretLikeValue(serialized),
    "trusted context resembles a secret",
  );
  await writeFile(
    outputPath,
    `${prompt}\n\n## Sanitized trusted context\n\nBEGIN_CONTEXT_JSON\n${serialized}\nEND_CONTEXT_JSON\n`,
  );
}

async function validatePlanner(outputPath, inventoryPath, artifactDirectory) {
  const status = execFileSync(
    "git",
    ["status", "--porcelain", "--untracked-files=all"],
    { cwd: repositoryRoot, encoding: "utf8" },
  );
  invariant(status.trim() === "", "planner modified the repository workspace");
  const [output, inventory, schema, policy] = await Promise.all([
    readJson(outputPath),
    readJson(inventoryPath),
    readJson(plannerSchemaPath),
    readJson(policyPath),
  ]);
  validatePlannerOutput(output, schema, policy, inventory);
  await rm(artifactDirectory, { recursive: true, force: true });
  await mkdir(artifactDirectory, { recursive: true });
  await writeFile(
    path.join(artifactDirectory, "planner-output.json"),
    `${JSON.stringify(output, null, 2)}\n`,
  );
  await cp(
    inventoryPath,
    path.join(artifactDirectory, "planner-inventory.json"),
  );
}

async function publishPlanner(artifactDirectory) {
  invariant(!paused(), "autonomy is paused");
  const [output, inventory, schema, policy] = await Promise.all([
    readJson(path.join(artifactDirectory, "planner-output.json")),
    readJson(path.join(artifactDirectory, "planner-inventory.json")),
    readJson(plannerSchemaPath),
    readJson(policyPath),
  ]);
  validatePlannerOutput(output, schema, policy, inventory);
  const client = githubClient("AUTONOMY_GITHUB_TOKEN");
  const open = (await client.listIssues("open")).filter(isOpenIssue);
  const proposedCount = open.filter((issue) =>
    labelNames(issue).includes(policy.labels.proposed),
  ).length;
  let slots = Math.max(0, policy.limits.open_proposals - proposedCount);
  const existing = new Set(open.map((issue) => normalizeTitle(issue.title)));
  for (const proposal of output.proposals) {
    if (slots === 0) break;
    const normalized = normalizeTitle(proposal.title);
    invariant(
      !existing.has(normalized),
      `proposal became a duplicate before publication: ${proposal.title}`,
    );
    await client.createIssue({
      title: proposal.title,
      body: proposalBody(proposal),
      labels: [policy.labels.proposed, ...proposal.labels],
    });
    existing.add(normalized);
    slots -= 1;
  }
}

async function publishBuilder(artifactDirectory, attestationPath) {
  invariant(!paused(), "autonomy is paused");
  await verifyAttestation(artifactDirectory, attestationPath);
  const [manifest, output, context, policy] = await Promise.all([
    readJson(path.join(artifactDirectory, "manifest.json")),
    readJson(path.join(artifactDirectory, "builder-output.json")),
    readJson(path.join(artifactDirectory, "context.json")),
    readJson(policyPath),
  ]);
  const client = githubClient("AUTONOMY_GITHUB_TOKEN");
  const issue = await client.getIssue(context.issue.number);
  invariant(isOpenIssue(issue), "source issue is no longer open");
  const names = labelNames(issue);
  invariant(
    names.includes(policy.labels.claimed) &&
      !names.includes(policy.labels.blocked),
    "source issue is not in a publishable claim state",
  );
  await assertNoGeneratedPullRequest(client, policy);
  for (const required of context.authorized_labels) {
    invariant(
      names.includes(required),
      `authorization label was removed: ${required}`,
    );
    invariant(
      await authorizedLabelEvent(
        client,
        issue.number,
        required,
        authorizationSet(),
      ),
      `authorization label is no longer attributable to an authorized maintainer: ${required}`,
    );
  }
  const main = await client.getRef("main");
  invariant(
    main.object?.sha === manifest.base_sha,
    "main moved after generation; publication requires a fresh attempt",
  );

  const baseCommit = await client.getGitCommit(manifest.base_sha);
  const treeEntries = [];
  for (const entry of manifest.entries) {
    if (entry.operation === "delete") {
      treeEntries.push({
        path: entry.path,
        mode: "100644",
        type: "blob",
        sha: null,
      });
      continue;
    }
    const bytes = await readFile(
      path.join(artifactDirectory, "files", entry.path),
    );
    invariant(
      createHash("sha256").update(bytes).digest("hex") === entry.sha256,
      `candidate file changed after validation: ${entry.path}`,
    );
    const blob = await client.createBlob(bytes.toString("base64"));
    treeEntries.push({
      path: entry.path,
      mode: entry.mode,
      type: "blob",
      sha: blob.sha,
    });
  }
  const tree = await client.createTree(baseCommit.tree.sha, treeEntries);
  const conventional =
    /^(?:feat|fix|chore|docs|refactor|test|perf|security|ci)(?:\([^)]+\))?:\s\S/u.test(
      context.issue.title,
    );
  const title = conventional
    ? context.issue.title
    : `chore: implement issue #${issue.number}`;
  const commit = await client.createCommit(title, tree.sha, manifest.base_sha);
  const branch = deriveBranchName(issue.number, context.issue.title);
  await client.createRef(branch, commit.sha);
  const pullRequest = await client.createPullRequest({
    title,
    body: builderPullRequestBody(output, context),
    head: branch,
  });
  const transferable = names.filter((label) =>
    policy.allowed_issue_labels.includes(label),
  );
  await client.addLabels(pullRequest.number, [
    policy.labels.generated,
    ...transferable,
  ]);
  await client.addLabels(issue.number, [policy.labels.generated]);
  await client.removeLabel(issue.number, policy.labels.claimed);
  await client.addComment(
    issue.number,
    `Draft implementation opened as #${pullRequest.number}. Human review, CI, approval, and merge are required.`,
  );
}

async function blockIssue(contextPath, reason) {
  if (paused()) return;
  const [context, policy] = await Promise.all([
    readJson(contextPath),
    readJson(policyPath),
  ]);
  const client = githubClient("AUTONOMY_GITHUB_TOKEN");
  const issue = await client.getIssue(context.issue.number);
  if (
    !isOpenIssue(issue) ||
    labelNames(issue).includes(policy.labels.generated)
  )
    return;
  await client.addLabels(issue.number, [policy.labels.blocked]);
  await client.addComment(
    issue.number,
    `Bounded autonomous attempt stopped during ${reason}. It will not retry automatically. Inspect ${context.run_url}, correct the cause, then have a maintainer clear the blocked/claimed state and deliberately re-approve.`,
  );
}

async function validateConfiguration() {
  const policy = await readJson(policyPath);
  invariant(policy.version === 1, "unsupported autonomy policy version");
  invariant(
    policy.models.planner.id === "gpt-5.6-sol" &&
      policy.models.planner.effort === "xhigh",
    "planner must use GPT-5.6 Sol with xhigh reasoning",
  );
  invariant(
    policy.models.builder.id === "gpt-5.6-sol" &&
      policy.models.builder.effort === "high",
    "builder must use GPT-5.6 Sol with high reasoning",
  );
  invariant(
    policy.models.planner.mode === "standard" &&
      policy.models.builder.mode === "standard",
    "autonomy must use standard reasoning mode",
  );
  invariant(
    policy.limits.planner_proposals <= 3 && policy.limits.open_proposals <= 3,
    "proposal bounds cannot exceed three",
  );
  for (const schemaPath of [plannerSchemaPath, builderSchemaPath]) {
    const schema = await readJson(schemaPath);
    invariant(
      schema.$schema === "https://json-schema.org/draft/2020-12/schema",
      `${schemaPath} must declare JSON Schema 2020-12`,
    );
  }
  console.log("Autonomy policy and schemas are valid.");
}

function assertUnpaused(stage) {
  invariant(!paused(), `autonomy is paused before ${stage || "this stage"}`);
  console.log(`Pause check passed before ${stage || "stage"}.`);
}

const [command, ...arguments_] = process.argv.slice(2);
const commands = {
  "assert-unpaused": () => assertUnpaused(arguments_[0]),
  "check-config": () => validateConfiguration(),
  "planner-preflight": () => plannerPreflight(arguments_[0]),
  "builder-preflight": () => builderPreflight(arguments_[0]),
  "claim-issue": () => claimIssue(arguments_[0]),
  "prepare-prompt": () =>
    preparePrompt(arguments_[0], arguments_[1], arguments_[2]),
  "validate-planner": () =>
    validatePlanner(arguments_[0], arguments_[1], arguments_[2]),
  "publish-planner": () => publishPlanner(arguments_[0]),
  "capture-candidate": () =>
    captureCandidate({
      repositoryRoot,
      outputPath: arguments_[0],
      contextPath: arguments_[1],
      schemaPath: builderSchemaPath,
      policyPath,
      artifactDirectory: arguments_[2],
    }),
  "apply-candidate": () =>
    applyCandidate({ repositoryRoot, artifactDirectory: arguments_[0] }),
  "validate-candidate": () =>
    validateCandidate({
      repositoryRoot,
      artifactDirectory: arguments_[0],
      trustedDirectory: arguments_[1],
      attestationPath: arguments_[2],
    }),
  "publish-builder": () => publishBuilder(arguments_[0], arguments_[1]),
  "block-issue": () => blockIssue(arguments_[0], arguments_[1] ?? "validation"),
};

invariant(
  Object.hasOwn(commands, command),
  `unknown autonomy command: ${command ?? "<missing>"}`,
);
await commands[command]();
