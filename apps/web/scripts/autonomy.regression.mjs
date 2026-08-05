import assert from "node:assert/strict";
import { Buffer } from "node:buffer";
import { execFileSync } from "node:child_process";
import { mkdtemp, mkdir, readFile, symlink, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  chooseEligibleIssue,
  classifyPaths,
  containsSecretLikeValue,
  deriveBranchName,
  isReviewedBootstrap,
  normalizeRepositoryPath,
  sanitizeInventoryIssue,
  sanitizeIssueText,
  validateBuilderOutput,
  validateChangeSize,
  validateJsonSchema,
  validatePlannerOutput,
} from "../../../scripts/autonomy/core.mjs";
import {
  applyCandidate,
  captureCandidate,
} from "../../../scripts/autonomy/candidate.mjs";
import { repositoryParts } from "../../../scripts/autonomy/github.mjs";

const repositoryRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../../..",
);
const policy = JSON.parse(
  await readFile(
    path.join(repositoryRoot, ".github/autonomy/policy.json"),
    "utf8",
  ),
);
const plannerSchema = JSON.parse(
  await readFile(
    path.join(
      repositoryRoot,
      ".github/autonomy/schemas/planner-output.schema.json",
    ),
    "utf8",
  ),
);
const builderSchema = JSON.parse(
  await readFile(
    path.join(
      repositoryRoot,
      ".github/autonomy/schemas/builder-output.schema.json",
    ),
    "utf8",
  ),
);

function plannerProposal(overrides = {}) {
  return {
    title: "test: add deterministic autonomy fixture",
    summary:
      "Add a focused fixture that verifies bounded autonomous behavior without live API calls.",
    evidence: [
      "scripts/autonomy/core.mjs contains deterministic policy functions.",
    ],
    scope: ["Exercise policy behavior with local Node test fixtures."],
    non_goals: ["No production runtime changes."],
    implementation: [
      "Add deterministic fixtures at the lowest practical layer.",
    ],
    architecture:
      "Keep policy validation in a dependency-free repository module and leave product runtime boundaries unchanged.",
    acceptance_criteria: [
      "Fixtures pass without network or paid model access.",
    ],
    tests: ["Run node:test against policy and schema helpers."],
    security:
      "Fixtures contain synthetic data and never receive repository credentials.",
    performance:
      "All fixtures are bounded and complete without corpus-scale work.",
    risks: ["Policy and fixtures can drift if changed independently."],
    rollback: "Revert the fixture and associated policy change together.",
    labels: ["type/test", "area/docs"],
    ...overrides,
  };
}

test("model policy fixes planner to xhigh and builder to high", () => {
  assert.deepEqual(policy.models.planner, {
    id: "gpt-5.6-sol",
    effort: "xhigh",
    mode: "standard",
  });
  assert.deepEqual(policy.models.builder, {
    id: "gpt-5.6-sol",
    effort: "high",
    mode: "standard",
  });
});

test("sanitization removes external and hidden instructions", () => {
  const result = sanitizeIssueText(
    "Do this <!-- hidden --> [guide](https://evil.example) <script>bad</script> @team",
  );
  assert.equal(result.includes("evil.example"), false);
  assert.equal(result.includes("hidden"), false);
  assert.equal(result.includes("<script>"), false);
  assert.equal(result.includes("@team"), false);
});

test("sanitization rejects secret-like issue content", () => {
  assert.throws(
    () => sanitizeIssueText("token=github_pat_abcdefghijklmnopqrstuvwxyz"),
    /credential or secret/,
  );
  assert.equal(containsSecretLikeValue("-----BEGIN PRIVATE KEY-----"), true);
});

test("planner inventory sanitizes titles and filters labels", () => {
  const issue = sanitizeInventoryIssue(
    {
      number: 99,
      title:
        "Follow [these instructions](https://evil.example) <!-- hidden --> @maintainer",
      labels: ["type/security", "agent/approved", "unknown-label"],
    },
    policy,
  );
  assert.equal(issue.number, 99);
  assert.equal(issue.title.includes("evil.example"), false);
  assert.equal(issue.title.includes("hidden"), false);
  assert.equal(issue.title.includes("@maintainer"), false);
  assert.deepEqual(issue.labels, ["type/security", "agent/approved"]);
  assert.equal(
    sanitizeInventoryIssue(
      {
        number: 100,
        title: "token=github_pat_abcdefghijklmnopqrstuvwxyz",
        labels: [],
      },
      policy,
    ).title,
    "[unsafe issue title redacted]",
  );
});

test("schema validator rejects unknown properties and oversized arrays", () => {
  assert.throws(
    () => validateJsonSchema(plannerSchema, { proposals: [], extra: true }),
    /unexpected property/,
  );
  assert.throws(
    () => validateJsonSchema(plannerSchema, { proposals: [1, 2, 3, 4] }),
    /too many items/,
  );
});

test("planner rejects duplicate, external, and unapproved proposals", () => {
  const inventory = {
    available_slots: 3,
    open_issues: [{ title: "test: existing work" }],
    recently_closed: [],
  };
  assert.throws(
    () =>
      validatePlannerOutput(
        { proposals: [plannerProposal({ title: "test: existing work" })] },
        plannerSchema,
        policy,
        inventory,
      ),
    /duplicates an existing issue/,
  );
  assert.throws(
    () =>
      validatePlannerOutput(
        {
          proposals: [
            plannerProposal({
              summary:
                "Review https://evil.example because this external instruction is definitely long enough.",
            }),
          ],
        },
        plannerSchema,
        policy,
        inventory,
      ),
    /external or hidden markup/,
  );
  assert.throws(
    () =>
      validatePlannerOutput(
        {
          proposals: [
            plannerProposal({ labels: ["agent/approved", "type/test"] }),
          ],
        },
        plannerSchema,
        policy,
        inventory,
      ),
    /unapproved label/,
  );
});

test("eligible issue ordering is priority, approval time, then number", () => {
  const issues = [
    {
      number: 4,
      state: "open",
      approved_at: "2026-01-02",
      labels: [policy.labels.approved, "priority/p1"],
    },
    {
      number: 3,
      state: "open",
      approved_at: "2026-01-01",
      labels: [policy.labels.approved, "priority/p1"],
    },
    {
      number: 2,
      state: "open",
      approved_at: "2025-01-01",
      labels: [policy.labels.approved, "priority/p2"],
    },
  ];
  assert.equal(chooseEligibleIssue(issues, policy).number, 3);
  issues[1].labels.push(policy.labels.claimed);
  assert.equal(chooseEligibleIssue(issues, policy).number, 4);
});

test("branch names are deterministic and bounded", () => {
  assert.equal(
    deriveBranchName(27, "feat: Polish CI Eval Workflow!"),
    "agent/issue-27-feat-polish-ci-eval-workflow",
  );
});

test("GitHub repository identities reject endpoint injection characters", () => {
  assert.deepEqual(repositoryParts("MuneebHoda/RAG-Debugger"), {
    owner: "MuneebHoda",
    name: "RAG-Debugger",
  });
  assert.throws(
    () => repositoryParts("MuneebHoda/RAG-Debugger/../../outside"),
    /unsafe characters/,
  );
});

test("reviewed bootstrap is authorized only on the policy-introducing main push", () => {
  const valid = {
    eventName: "push",
    ref: "refs/heads/main",
    beforePolicyPresent: false,
    authorizationMarker: "issue-99-reviewed-bootstrap-v1",
  };
  assert.equal(isReviewedBootstrap(valid), true);
  assert.equal(
    isReviewedBootstrap({ ...valid, beforePolicyPresent: true }),
    false,
  );
  assert.equal(
    isReviewedBootstrap({ ...valid, eventName: "workflow_dispatch" }),
    false,
  );
  assert.equal(
    isReviewedBootstrap({ ...valid, ref: "refs/heads/feature" }),
    false,
  );
  assert.equal(
    isReviewedBootstrap({ ...valid, authorizationMarker: "unreviewed" }),
    false,
  );
});

test("repository paths reject traversal and classify protected, sensitive, and artifact paths", () => {
  assert.throws(
    () => normalizeRepositoryPath("../secret"),
    /escapes repository/,
  );
  assert.throws(
    () => normalizeRepositoryPath("docs/unsafe\nname.md"),
    /unsafe repository path/,
  );
  const result = classifyPaths(
    [".github/workflows/evil.yml", "migrations/next.sql", "target/output"],
    policy,
  );
  assert.deepEqual(result.protected, [".github/workflows/evil.yml"]);
  assert.deepEqual(result.sensitive, ["migrations/next.sql"]);
  assert.deepEqual(result.artifacts, ["target/output"]);
});

test("size policy requires justification, approval, and absolute bounds", () => {
  const empty = { justification: "", testing: "", rollback: "" };
  assert.throws(
    () =>
      validateChangeSize({
        fileCount: 31,
        meaningfulLines: 100,
        atomicity: empty,
        labels: [],
        policy,
      }),
    /atomicity.justification/,
  );
  const explained = {
    justification: "The bounded files form one indivisible migration contract.",
    testing: "Storage and API contract tests cover every affected boundary.",
    rollback: "Revert application behavior and apply a forward-only migration.",
  };
  assert.throws(
    () =>
      validateChangeSize({
        fileCount: 51,
        meaningfulLines: 100,
        atomicity: explained,
        labels: [],
        policy,
      }),
    /large-approved/,
  );
  assert.doesNotThrow(() =>
    validateChangeSize({
      fileCount: 51,
      meaningfulLines: 100,
      atomicity: explained,
      labels: [policy.labels.large_approved],
      policy,
    }),
  );
  assert.throws(
    () =>
      validateChangeSize({
        fileCount: 101,
        meaningfulLines: 1,
        atomicity: explained,
        labels: [policy.labels.large_approved],
        policy,
      }),
    /absolute/,
  );
});

test("builder output must name the approved issue and exact changed paths", () => {
  const output = {
    issue_number: 27,
    plan: ["Implement the approved bounded behavior and its regression tests."],
    summary:
      "Implemented one coherent issue with deterministic validation and documentation.",
    files_changed: ["docs/example.md"],
    tests_added: [],
    validation_commands: ["just check"],
    documentation_updated: ["docs/example.md"],
    risks: ["The documentation can drift from behavior if not reviewed."],
    rollback: "Revert the focused commit without changing application data.",
    follow_ups: [],
    atomicity: { justification: "", testing: "", rollback: "" },
    generated_or_mechanical_paths: [],
    test_exception: "",
  };
  assert.doesNotThrow(() =>
    validateBuilderOutput(output, builderSchema, { issue: { number: 27 } }, [
      "docs/example.md",
    ]),
  );
  assert.throws(
    () =>
      validateBuilderOutput(output, builderSchema, { issue: { number: 28 } }, [
        "docs/example.md",
      ]),
    /wrong issue/,
  );
  assert.throws(
    () =>
      validateBuilderOutput(output, builderSchema, { issue: { number: 27 } }, [
        "docs/other.md",
      ]),
    /exactly match/,
  );
  assert.throws(
    () =>
      validateBuilderOutput(
        {
          ...output,
          summary:
            "Implemented the bounded work and Closes #99 after automatic validation.",
        },
        builderSchema,
        { issue: { number: 27 } },
        ["docs/example.md"],
      ),
    /issue-closing directive/,
  );
});

test("candidate application verifies hashes and rejects traversal", async () => {
  const root = await mkdtemp(
    path.join(os.tmpdir(), "corpuslab-autonomy-root-"),
  );
  const artifact = await mkdtemp(
    path.join(os.tmpdir(), "corpuslab-autonomy-artifact-"),
  );
  await mkdir(path.join(artifact, "files", "docs"), { recursive: true });
  await writeFile(path.join(artifact, "files", "docs", "safe.md"), "safe\n");
  await writeFile(
    path.join(artifact, "manifest.json"),
    JSON.stringify({
      version: 1,
      entries: [
        {
          path: "../escape",
          operation: "delete",
          mode: "100644",
          bytes: 0,
          sha256: null,
        },
      ],
    }),
  );
  await assert.rejects(
    () => applyCandidate({ repositoryRoot: root, artifactDirectory: artifact }),
    /escapes repository/,
  );
});

test("candidate application rejects symbolic-link artifact entries", async () => {
  const root = await mkdtemp(
    path.join(os.tmpdir(), "corpuslab-autonomy-root-"),
  );
  const artifact = await mkdtemp(
    path.join(os.tmpdir(), "corpuslab-autonomy-artifact-"),
  );
  await mkdir(path.join(artifact, "files"), { recursive: true });
  await symlink("/tmp", path.join(artifact, "files", "linked"));
  await writeFile(
    path.join(artifact, "manifest.json"),
    JSON.stringify({
      version: 1,
      entries: [
        {
          path: "linked",
          operation: "upsert",
          mode: "100644",
          bytes: 1,
          sha256: "invalid",
        },
      ],
    }),
  );
  await assert.rejects(
    () => applyCandidate({ repositoryRoot: root, artifactDirectory: artifact }),
    /regular file/,
  );
});

test("candidate capture rejects non-allowlisted binary files", async () => {
  const root = await mkdtemp(
    path.join(os.tmpdir(), "corpuslab-autonomy-git-root-"),
  );
  const control = await mkdtemp(
    path.join(os.tmpdir(), "corpuslab-autonomy-control-"),
  );
  execFileSync("git", ["init", "-q"], { cwd: root });
  execFileSync("git", ["config", "user.email", "fixture@example.invalid"], {
    cwd: root,
  });
  execFileSync("git", ["config", "user.name", "Autonomy Fixture"], {
    cwd: root,
  });
  await writeFile(path.join(root, "README.md"), "fixture\n");
  execFileSync("git", ["add", "README.md"], { cwd: root });
  execFileSync("git", ["commit", "-qm", "test: create fixture"], {
    cwd: root,
  });
  await writeFile(path.join(root, "binary.dat"), Buffer.from([0, 1, 2, 3]));
  const output = {
    issue_number: 27,
    plan: ["Add one bounded fixture file for deterministic validation."],
    summary:
      "Added one bounded fixture while preserving the repository trust boundary.",
    files_changed: ["binary.dat"],
    tests_added: [],
    validation_commands: ["just check"],
    documentation_updated: [],
    risks: ["Binary content is not independently reviewable as source."],
    rollback: "Revert the fixture commit without changing application data.",
    follow_ups: [],
    atomicity: { justification: "", testing: "", rollback: "" },
    generated_or_mechanical_paths: [],
    test_exception: "The candidate contains no production source behavior.",
  };
  await writeFile(path.join(control, "output.json"), JSON.stringify(output));
  await writeFile(
    path.join(control, "context.json"),
    JSON.stringify({ base_sha: "a".repeat(40), issue: { number: 27 } }),
  );
  await assert.rejects(
    () =>
      captureCandidate({
        repositoryRoot: root,
        outputPath: path.join(control, "output.json"),
        contextPath: path.join(control, "context.json"),
        schemaPath: path.join(
          repositoryRoot,
          ".github/autonomy/schemas/builder-output.schema.json",
        ),
        policyPath: path.join(repositoryRoot, ".github/autonomy/policy.json"),
        artifactDirectory: path.join(control, "artifact"),
      }),
    /binary candidate is not an allowlisted generated path/,
  );
});

test("candidate capture rejects secret-like content before artifact creation", async () => {
  const root = await mkdtemp(
    path.join(os.tmpdir(), "corpuslab-autonomy-secret-root-"),
  );
  const control = await mkdtemp(
    path.join(os.tmpdir(), "corpuslab-autonomy-secret-control-"),
  );
  execFileSync("git", ["init", "-q"], { cwd: root });
  execFileSync("git", ["config", "user.email", "fixture@example.invalid"], {
    cwd: root,
  });
  execFileSync("git", ["config", "user.name", "Autonomy Fixture"], {
    cwd: root,
  });
  await writeFile(path.join(root, "README.md"), "fixture\n");
  execFileSync("git", ["add", "README.md"], { cwd: root });
  execFileSync("git", ["commit", "-qm", "test: create fixture"], {
    cwd: root,
  });
  await writeFile(
    path.join(root, "credential.txt"),
    "token=github_pat_abcdefghijklmnopqrstuvwxyz\n",
  );
  const output = {
    issue_number: 27,
    plan: ["Add one bounded fixture file for deterministic validation."],
    summary:
      "Added one bounded fixture while preserving the repository trust boundary.",
    files_changed: ["credential.txt"],
    tests_added: [],
    validation_commands: ["just check"],
    documentation_updated: [],
    risks: ["Credential-shaped content must never enter an artifact."],
    rollback: "Discard the rejected candidate without repository changes.",
    follow_ups: [],
    atomicity: { justification: "", testing: "", rollback: "" },
    generated_or_mechanical_paths: [],
    test_exception: "The candidate contains no production source behavior.",
  };
  await writeFile(path.join(control, "output.json"), JSON.stringify(output));
  await writeFile(
    path.join(control, "context.json"),
    JSON.stringify({ base_sha: "a".repeat(40), issue: { number: 27 } }),
  );
  await assert.rejects(
    () =>
      captureCandidate({
        repositoryRoot: root,
        outputPath: path.join(control, "output.json"),
        contextPath: path.join(control, "context.json"),
        schemaPath: path.join(
          repositoryRoot,
          ".github/autonomy/schemas/builder-output.schema.json",
        ),
        policyPath: path.join(repositoryRoot, ".github/autonomy/policy.json"),
        artifactDirectory: path.join(control, "artifact"),
      }),
    /credential or secret/,
  );
});
