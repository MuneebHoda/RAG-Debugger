import { readFile } from "node:fs/promises";
import path from "node:path";

export function invariant(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

export async function readJson(filePath) {
  return JSON.parse(await readFile(filePath, "utf8"));
}

export function uniqueFirst(values) {
  const seen = new Set();
  return values.filter((value) => {
    if (seen.has(value)) return false;
    seen.add(value);
    return true;
  });
}

export function normalizeTitle(value) {
  return value
    .normalize("NFKC")
    .toLocaleLowerCase("en-US")
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

export function branchSlug(value) {
  const slug = normalizeTitle(value).replace(/\s+/g, "-").slice(0, 52);
  return slug || "approved-work";
}

export function deriveBranchName(issueNumber, title) {
  invariant(
    Number.isInteger(issueNumber) && issueNumber > 0,
    "invalid issue number",
  );
  return `agent/issue-${issueNumber}-${branchSlug(title)}`;
}

export function isReviewedBootstrap({
  eventName,
  ref,
  beforePolicyPresent,
  authorizationMarker,
}) {
  return (
    eventName === "push" &&
    ref === "refs/heads/main" &&
    beforePolicyPresent === false &&
    authorizationMarker === "issue-99-reviewed-bootstrap-v1"
  );
}

const commitShaPattern = /^[0-9a-f]{40}$/u;

function bootstrapSensitivePaths(policy) {
  const paths = policy.bootstrap?.authorization?.sensitive_paths;
  invariant(
    Array.isArray(paths) && paths.length > 0,
    "bootstrap sensitive-path authorization is missing",
  );
  const normalized = uniqueFirst(paths.map(normalizeRepositoryPath));
  invariant(
    normalized.length === paths.length &&
      normalized.every((filePath) => !filePath.endsWith("/")),
    "bootstrap authorization must contain unique exact file paths",
  );
  const classification = classifyPaths(normalized, policy);
  invariant(
    classification.sensitive.length === normalized.length,
    "bootstrap authorization may contain only sensitive paths",
  );
  invariant(
    classification.protected.length === 0 &&
      classification.artifacts.length === 0,
    "bootstrap authorization cannot include protected or artifact paths",
  );
  return normalized;
}

export function createTrustedBootstrapAuthorization({
  policy,
  issueNumber,
  baseSha,
  beforeSha,
  eventName,
  ref,
  beforePolicyPresent,
}) {
  invariant(
    issueNumber === policy.bootstrap.issue_number,
    "bootstrap authorization is restricted to the configured issue",
  );
  invariant(
    isReviewedBootstrap({
      eventName,
      ref,
      beforePolicyPresent,
      authorizationMarker: policy.authorization_marker,
    }),
    "bootstrap authorization requires the reviewed policy-introducing push",
  );
  invariant(
    commitShaPattern.test(baseSha) &&
      commitShaPattern.test(beforeSha) &&
      baseSha !== beforeSha,
    "bootstrap authorization requires distinct trusted commit SHAs",
  );
  invariant(
    typeof policy.bootstrap.authorization.id === "string" &&
      policy.bootstrap.authorization.id.length >= 12,
    "bootstrap authorization identifier is invalid",
  );
  return {
    version: 1,
    id: policy.bootstrap.authorization.id,
    issue_number: issueNumber,
    policy_marker: policy.authorization_marker,
    event_name: eventName,
    ref,
    before_sha: beforeSha,
    base_sha: baseSha,
    sensitive_paths: bootstrapSensitivePaths(policy),
  };
}

export function validateTrustedBootstrapAuthorization(context, policy) {
  invariant(
    context.trigger === "bootstrap" &&
      context.issue?.number === policy.bootstrap.issue_number,
    "bootstrap capability is restricted to its configured issue",
  );
  const authorization = context.bootstrap_authorization;
  invariant(
    authorization && typeof authorization === "object",
    "trusted bootstrap authorization is missing",
  );
  const expected = createTrustedBootstrapAuthorization({
    policy,
    issueNumber: context.issue.number,
    baseSha: context.base_sha,
    beforeSha: authorization.before_sha,
    eventName: authorization.event_name,
    ref: authorization.ref,
    beforePolicyPresent: false,
  });
  invariant(
    JSON.stringify(authorization) === JSON.stringify(expected),
    "bootstrap authorization does not match trusted repository policy",
  );
  return expected;
}

export function createBuilderContext({
  baseSha,
  trigger,
  repository,
  runUrl,
  authorizedLabels,
  issue,
  policy,
  bootstrapEvent,
}) {
  const bootstrapAuthorization =
    trigger === "bootstrap"
      ? createTrustedBootstrapAuthorization({
          policy,
          issueNumber: issue.number,
          baseSha,
          ...bootstrapEvent,
        })
      : null;
  invariant(
    trigger === "bootstrap" || bootstrapEvent === undefined,
    "ordinary issues cannot carry bootstrap provenance",
  );
  return {
    version: 1,
    base_sha: baseSha,
    trigger,
    repository,
    run_url: runUrl,
    authorized_labels: uniqueFirst(authorizedLabels),
    bootstrap_authorization: bootstrapAuthorization,
    issue: {
      number: issue.number,
      title: sanitizeIssueText(issue.title, 200),
      body: sanitizeIssueText(
        issue.body ?? "",
        policy.limits.issue_body_characters,
      ),
      labels: (issue.labels ?? [])
        .map((label) => (typeof label === "string" ? label : label.name))
        .filter((label) => !label.startsWith("agent/")),
    },
  };
}

const secretPatterns = [
  /-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----/i,
  /\bsk-(?:proj-)?[A-Za-z0-9_-]{20,}\b/,
  /\bgh[oprsu]_[A-Za-z0-9]{20,}\b/,
  /\bgithub_pat_[A-Za-z0-9_]{20,}\b/,
  /\bAKIA[0-9A-Z]{16}\b/,
  /\b(?:password|passwd|secret|token)\s*[:=]\s*["']?[^\s"']{12,}/i,
];

export function containsSecretLikeValue(value) {
  return secretPatterns.some((pattern) => pattern.test(value));
}

export function sanitizeIssueText(value, maximumLength = 12000) {
  invariant(typeof value === "string", "issue content must be text");
  const sanitized = value
    .normalize("NFKC")
    .replace(/<!--[\s\S]*?-->/g, " ")
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1 [external image removed]")
    .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1 [external link removed]")
    .replace(/<[^>\r\n]+>/g, " ")
    .replace(/https?:\/\/[^\s)\]}]+/gi, "[external link removed]")
    .replace(/[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/g, "")
    .replace(/@(?=[A-Za-z0-9_-])/g, "")
    .trim();

  invariant(
    sanitized.length <= maximumLength,
    "sanitized issue content exceeds policy limit",
  );
  invariant(
    !containsSecretLikeValue(sanitized),
    "issue content resembles a credential or secret",
  );
  return sanitized;
}

export function sanitizeInventoryIssue(issue, policy) {
  let title;
  try {
    title = sanitizeIssueText(issue.title, 200);
  } catch {
    title = "[unsafe issue title redacted]";
  }
  const allowedLabels = new Set([
    ...policy.allowed_issue_labels,
    ...Object.values(policy.labels),
  ]);
  const labels = (issue.labels ?? [])
    .map((label) => (typeof label === "string" ? label : label.name))
    .filter((label) => allowedLabels.has(label));
  return { number: issue.number, title, labels };
}

function typeMatches(expected, value) {
  if (Array.isArray(expected))
    return expected.some((item) => typeMatches(item, value));
  if (expected === "array") return Array.isArray(value);
  if (expected === "object")
    return value !== null && typeof value === "object" && !Array.isArray(value);
  if (expected === "integer") return Number.isInteger(value);
  if (expected === "number")
    return typeof value === "number" && Number.isFinite(value);
  if (expected === "null") return value === null;
  return typeof value === expected;
}

export function validateJsonSchema(schema, value, location = "$") {
  invariant(
    schema && typeof schema === "object",
    `${location}: schema must be an object`,
  );
  if (schema.type !== undefined) {
    invariant(
      typeMatches(schema.type, value),
      `${location}: expected ${JSON.stringify(schema.type)}`,
    );
  }
  if (schema.enum)
    invariant(schema.enum.includes(value), `${location}: value is not allowed`);

  if (typeof value === "string") {
    if (schema.minLength !== undefined)
      invariant(
        value.length >= schema.minLength,
        `${location}: string is too short`,
      );
    if (schema.maxLength !== undefined)
      invariant(
        value.length <= schema.maxLength,
        `${location}: string is too long`,
      );
    if (schema.pattern !== undefined)
      invariant(
        new RegExp(schema.pattern, "u").test(value),
        `${location}: string does not match pattern`,
      );
  }

  if (typeof value === "number") {
    if (schema.minimum !== undefined)
      invariant(value >= schema.minimum, `${location}: value is below minimum`);
    if (schema.maximum !== undefined)
      invariant(value <= schema.maximum, `${location}: value exceeds maximum`);
  }

  if (Array.isArray(value)) {
    if (schema.minItems !== undefined)
      invariant(
        value.length >= schema.minItems,
        `${location}: array has too few items`,
      );
    if (schema.maxItems !== undefined)
      invariant(
        value.length <= schema.maxItems,
        `${location}: array has too many items`,
      );
    if (schema.uniqueItems) {
      const serialized = value.map((item) => JSON.stringify(item));
      invariant(
        new Set(serialized).size === serialized.length,
        `${location}: array items must be unique`,
      );
    }
    if (schema.items)
      value.forEach((item, index) =>
        validateJsonSchema(schema.items, item, `${location}[${index}]`),
      );
  }

  if (value !== null && typeof value === "object" && !Array.isArray(value)) {
    const properties = schema.properties ?? {};
    for (const required of schema.required ?? []) {
      invariant(
        Object.hasOwn(value, required),
        `${location}: missing required property ${required}`,
      );
    }
    if (schema.additionalProperties === false) {
      for (const key of Object.keys(value))
        invariant(
          Object.hasOwn(properties, key),
          `${location}: unexpected property ${key}`,
        );
    }
    for (const [key, propertySchema] of Object.entries(properties)) {
      if (Object.hasOwn(value, key))
        validateJsonSchema(propertySchema, value[key], `${location}.${key}`);
    }
  }

  return value;
}

export function normalizeRepositoryPath(value) {
  invariant(
    typeof value === "string" && value.length > 0,
    "repository path is empty",
  );
  invariant(
    !value.includes("\\") && !/[\u0000-\u001f\u007f]/u.test(value),
    `unsafe repository path: ${value}`,
  );
  invariant(
    !path.posix.isAbsolute(value),
    `absolute repository path is forbidden: ${value}`,
  );
  const normalized = path.posix.normalize(value);
  invariant(
    normalized !== "." && normalized !== ".." && !normalized.startsWith("../"),
    `path escapes repository: ${value}`,
  );
  invariant(
    normalized === value,
    `repository path is not normalized: ${value}`,
  );
  return normalized;
}

export function matchesPolicyPath(filePath, configuredPath) {
  return configuredPath.endsWith("/")
    ? filePath.startsWith(configuredPath)
    : filePath === configuredPath;
}

export function classifyPaths(paths, policy) {
  const normalized = uniqueFirst(paths.map(normalizeRepositoryPath));
  return {
    normalized,
    protected: normalized.filter((filePath) =>
      policy.protected_paths.some((candidate) =>
        matchesPolicyPath(filePath, candidate),
      ),
    ),
    sensitive: normalized.filter((filePath) =>
      policy.sensitive_paths.some((candidate) =>
        matchesPolicyPath(filePath, candidate),
      ),
    ),
    artifacts: normalized.filter((filePath) =>
      policy.artifact_paths.some((candidate) =>
        matchesPolicyPath(filePath, candidate),
      ),
    ),
  };
}

export function validateSensitivePathAuthorization(
  sensitivePaths,
  context,
  policy,
) {
  if (context.trigger === "bootstrap") {
    const authorization = validateTrustedBootstrapAuthorization(
      context,
      policy,
    );
    if (sensitivePaths.length === 0) return;
    const allowed = new Set(authorization.sensitive_paths);
    const unauthorized = sensitivePaths.filter(
      (filePath) => !allowed.has(filePath),
    );
    invariant(
      unauthorized.length === 0,
      `bootstrap candidate contains unauthorized sensitive paths: ${unauthorized.join(", ")}`,
    );
    return;
  }
  if (sensitivePaths.length === 0) {
    invariant(
      context.bootstrap_authorization === null ||
        context.bootstrap_authorization === undefined,
      "ordinary issues cannot carry bootstrap authorization",
    );
    return;
  }
  invariant(
    (context.authorized_labels ?? []).includes(
      policy.labels.sensitive_approved,
    ),
    `sensitive paths require ${policy.labels.sensitive_approved}`,
  );
  invariant(
    context.bootstrap_authorization === null ||
      context.bootstrap_authorization === undefined,
    "ordinary issues cannot carry bootstrap authorization",
  );
}

function issueLabelNames(issue) {
  return (issue.labels ?? []).map((label) =>
    typeof label === "string" ? label : label.name,
  );
}

export function assertIssueClaimable(issue, context, policy) {
  invariant(
    issue?.state === "open" && !issue.pull_request,
    "selected issue is no longer open",
  );
  invariant(
    issue.number === context.issue.number,
    "selected issue does not match trusted context",
  );
  const names = issueLabelNames(issue);
  invariant(
    ![
      policy.labels.claimed,
      policy.labels.blocked,
      policy.labels.generated,
    ].some((label) => names.includes(label)),
    "selected issue is no longer claimable",
  );
  if (context.trigger === "bootstrap") {
    validateTrustedBootstrapAuthorization(context, policy);
  } else {
    invariant(
      names.includes(policy.labels.approved),
      "selected issue is no longer approved",
    );
  }
}

export function bootstrapPublicationRecorded(issue, policy) {
  return issueLabelNames(issue).includes(policy.labels.generated);
}

export function createBuilderPublicationPlan({
  manifest,
  output,
  context,
  policy,
  issue,
}) {
  invariant(
    issue?.state === "open" && issue.number === context.issue.number,
    "source issue is not publishable",
  );
  const names = issueLabelNames(issue);
  invariant(
    names.includes(policy.labels.claimed) &&
      !names.includes(policy.labels.blocked) &&
      !names.includes(policy.labels.generated),
    "source issue is not in a publishable claim state",
  );
  invariant(
    manifest.base_sha === context.base_sha &&
      manifest.issue_number === context.issue.number,
    "publication identity does not match the trusted candidate",
  );
  if (context.trigger === "bootstrap")
    validateTrustedBootstrapAuthorization(context, policy);
  else
    invariant(
      context.bootstrap_authorization === null ||
        context.bootstrap_authorization === undefined,
      "ordinary publication cannot carry bootstrap authorization",
    );

  const conventional =
    /^(?:feat|fix|chore|docs|refactor|test|perf|security|ci)(?:\([^)]+\))?:\s\S/u.test(
      context.issue.title,
    );
  const title = conventional
    ? context.issue.title
    : `chore: implement issue #${issue.number}`;
  return {
    branch: deriveBranchName(issue.number, context.issue.title),
    pull_request: {
      title,
      body: builderPullRequestBody(output, context),
      base: "main",
      draft: true,
    },
    transferable_labels: names.filter((label) =>
      policy.allowed_issue_labels.includes(label),
    ),
  };
}

export function validateChangeSize({
  fileCount,
  meaningfulLines,
  atomicity,
  labels,
  policy,
}) {
  const limits = policy.limits;
  invariant(
    fileCount <= limits.absolute_files,
    `change exceeds absolute ${limits.absolute_files}-file limit`,
  );
  invariant(
    meaningfulLines <= limits.absolute_lines,
    `change exceeds absolute ${limits.absolute_lines}-line limit`,
  );

  const needsJustification =
    fileCount > limits.justification_files ||
    meaningfulLines > limits.justification_lines;
  if (needsJustification) {
    for (const field of ["justification", "testing", "rollback"]) {
      invariant(
        typeof atomicity?.[field] === "string" &&
          atomicity[field].trim().length >= 20,
        `large change requires atomicity.${field}`,
      );
    }
  }

  const needsApproval =
    fileCount > limits.approval_files ||
    meaningfulLines > limits.approval_lines;
  if (needsApproval)
    invariant(
      labels.includes(policy.labels.large_approved),
      `large change requires ${policy.labels.large_approved}`,
    );
}

const priorityRank = new Map([
  ["priority/p0", 0],
  ["priority/p1", 1],
  ["priority/p2", 2],
  ["priority/p3", 3],
]);

function issuePriority(issue) {
  for (const label of issue.labels ?? []) {
    const name = typeof label === "string" ? label : label.name;
    if (priorityRank.has(name)) return priorityRank.get(name);
  }
  return 4;
}

export function chooseEligibleIssue(issues, policy) {
  const excluded = new Set([
    policy.labels.claimed,
    policy.labels.blocked,
    policy.labels.generated,
  ]);
  return [...issues]
    .filter((issue) => issue.state === "open")
    .filter((issue) => {
      const labels = (issue.labels ?? []).map((label) =>
        typeof label === "string" ? label : label.name,
      );
      return (
        labels.includes(policy.labels.approved) &&
        !labels.some((label) => excluded.has(label))
      );
    })
    .sort(
      (left, right) =>
        issuePriority(left) - issuePriority(right) ||
        String(left.approved_at ?? "").localeCompare(
          String(right.approved_at ?? ""),
        ) ||
        left.number - right.number,
    )[0];
}

export function validatePlannerOutput(output, schema, policy, inventory) {
  validateJsonSchema(schema, output);
  invariant(
    output.proposals.length <= inventory.available_slots,
    "planner returned more proposals than available slots",
  );
  const existing = new Set([
    ...inventory.open_issues.map((issue) => normalizeTitle(issue.title)),
    ...inventory.recently_closed.map((issue) => normalizeTitle(issue.title)),
  ]);
  const proposed = new Set();
  for (const proposal of output.proposals) {
    invariant(
      /^(?:feat|fix|chore|docs|refactor|test|perf|security|ci)(?:\([^)]+\))?:\s\S/u.test(
        proposal.title,
      ),
      `proposal title must be conventional: ${proposal.title}`,
    );
    const normalized = normalizeTitle(proposal.title);
    invariant(
      !existing.has(normalized),
      `proposal duplicates an existing issue: ${proposal.title}`,
    );
    invariant(
      !proposed.has(normalized),
      `planner returned duplicate proposals: ${proposal.title}`,
    );
    proposed.add(normalized);
    for (const label of proposal.labels)
      invariant(
        policy.allowed_issue_labels.includes(label),
        `planner used unapproved label: ${label}`,
      );
    const serialized = JSON.stringify(proposal);
    invariant(
      !containsSecretLikeValue(serialized),
      "planner output resembles a credential or secret",
    );
    invariant(
      !/(?:https?:\/\/|<!--|<script\b)/iu.test(serialized),
      "planner output contains external or hidden markup",
    );
  }
  return output;
}

export function validateBuilderOutput(output, schema, context, actualPaths) {
  validateJsonSchema(schema, output);
  invariant(
    output.issue_number === context.issue.number,
    "builder output references the wrong issue",
  );
  const serialized = JSON.stringify(output);
  invariant(
    !containsSecretLikeValue(serialized),
    "builder output resembles a credential or secret",
  );
  invariant(
    !/(?:https?:\/\/|<!--|<script\b|@[A-Za-z0-9_-]+)/iu.test(serialized),
    "builder output contains external or active markup",
  );
  invariant(
    !/\b(?:close[sd]?|fixe[sd]?|resolve[sd]?)\s+#\d+/iu.test(serialized),
    "builder output contains an automatic issue-closing directive",
  );
  const listed = [...output.files_changed].map(normalizeRepositoryPath).sort();
  const actual = [...actualPaths].map(normalizeRepositoryPath).sort();
  invariant(
    JSON.stringify(listed) === JSON.stringify(actual),
    "builder files_changed does not exactly match the candidate diff",
  );
  return output;
}

export function markdownList(values, fallback = "None") {
  return values.length > 0
    ? values
        .map((value) => `- ${String(value).replace(/</g, "&lt;")}`)
        .join("\n")
    : `- ${fallback}`;
}

export function proposalBody(proposal) {
  return [
    "## Summary",
    proposal.summary,
    "## Repository Evidence",
    markdownList(proposal.evidence),
    "## Scope",
    markdownList(proposal.scope),
    "## Non-Goals",
    markdownList(proposal.non_goals),
    "## Implementation",
    markdownList(proposal.implementation),
    "## Architecture Implications",
    proposal.architecture,
    "## Acceptance Criteria",
    markdownList(proposal.acceptance_criteria),
    "## Tests",
    markdownList(proposal.tests),
    "## Security And Privacy",
    proposal.security,
    "## Performance",
    proposal.performance,
    "## Risks",
    markdownList(proposal.risks),
    "## Rollback",
    proposal.rollback,
    "\n_Proposed by CorpusLab's bounded autonomous planner. A maintainer must apply `agent/approved` before implementation._",
  ].join("\n\n");
}

export function builderPullRequestBody(output, context) {
  return [
    `Refs #${context.issue.number}`,
    "## Summary",
    output.summary,
    "## Implementation",
    markdownList(output.plan),
    "## Files Changed",
    markdownList(output.files_changed.map((value) => `\`${value}\``)),
    "## Tests Added Or Updated",
    markdownList(output.tests_added),
    "## Validation Requested",
    markdownList(output.validation_commands.map((value) => `\`${value}\``)),
    "## Documentation",
    markdownList(output.documentation_updated),
    "## Risks",
    markdownList(output.risks),
    "## Rollback",
    output.rollback,
    "## Follow-Ups",
    markdownList(output.follow_ups),
    "\n_This draft was generated by the bounded autonomous builder. Human review, CI, approval, and merge are required._",
  ].join("\n\n");
}
