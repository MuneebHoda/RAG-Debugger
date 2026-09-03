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
export const DEPLOYMENT_CONTRACT_SECTIONS = [
  "Approved Topology",
  "Environment Contract",
  "Runtime Configuration Inventory",
  "Provider And Data Processing Inventory",
  "Build And Release Contract",
  "Database And Migrations Contract",
  "Operational Baseline",
  "Cost And Growth Boundary",
  "Follow-Up Issue Map",
];
export const DEPLOYMENT_ACCESS_MARKERS = [
  "multi-domain Access application",
  "Eager redirect cookie",
  "### #107 Access And CORS Qualification",
];
const DEPLOYMENT_DOCUMENT_LINKS = new Map([
  ["docs/architecture.md", "deployment-architecture.md"],
  ["docs/privacy-security.md", "deployment-architecture.md"],
  ["docs/logging-redaction.md", "deployment-architecture.md"],
  ["docs/development.md", "deployment-architecture.md"],
  ["docs/releasing.md", "deployment-architecture.md"],
  ["docs/technical-handbook.md", "deployment-architecture.md"],
]);

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

function parseFence(line) {
  const match = /^ {0,3}(`{3,}|~{3,})(.*)$/.exec(line);
  if (!match) {
    return undefined;
  }

  return {
    character: match[1][0],
    length: match[1].length,
    trailing: match[2],
  };
}

function removeFencedCode(markdown) {
  let activeFence;

  return markdown
    .split(/\r?\n/)
    .map((line) => {
      const fence = parseFence(line);
      if (activeFence) {
        if (
          fence &&
          fence.character === activeFence.character &&
          fence.length >= activeFence.length &&
          fence.trailing.trim().length === 0
        ) {
          activeFence = undefined;
        }
        return "";
      }

      if (fence) {
        activeFence = fence;
        return "";
      }

      return line;
    })
    .join("\n");
}

function backtickRunLength(markdown, start) {
  let end = start;
  while (markdown[end] === "`") {
    end += 1;
  }
  return end - start;
}

function removeInlineCode(markdown) {
  let rendered = "";
  let index = 0;

  while (index < markdown.length) {
    if (markdown[index] !== "`") {
      rendered += markdown[index];
      index += 1;
      continue;
    }

    const openingLength = backtickRunLength(markdown, index);
    let searchIndex = index + openingLength;
    let closingIndex = -1;

    while (searchIndex < markdown.length) {
      const candidateIndex = markdown.indexOf("`", searchIndex);
      if (candidateIndex === -1) {
        break;
      }

      const candidateLength = backtickRunLength(markdown, candidateIndex);
      if (candidateLength === openingLength) {
        closingIndex = candidateIndex;
        break;
      }
      searchIndex = candidateIndex + candidateLength;
    }

    if (closingIndex === -1) {
      rendered += markdown.slice(index, index + openingLength);
      index += openingLength;
      continue;
    }

    rendered += " ";
    index = closingIndex + openingLength;
  }

  return rendered;
}

export function markdownOutsideCode(markdown) {
  return removeInlineCode(removeFencedCode(markdown));
}

export function extractMarkdownLinkDestinations(markdown) {
  const destinations = [];
  const linkPattern =
    /(?<!!)\[[^\]\r\n]+\]\(\s*(?:<([^>\r\n]+)>|([^\s)\r\n]+))(?:\s+(?:"[^"\r\n]*"|'[^'\r\n]*'|\([^\r\n)]*\)))?\s*\)/g;

  for (const match of markdownOutsideCode(markdown).matchAll(linkPattern)) {
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
  const lines = markdownOutsideCode(markdown).split("\n");
  const marker = `## ${heading}`;
  const headingIndex = lines.findIndex((line) => line.trim() === marker);
  if (headingIndex === -1) {
    return undefined;
  }

  const nextHeadingOffset = lines
    .slice(headingIndex + 1)
    .findIndex((line) => /^ {0,3}##(?:\s|$)/.test(line));
  const sectionEnd =
    nextHeadingOffset === -1
      ? lines.length
      : headingIndex + 1 + nextHeadingOffset;

  return lines.slice(headingIndex + 1, sectionEnd).join("\n");
}

export function validateDeploymentContract(relativePath, markdown) {
  for (const heading of DEPLOYMENT_CONTRACT_SECTIONS) {
    assert(
      extractLevelTwoSection(markdown, heading) !== undefined,
      `${relativePath} must define the ${heading} section`,
    );
  }

  const destinations = extractMarkdownLinkDestinations(markdown);
  assert(
    destinations.includes("adr/0010-private-alpha-deployment.md"),
    `${relativePath} must link to ADR 0010`,
  );
  for (const marker of DEPLOYMENT_ACCESS_MARKERS) {
    assert(
      markdown.includes(marker),
      `${relativePath} must retain the deployment requirement: ${marker}`,
    );
  }
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

function validateIssueForm(relativePath, form, configuredLabels) {
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
      markdown.push(markdownOutsideCode(item.attributes.value));
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
    validateIssueForm(relativePath, document.value, configuredLabels);
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
  const renderedSecurityPolicy = markdownOutsideCode(securityPolicy);
  assert(
    /do not open a public GitHub issue/i.test(renderedSecurityPolicy),
    `${securityPath} must prohibit public vulnerability reports`,
  );
  const reportingSection = extractLevelTwoSection(
    renderedSecurityPolicy,
    "Report A Vulnerability",
  );
  assert(
    reportingSection,
    `${securityPath} must define the vulnerability-reporting section`,
  );
  validateCanonicalAdvisoryLinks(securityPath, reportingSection);
  const normalizedPolicy = renderedSecurityPolicy.toLowerCase();
  for (const marker of SENSITIVE_DATA_MARKERS) {
    assert(
      normalizedPolicy.includes(marker),
      `${securityPath} private-reporting boundary must mention ${marker}`,
    );
  }

  const deploymentPath = "docs/deployment-architecture.md";
  validateDeploymentContract(
    deploymentPath,
    await readRepositoryFile(deploymentPath),
  );
  const deploymentAdrPath = "docs/adr/0010-private-alpha-deployment.md";
  const deploymentAdr = await readRepositoryFile(deploymentAdrPath);
  assert(
    extractMarkdownLinkDestinations(deploymentAdr).includes(
      "../deployment-architecture.md#follow-up-issue-map",
    ),
    `${deploymentAdrPath} must link to the deployment follow-up map`,
  );
  for (const [relativePath, destination] of DEPLOYMENT_DOCUMENT_LINKS) {
    const markdown = await readRepositoryFile(relativePath);
    assert(
      extractMarkdownLinkDestinations(markdown).includes(destination),
      `${relativePath} must link to ${destination}`,
    );
  }

  console.log(
    `Governance validation passed for ${templateNames.length} public issue forms and the deployment contract.`,
  );
}

const invokedPath = process.argv[1] && path.resolve(process.argv[1]);
if (invokedPath === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(`Governance validation failed: ${error.message}`);
    process.exitCode = 1;
  });
}
