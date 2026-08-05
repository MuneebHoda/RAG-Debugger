import { readFile, readdir } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

import console from "node:console";

import { __parsePrettierYamlConfig as parseYaml } from "prettier/plugins/yaml";

export const PRIVATE_ADVISORY_URL =
  "https://github.com/MuneebHoda/RAG-Debugger/security/advisories/new";
const REQUIRED_FORMS = new Set([
  "bug_report.yml",
  "feature_request.yml",
  "security_privacy.yml",
]);
const ISSUE_BODY_TYPES = new Set([
  "checkboxes",
  "dropdown",
  "input",
  "markdown",
  "textarea",
]);
const SENSITIVE_DATA_MARKERS = [
  "secret",
  "credentials",
  "customer corpus",
  "queries",
  "traces",
  "sensitive data",
];

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDirectory, "../../..");
const issueTemplateDirectory = path.join(
  repositoryRoot,
  ".github/ISSUE_TEMPLATE",
);

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

export function extractMarkdownLinkDestinations(markdown) {
  const destinations = [];
  const linkPattern =
    /(?<!!)\[[^\]\r\n]+\]\(\s*(?:<([^>\r\n]+)>|([^\s)\r\n]+))(?:\s+(?:"[^"\r\n]*"|'[^'\r\n]*'|\([^\r\n)]*\)))?\s*\)/g;

  for (const match of markdown.matchAll(linkPattern)) {
    destinations.push(match[1] ?? match[2]);
  }

  return destinations;
}

export function validateCanonicalAdvisoryLinks(relativePath, markdown) {
  const destinations = extractMarkdownLinkDestinations(markdown);
  assert(
    destinations.length > 0,
    `${relativePath} must link to the private advisory URL`,
  );

  for (const destination of destinations) {
    assert(
      destination === PRIVATE_ADVISORY_URL,
      `${relativePath} contains a non-canonical vulnerability-reporting URL`,
    );
  }
}

function extractLevelTwoSection(markdown, heading) {
  const marker = `## ${heading}`;
  const headingStart = markdown.indexOf(marker);
  if (headingStart === -1) {
    return undefined;
  }

  const contentStart = markdown.indexOf("\n", headingStart + marker.length);
  if (contentStart === -1) {
    return "";
  }

  const nextHeading = markdown.indexOf("\n## ", contentStart + 1);
  return markdown.slice(
    contentStart + 1,
    nextHeading === -1 ? markdown.length : nextHeading,
  );
}

async function readRepositoryFile(relativePath) {
  return readFile(path.join(repositoryRoot, relativePath), "utf8");
}

async function readYaml(relativePath) {
  const source = await readRepositoryFile(relativePath);

  try {
    return { source, value: await parseYaml(source) };
  } catch (error) {
    throw new Error(`${relativePath} is not valid YAML: ${error.message}`, {
      cause: error,
    });
  }
}

function validateIssueForm(relativePath, source, form, configuredLabels) {
  assert(
    form && typeof form === "object" && !Array.isArray(form),
    `${relativePath} must contain a YAML object`,
  );
  assert(
    typeof form.name === "string" && form.name.length > 0,
    `${relativePath} must define a name`,
  );
  assert(
    typeof form.description === "string" && form.description.length > 0,
    `${relativePath} must define a description`,
  );
  assert(
    Array.isArray(form.body) && form.body.length > 0,
    `${relativePath} must define a non-empty body`,
  );

  const ids = new Set();
  const markdown = [];

  for (const [index, item] of form.body.entries()) {
    assert(
      item && ISSUE_BODY_TYPES.has(item.type),
      `${relativePath} body item ${index + 1} has an unsupported type`,
    );
    assert(
      item.attributes && typeof item.attributes === "object",
      `${relativePath} body item ${index + 1} must define attributes`,
    );

    if (item.type === "markdown") {
      assert(
        typeof item.attributes.value === "string" &&
          item.attributes.value.length > 0,
        `${relativePath} markdown item ${index + 1} must define content`,
      );
      markdown.push(item.attributes.value);
      continue;
    }

    assert(
      typeof item.id === "string" && /^[A-Za-z0-9_-]+$/.test(item.id),
      `${relativePath} body item ${index + 1} must define a valid id`,
    );
    assert(!ids.has(item.id), `${relativePath} repeats id ${item.id}`);
    ids.add(item.id);
    assert(
      typeof item.attributes.label === "string" &&
        item.attributes.label.length > 0,
      `${relativePath} body item ${item.id} must define a label`,
    );

    if (item.type === "dropdown" || item.type === "checkboxes") {
      assert(
        Array.isArray(item.attributes.options) &&
          item.attributes.options.length > 0,
        `${relativePath} body item ${item.id} must define options`,
      );
    }

    if (Object.hasOwn(item.validations ?? {}, "required")) {
      assert(
        typeof item.validations.required === "boolean",
        `${relativePath} body item ${item.id} has a non-boolean required value`,
      );
    }
  }

  for (const label of form.labels ?? []) {
    assert(
      configuredLabels.has(label),
      `${relativePath} assigns unknown label ${label}`,
    );
  }

  const warning = markdown.find((value) =>
    SENSITIVE_DATA_MARKERS.every((marker) =>
      value.toLowerCase().includes(marker),
    ),
  );
  assert(
    warning,
    `${relativePath} must direct vulnerabilities to the private advisory URL`,
  );
  const normalizedWarning = warning.toLowerCase();
  for (const marker of SENSITIVE_DATA_MARKERS) {
    assert(
      normalizedWarning.includes(marker),
      `${relativePath} warning must mention ${marker}`,
    );
  }

  validateCanonicalAdvisoryLinks(relativePath, warning);
}

async function main() {
  const labelsDocument = await readYaml(".github/labels.yml");
  assert(
    Array.isArray(labelsDocument.value),
    ".github/labels.yml must contain a list",
  );
  const configuredLabels = new Set(
    labelsDocument.value.map((label) => label?.name).filter(Boolean),
  );

  const templateNames = (await readdir(issueTemplateDirectory))
    .filter((name) => name.endsWith(".yml") && name !== "config.yml")
    .sort();
  for (const requiredForm of REQUIRED_FORMS) {
    assert(
      templateNames.includes(requiredForm),
      `Missing required issue form ${requiredForm}`,
    );
  }

  for (const templateName of templateNames) {
    const relativePath = `.github/ISSUE_TEMPLATE/${templateName}`;
    const document = await readYaml(relativePath);
    validateIssueForm(
      relativePath,
      document.source,
      document.value,
      configuredLabels,
    );
  }

  const configPath = ".github/ISSUE_TEMPLATE/config.yml";
  const configDocument = await readYaml(configPath);
  const config = configDocument.value;
  assert(
    config && typeof config === "object" && !Array.isArray(config),
    `${configPath} must contain a YAML object`,
  );
  assert(
    config.blank_issues_enabled === false,
    `${configPath} must keep blank issues disabled`,
  );
  assert(
    Array.isArray(config.contact_links),
    `${configPath} must define contact links`,
  );
  const securityContactLinks = config.contact_links.filter((link) =>
    /vulnerab|security advis|private report/i.test(
      `${link?.name ?? ""} ${link?.about ?? ""}`,
    ),
  );
  assert(
    securityContactLinks.length > 0,
    `${configPath} must expose a private advisory contact link`,
  );
  for (const link of securityContactLinks) {
    assert(
      link.url === PRIVATE_ADVISORY_URL,
      `${configPath} contains a non-canonical vulnerability-reporting URL`,
    );
  }

  const securityPath = "SECURITY.md";
  const securityPolicy = await readRepositoryFile(securityPath);
  assert(
    /do not open a public GitHub issue/i.test(securityPolicy),
    `${securityPath} must prohibit public vulnerability reports`,
  );
  const reportingSection = extractLevelTwoSection(
    securityPolicy,
    "Report A Vulnerability",
  );
  assert(
    reportingSection,
    `${securityPath} must define the vulnerability-reporting section`,
  );
  validateCanonicalAdvisoryLinks(securityPath, reportingSection);
  const normalizedPolicy = securityPolicy.toLowerCase();
  for (const marker of SENSITIVE_DATA_MARKERS) {
    assert(
      normalizedPolicy.includes(marker),
      `${securityPath} private-reporting boundary must mention ${marker}`,
    );
  }

  console.log(
    `Governance validation passed for ${templateNames.length} public issue forms.`,
  );
}

const invokedPath = process.argv[1] && path.resolve(process.argv[1]);
if (invokedPath === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(`Governance validation failed: ${error.message}`);
    process.exitCode = 1;
  });
}
