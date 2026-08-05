import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtemp, mkdir, readFile, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  assertIssueClaimable,
  bootstrapPublicationRecorded,
  createBuilderContext,
  createBuilderPublicationPlan,
  isReviewedBootstrap,
  policyIntroducedByRepositoryPush,
  validateSensitivePathAuthorization,
} from "../../../scripts/autonomy/core.mjs";
import {
  applyCandidate,
  captureCandidate,
  validateCandidate,
  verifyAttestation,
} from "../../../scripts/autonomy/candidate.mjs";
import { validateAutonomyRepository } from "./validate-autonomy.mjs";

const repositoryRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../../..",
);
const fixture = JSON.parse(
  await readFile(
    path.join(
      repositoryRoot,
      "apps/web/scripts/fixtures/autonomy/issue-27-bootstrap.json",
    ),
    "utf8",
  ),
);
const policy = JSON.parse(
  await readFile(
    path.join(repositoryRoot, ".github/autonomy/policy.json"),
    "utf8",
  ),
);

function git(root, arguments_) {
  return execFileSync("git", arguments_, { cwd: root, encoding: "utf8" });
}

async function initializeRepository() {
  const root = await mkdtemp(
    path.join(os.tmpdir(), "corpuslab-bootstrap-repository-"),
  );
  git(root, ["init", "-q"]);
  git(root, ["config", "user.email", "fixture@example.invalid"]);
  git(root, ["config", "user.name", "Autonomy Fixture"]);
  await writeFile(path.join(root, "README.md"), "bootstrap fixture\n");
  git(root, ["add", "README.md"]);
  git(root, ["commit", "-qm", "test: create bootstrap base"]);
  const beforeSha = git(root, ["rev-parse", "HEAD"]).trim();
  await mkdir(path.join(root, ".github/autonomy"), { recursive: true });
  await writeFile(
    path.join(root, ".github/autonomy/policy.json"),
    `${JSON.stringify(policy, null, 2)}\n`,
  );
  git(root, ["add", ".github/autonomy/policy.json"]);
  git(root, ["commit", "-qm", "chore: introduce reviewed policy"]);
  return {
    root,
    beforeSha,
    baseSha: git(root, ["rev-parse", "HEAD"]).trim(),
  };
}

function builderOutput(paths) {
  return {
    issue_number: 27,
    plan: [
      "Polish CI key onboarding and failed-gate reports within the reviewed bootstrap capability.",
    ],
    summary:
      "Improved the bounded CI eval workflow with privacy-safe diagnostics and focused coverage.",
    files_changed: paths,
    tests_added: [
      "Added API-key authentication and Settings onboarding coverage.",
    ],
    validation_commands: ["just check", "just ci-check"],
    documentation_updated: [
      "docs/ci-eval-workflows.md",
      "docs/privacy-review-checklist.md",
    ],
    risks: ["CI onboarding copy can drift from endpoint behavior."],
    rollback: "Revert the focused CI eval polish commit without changing data.",
    follow_ups: [],
    atomicity: { justification: "", testing: "", rollback: "" },
    generated_or_mechanical_paths: [],
    test_exception: "",
  };
}

async function writeCandidate(root) {
  for (const [relativePath, content] of Object.entries(
    fixture.candidate_files,
  )) {
    const destination = path.join(root, relativePath);
    await mkdir(path.dirname(destination), { recursive: true });
    await writeFile(destination, content);
  }
}

test("Issue 27 reviewed bootstrap reaches deterministic publication eligibility", async () => {
  const { root, beforeSha, baseSha } = await initializeRepository();
  const control = await mkdtemp(
    path.join(os.tmpdir(), "corpuslab-bootstrap-control-"),
  );
  const bootstrapEvent = {
    eventName: "push",
    ref: "refs/heads/main",
    beforeSha,
    beforePolicyPresent: false,
  };
  assert.equal(
    policyIntroducedByRepositoryPush({
      policy,
      beforeSha,
      eventName: bootstrapEvent.eventName,
      ref: bootstrapEvent.ref,
      repositoryRoot: root,
    }),
    true,
  );
  assert.equal(
    isReviewedBootstrap({
      ...bootstrapEvent,
      authorizationMarker: policy.authorization_marker,
    }),
    true,
  );

  const context = createBuilderContext({
    baseSha,
    trigger: "bootstrap",
    repository: "MuneebHoda/RAG-Debugger",
    runUrl: "https://github.com/MuneebHoda/RAG-Debugger/actions/runs/27",
    authorizedLabels: [],
    issue: fixture.issue,
    policy,
    bootstrapEvent,
  });
  assertIssueClaimable(fixture.issue, context, policy);
  assert.equal(context.issue.body.includes("external"), true);
  assert.deepEqual(
    context.bootstrap_authorization.sensitive_paths,
    policy.bootstrap.authorization.sensitive_paths,
  );

  await writeCandidate(root);
  const paths = Object.keys(fixture.candidate_files).sort();
  const output = builderOutput(paths);
  const outputPath = path.join(control, "builder-output.json");
  const contextPath = path.join(control, "builder-context.json");
  const artifactDirectory = path.join(control, "candidate");
  const attestationPath = path.join(control, "attestation.json");
  await writeFile(outputPath, JSON.stringify(output));
  await writeFile(contextPath, JSON.stringify(context));
  const manifest = await captureCandidate({
    repositoryRoot: root,
    outputPath,
    contextPath,
    schemaPath: path.join(
      repositoryRoot,
      ".github/autonomy/schemas/builder-output.schema.json",
    ),
    policyPath: path.join(repositoryRoot, ".github/autonomy/policy.json"),
    artifactDirectory,
  });
  assert.equal(manifest.base_sha, baseSha);

  git(root, ["reset", "--hard", "-q", baseSha]);
  git(root, ["clean", "-fdq"]);
  assert.equal(git(root, ["rev-parse", "HEAD"]).trim(), baseSha);
  await applyCandidate({ repositoryRoot: root, artifactDirectory });
  const attestation = await validateCandidate({
    repositoryRoot: root,
    artifactDirectory,
    trustedDirectory: repositoryRoot,
    attestationPath,
  });
  git(root, ["diff", "--check"]);
  await validateAutonomyRepository();
  await verifyAttestation(artifactDirectory, attestationPath);
  assert.equal(attestation.base_sha, baseSha);

  const claimedIssue = {
    ...fixture.issue,
    labels: [...fixture.issue.labels, policy.labels.claimed],
  };
  const publication = createBuilderPublicationPlan({
    manifest,
    output,
    context,
    policy,
    issue: claimedIssue,
  });
  assert.equal(
    publication.branch,
    "agent/issue-27-feat-polish-ci-eval-workflow",
  );
  assert.equal(publication.pull_request.draft, true);
  assert.equal(publication.pull_request.base, "main");
  assert.match(publication.pull_request.body, /Refs #27/u);
  assert.equal(bootstrapPublicationRecorded(claimedIssue, policy), false);
  assert.equal(
    bootstrapPublicationRecorded(
      {
        ...claimedIssue,
        labels: [...claimedIssue.labels, policy.labels.generated],
      },
      policy,
    ),
    true,
  );
});

test("bootstrap authorization rejects unreadable commit history", async () => {
  const { root } = await initializeRepository();
  assert.equal(
    policyIntroducedByRepositoryPush({
      policy,
      beforeSha: "f".repeat(40),
      eventName: "push",
      ref: "refs/heads/main",
      repositoryRoot: root,
    }),
    false,
  );
});

test("ordinary issues cannot inherit the Issue 27 sensitive capability", () => {
  const context = createBuilderContext({
    baseSha: "a".repeat(40),
    trigger: "approval_label",
    repository: "MuneebHoda/RAG-Debugger",
    runUrl: "https://github.com/MuneebHoda/RAG-Debugger/actions/runs/28",
    authorizedLabels: [policy.labels.approved],
    issue: {
      number: 28,
      title: "feat: ordinary issue",
      body: "Implement one ordinary reviewed change.",
      labels: [policy.labels.approved],
    },
    policy,
  });
  assert.throws(
    () =>
      validateSensitivePathAuthorization(
        ["apps/api/src/http/api_keys.rs"],
        context,
        policy,
      ),
    /sensitive-approved/u,
  );
});

test("Issue 27 capability rejects sensitive files outside its exact allowlist", () => {
  const context = createBuilderContext({
    baseSha: "a".repeat(40),
    trigger: "bootstrap",
    repository: "MuneebHoda/RAG-Debugger",
    runUrl: "https://github.com/MuneebHoda/RAG-Debugger/actions/runs/27",
    authorizedLabels: [],
    issue: fixture.issue,
    policy,
    bootstrapEvent: {
      eventName: "push",
      ref: "refs/heads/main",
      beforeSha: "b".repeat(40),
      beforePolicyPresent: false,
    },
  });
  assert.throws(
    () =>
      validateSensitivePathAuthorization(
        ["apps/api/src/auth.rs"],
        context,
        policy,
      ),
    /unauthorized sensitive paths/u,
  );
  assert.throws(
    () =>
      validateSensitivePathAuthorization(
        ["migrations/20990101000000_unreviewed.sql"],
        context,
        policy,
      ),
    /unauthorized sensitive paths/u,
  );
});

test("bootstrap authorization is one-time and completion is publication-only", () => {
  const event = {
    eventName: "push",
    ref: "refs/heads/main",
    authorizationMarker: policy.authorization_marker,
  };
  assert.equal(
    isReviewedBootstrap({ ...event, beforePolicyPresent: false }),
    true,
  );
  assert.equal(
    isReviewedBootstrap({ ...event, beforePolicyPresent: true }),
    false,
  );
  assert.equal(
    bootstrapPublicationRecorded(
      {
        labels: [policy.labels.claimed, policy.labels.blocked],
      },
      policy,
    ),
    false,
  );
  assert.equal(
    bootstrapPublicationRecorded(
      {
        labels: [policy.labels.generated],
      },
      policy,
    ),
    true,
  );
});

test("builder workflow contains one model call and no automatic retry", async () => {
  const workflow = await readFile(
    path.join(repositoryRoot, ".github/workflows/autonomy-builder.yml"),
    "utf8",
  );
  assert.equal(workflow.match(/uses: openai\/codex-action@/gu)?.length, 1);
  assert.doesNotMatch(
    workflow,
    /nick-invision\/retry|retry-action|max-attempts|rerun|workflow_run/iu,
  );
  assert.match(workflow, /needs\.publish\.result != 'success'/u);
});
