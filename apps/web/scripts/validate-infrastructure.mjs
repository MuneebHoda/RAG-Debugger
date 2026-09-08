import console from "node:console";
import { readFile, readdir } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

import { __parsePrettierYamlConfig as parseYaml } from "prettier/plugins/yaml";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDirectory, "../../..");
const workflowDirectory = path.join(repositoryRoot, ".github/workflows");

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function exactArray(actual, expected, label) {
  assert(Array.isArray(actual), `${label} must be a list`);
  assert(
    actual.length === expected.length &&
      actual.every((value, index) => value === expected[index]),
    `${label} must be exactly ${expected.join(", ")}`,
  );
}

function distinct(staging, production, label) {
  assert(staging !== production, `staging and production ${label} must differ`);
}

function assertResourceCeiling(resource, label) {
  assert(
    typeof resource.plan === "string" && resource.plan.length > 0,
    `${label} must define a compute plan`,
  );
  assert(resource.instances === 1, `${label} must run exactly one instance`);
  assert(resource.maxInstances === 1, `${label} must cap instances at one`);
  assert(resource.autoscaling === false, `${label} must disable autoscaling`);
}

function assertNoCredentialValues(value, pathParts = []) {
  if (Array.isArray(value)) {
    value.forEach((item, index) =>
      assertNoCredentialValues(item, [...pathParts, String(index)]),
    );
    return;
  }
  if (!value || typeof value !== "object") {
    return;
  }

  for (const [key, child] of Object.entries(value)) {
    assert(
      !/^(credential|password|secretValue|token|value)$/i.test(key),
      `${[...pathParts, key].join(".")} must name a secret, never store its value`,
    );
    assertNoCredentialValues(child, [...pathParts, key]);
  }
}

function validateEnvironment(name, environment, shared) {
  const isStaging = name === "staging";
  const upperName = name.toUpperCase();
  const hostPrefix = isStaging ? ".staging" : "";
  const expectedWebHost = `app${hostPrefix}.{operator_domain}`;
  const expectedApiHost = `api${hostPrefix}.{operator_domain}`;
  const expectedApiOrigin = `https://${expectedApiHost}`;
  const expectedWebOrigin = `https://${expectedWebHost}`;
  const resourcePrefix = `corpuslab-${name}`;

  assert(
    environment.github.environment === name,
    `${name} must use the matching GitHub Environment`,
  );
  exactArray(
    environment.github.selectedBranches,
    ["main"],
    `${name} GitHub deployment branches`,
  );
  assert(
    environment.github.allowAdminBypass === false,
    `${name} GitHub Environment must prohibit administrative bypass`,
  );
  exactArray(
    environment.github.secrets,
    [
      `CLOUDFLARE_${upperName}_API_TOKEN`,
      `RENDER_${upperName}_API_TOKEN`,
      `DATABASE_${upperName}_MIGRATION_URL`,
      `CLOUDFLARE_${upperName}_ACCESS_CLIENT_ID`,
      `CLOUDFLARE_${upperName}_ACCESS_CLIENT_SECRET`,
    ],
    `${name} GitHub secret inventory`,
  );
  exactArray(
    environment.github.variables,
    [
      "OPERATOR_DOMAIN",
      "RENDER_REGION",
      "CLOUDFLARE_ACCOUNT_ID",
      "CLOUDFLARE_ZONE_ID",
      "CLOUDFLARE_PAGES_PROJECT_ID",
      "CLOUDFLARE_ACCESS_APPLICATION_ID",
      "CLOUDFLARE_TUNNEL_ID",
      "RENDER_PROJECT_ID",
      "RENDER_ENVIRONMENT_ID",
      "RENDER_API_SERVICE_ID",
      "RENDER_CONNECTOR_SERVICE_ID",
      "RENDER_DATABASE_ID",
    ],
    `${name} GitHub variable inventory`,
  );
  if (!isStaging) {
    assert(
      environment.github.requiredReviewers.length > 0,
      "production must require a maintainer reviewer",
    );
    assert(
      environment.github.preventSelfReview === true,
      "production must prevent self-review",
    );
  }

  const { access, pages, tunnel } = environment.cloudflare;
  assert(
    pages.project === `${resourcePrefix}-web`,
    `${name} Pages name drifted`,
  );
  assert(pages.uploadMode === "direct", `${name} Pages must use Direct Upload`);
  assert(pages.gitIntegration === false, `${name} Pages must not build source`);
  assert(pages.hostname === expectedWebHost, `${name} Pages hostname drifted`);
  assert(
    pages.previewDeployments === "disabled",
    `${name} Pages previews must remain disabled`,
  );
  assert(
    pages.pagesDevAction === `redirect to ${expectedWebOrigin}`,
    `${name} pages.dev origin must redirect to the Access-protected hostname`,
  );

  assert(access.applicationCount === 1, `${name} must have one Access app`);
  assert(
    access.application === `${resourcePrefix}-access`,
    `${name} Access application name drifted`,
  );
  assert(
    access.type === "self_hosted",
    `${name} Access app must be self-hosted`,
  );
  exactArray(
    access.domains,
    [expectedWebHost, expectedApiHost],
    `${name} Access domains`,
  );
  assert(
    access.domains.every((hostname) => !hostname.includes("*")),
    `${name} Access domains must not contain wildcards`,
  );
  assert(
    access.eagerRedirectCookie === true,
    `${name} Access app must enable eager redirect cookies`,
  );
  assert(
    access.humanPolicy === "default_deny_operator_allowlist",
    `${name} Access human policy must remain default-deny`,
  );
  assert(
    access.automationPolicy === "environment_specific_service_token",
    `${name} Access automation must use an environment-specific service token`,
  );
  exactArray(
    access.bypassMethods,
    ["OPTIONS"],
    `${name} Access bypass methods`,
  );
  assert(
    access.replacesCorpusLabAuthentication === false,
    `${name} Access must not replace CorpusLab authentication`,
  );

  assert(
    tunnel.tunnel === `${resourcePrefix}-tunnel`,
    `${name} tunnel name drifted`,
  );
  assert(
    tunnel.connectorIdentity === `${resourcePrefix}-cloudflared`,
    `${name} tunnel connector identity drifted`,
  );
  assert(
    tunnel.hostname === expectedApiHost,
    `${name} tunnel hostname drifted`,
  );
  assert(
    tunnel.httpRedirectToHttps === true,
    `${name} must redirect HTTP to HTTPS`,
  );
  assert(
    tunnel.ingress.length === 2 &&
      tunnel.ingress[0].hostname === expectedApiHost &&
      tunnel.ingress[0].service === `http://${resourcePrefix}-api:8080` &&
      tunnel.ingress[1].service === "http_status:404",
    `${name} tunnel must route only the API hostname and deny unmatched ingress`,
  );

  const { api, connector, database, databaseRoles } = environment.render;
  assert(
    environment.render.environment.name === resourcePrefix &&
      environment.render.environment.networkIsolation === "enabled" &&
      environment.render.environment.protection === "enabled",
    `${name} Render environment must be protected and network-isolated`,
  );
  assert(api.service === `${resourcePrefix}-api`, `${name} API name drifted`);
  assert(api.type === "pserv", `${name} API must be a private service`);
  assert(api.runtime === "image", `${name} API must be image-backed`);
  assert(
    api.imageReference === shared.apiImage.reference,
    `${name} API image drifted`,
  );
  assert(
    api.imageReference.includes("@sha256:") && api.sourceBuild === false,
    `${name} API must use the published digest without a source build`,
  );
  assert(
    api.publicProviderOrigin === false,
    `${name} API must not expose a provider public origin`,
  );
  assertResourceCeiling(api, `${name} API`);
  assert(api.plan === "0.5c-512mb", `${name} API compute ceiling drifted`);
  assert(
    api.internalPort === 8080 && api.providerHealth === "tcp:8080",
    `${name} API must expose the bounded private TCP health port`,
  );
  assert(
    api.externalReadinessPath === "/readyz",
    `${name} qualification must use /readyz`,
  );
  assert(
    api.maxShutdownDelaySeconds >= 30,
    `${name} API must retain graceful-shutdown time`,
  );

  assert(
    connector.service === `${resourcePrefix}-cloudflared`,
    `${name} connector name drifted`,
  );
  assert(
    connector.type === "worker" && connector.runtime === "image",
    `${name} connector must be an image-backed worker`,
  );
  assert(
    connector.imageReference === shared.connectorImage.reference &&
      connector.imageReference.includes("@sha256:") &&
      connector.sourceBuild === false,
    `${name} connector must use an immutable image without a source build`,
  );
  assertResourceCeiling(connector, `${name} connector`);
  assert(
    connector.plan === "0.5c-512mb",
    `${name} connector compute ceiling drifted`,
  );
  assert(
    api.regionSource === shared.renderRegionSource &&
      connector.regionSource === shared.renderRegionSource,
    `${name} services must use the selected database region`,
  );

  assert(
    database.database === `${resourcePrefix}-postgres`,
    `${name} database name drifted`,
  );
  assert(database.version === "17", `${name} database must pin Postgres 17`);
  assert(
    database.regionSource === shared.renderRegionSource,
    `${name} region drifted`,
  );
  assert(
    database.storageGiB === 1,
    `${name} database storage ceiling must be 1 GiB`,
  );
  assert(
    database.storageAutoscaling === false,
    `${name} database storage autoscaling must remain disabled`,
  );
  exactArray(
    database.publicIpAllowList,
    [],
    `${name} database public IP allowlist`,
  );
  assert(database.tlsMode === "require", `${name} database must require TLS`);
  assert(
    new Set(Object.values(databaseRoles)).size === 4,
    `${name} database identities must be distinct`,
  );
  if (isStaging) {
    assert(
      database.plan === "free" && database.pitrDays === 0,
      "staging free database must be synthetic-only with no recovery claim",
    );
  } else {
    assert(
      database.plan === "0.1c-256mb" && database.pitrDays === 7,
      "production must use paid Postgres with the documented PITR window",
    );
  }

  const runtime = api.runtimeVariables;
  const expectedRuntime = {
    RAG_DEBUGGER_ENV: name,
    RAG_DEBUGGER_API_HOST: "0.0.0.0",
    RAG_DEBUGGER_API_PORT: "8080",
    RAG_DEBUGGER_STORAGE_BACKEND: "postgres",
    RAG_DEBUGGER_WEB_ORIGIN: expectedWebOrigin,
    RAG_DEBUGGER_PUBLIC_API_BASE_URL: expectedApiOrigin,
    RAG_DEBUGGER_DEPLOYMENT_MODE: "hosted",
    RAG_DEBUGGER_RELEASE_SHA: "{release_manifest_source_sha}",
    RAG_DEBUGGER_LOG: "info",
    RAG_DEBUGGER_AUTH_PROVIDER: "local",
    RAG_DEBUGGER_SESSION_COOKIE_NAME: isStaging
      ? "__Host-corpuslab_staging_session"
      : "__Host-corpuslab_alpha_session",
    RAG_DEBUGGER_SESSION_TTL_HOURS: "168",
    RAG_DEBUGGER_SESSION_COOKIE_SECURE: "true",
    RAG_DEBUGGER_EMBEDDING_PROVIDER: "local",
    RAG_DEBUGGER_MAX_FILES_PER_REQUEST: "10",
    RAG_DEBUGGER_MAX_FILE_BYTES: "20971520",
    RAG_DEBUGGER_MAX_REQUEST_BYTES: "52428800",
  };
  assert(
    JSON.stringify(runtime) === JSON.stringify(expectedRuntime),
    `${name} hosted runtime safety contract drifted`,
  );
  assert(
    environment.data.automaticProductionClone === false,
    `${name} must forbid automatic production database cloning`,
  );
  assert(
    Number.isInteger(environment.data.logTargetDays) &&
      Number.isInteger(environment.data.providerLogDays),
    `${name} must define log-retention expectations`,
  );
  exactArray(
    environment.render.secrets.map((secret) => secret.name),
    [
      "DATABASE_URL",
      "RAG_DEBUGGER_BOOTSTRAP_PASSWORD",
      "TUNNEL_TOKEN",
      "GHCR_PULL_TOKEN",
    ],
    `${name} Render secret inventory`,
  );
  assert(
    environment.render.secrets.every(
      (secret) =>
        secret.store.toLowerCase().includes(name) && secret.identity.length > 0,
    ),
    `${name} Render secrets must remain in named environment-specific stores`,
  );
}

function workflowTriggersPullRequest(workflow) {
  return Boolean(
    workflow.on &&
    typeof workflow.on === "object" &&
    Object.hasOwn(workflow.on, "pull_request"),
  );
}

function jobEnvironment(job) {
  if (typeof job.environment === "string") {
    return job.environment;
  }
  return job.environment?.name;
}

function validateWorkflowBoundaries(spec, workflows) {
  const production = spec.environments.production;
  const productionSecrets = production.github.secrets;
  const productionResources = [
    production.github.deploymentIdentity,
    production.cloudflare.pages.project,
    production.cloudflare.access.application,
    production.cloudflare.tunnel.tunnel,
    production.render.environment.name,
    production.render.api.service,
    production.render.connector.service,
    production.render.database.database,
  ];

  for (const { path: relativePath, value: workflow } of workflows) {
    assert(
      workflow && typeof workflow === "object" && workflow.jobs,
      `${relativePath} must contain workflow jobs`,
    );
    const isPullRequestWorkflow = workflowTriggersPullRequest(workflow);

    for (const [jobName, job] of Object.entries(workflow.jobs)) {
      const serializedJob = JSON.stringify(job);
      const environment = jobEnvironment(job);
      const isBuildJob = /(^|[-_])build($|[-_])/i.test(jobName);
      if (environment !== "production" || isBuildJob) {
        for (const secret of productionSecrets) {
          assert(
            !serializedJob.includes(secret),
            `${relativePath}:${jobName} cannot reference production secret ${secret}`,
          );
        }
      }

      if (!isPullRequestWorkflow) {
        continue;
      }
      assert(
        environment !== "production",
        `${relativePath}:${jobName} cannot target production from pull_request`,
      );
      for (const resource of productionResources) {
        assert(
          !serializedJob.includes(resource),
          `${relativePath}:${jobName} cannot mutate production resource ${resource} from pull_request`,
        );
      }
      const permissions = {
        ...(workflow.permissions ?? {}),
        ...(job.permissions ?? {}),
      };
      assert(
        permissions.deployments !== "write",
        `${relativePath}:${jobName} cannot write deployments from pull_request`,
      );
    }
  }
}

export function validateInfrastructure(spec, workflows = []) {
  assert(spec.schemaVersion === 1, "infrastructure schemaVersion must be 1");
  assert(
    spec.status === "desired_state_only",
    "repository contract must not claim external provider state",
  );
  exactArray(
    Object.keys(spec.environments),
    ["staging", "production"],
    "environments",
  );
  exactArray(
    spec.operatorInputs.renderRegion.allowed,
    ["oregon", "ohio", "virginia"],
    "approved US Render regions",
  );
  assert(
    Object.values(spec.operatorInputs).every(
      (input) => input.requiredBeforeProvisioning === true,
    ),
    "every operator input must be explicit before provisioning",
  );
  assert(
    spec.shared.renderEnvironmentNetworkIsolation === "enabled" &&
      spec.shared.renderEnvironmentProtection === "enabled",
    "Render isolation and protection must remain enabled",
  );
  assert(
    spec.shared.previewInfrastructure === "disabled",
    "pull-request preview infrastructure must remain disabled",
  );
  assert(
    spec.shared.apiImage.selector === "digest" &&
      spec.shared.apiImage.sourceBuild === false &&
      spec.shared.apiImage.platform === "linux/amd64",
    "API artifact must be the published Linux AMD64 digest",
  );
  assert(
    spec.shared.connectorImage.selector === "digest" &&
      spec.shared.connectorImage.sourceBuild === false &&
      spec.shared.connectorImage.platform === "linux/amd64",
    "connector must use a reviewed Linux AMD64 digest",
  );
  assert(
    spec.shared.renderWorkspacePlan === "pro",
    "Render Pro is required by the selected recovery/log contract",
  );
  assert(
    spec.shared.tls.browser.includes("HTTPS") &&
      spec.shared.tls.database.includes("sslmode=require"),
    "browser and database TLS requirements must be explicit",
  );
  assert(
    spec.shared.costControls.monthlyBudgetUsd === 100 &&
      spec.shared.costControls.operatorReviewThresholdUsd === 75,
    "private-alpha cost ceiling must remain $100 with review at $75",
  );

  const staging = spec.environments.staging;
  const production = spec.environments.production;
  validateEnvironment("staging", staging, spec.shared);
  validateEnvironment("production", production, spec.shared);

  const pairs = [
    [
      staging.github.environment,
      production.github.environment,
      "GitHub Environments",
    ],
    [
      staging.github.deploymentIdentity,
      production.github.deploymentIdentity,
      "deployment identities",
    ],
    [
      staging.cloudflare.pages.project,
      production.cloudflare.pages.project,
      "Pages projects",
    ],
    [
      staging.cloudflare.pages.hostname,
      production.cloudflare.pages.hostname,
      "web hostnames",
    ],
    [
      staging.cloudflare.access.application,
      production.cloudflare.access.application,
      "Access applications",
    ],
    [
      staging.cloudflare.tunnel.tunnel,
      production.cloudflare.tunnel.tunnel,
      "tunnels",
    ],
    [
      staging.cloudflare.tunnel.connectorIdentity,
      production.cloudflare.tunnel.connectorIdentity,
      "connector identities",
    ],
    [
      staging.render.environment.name,
      production.render.environment.name,
      "Render environments",
    ],
    [
      staging.render.api.service,
      production.render.api.service,
      "Render API services",
    ],
    [
      staging.render.connector.service,
      production.render.connector.service,
      "Render connector services",
    ],
    [
      staging.render.database.database,
      production.render.database.database,
      "Postgres instances",
    ],
    [
      staging.render.database.databaseName,
      production.render.database.databaseName,
      "Postgres database names",
    ],
    [
      staging.render.database.endpointRef,
      production.render.database.endpointRef,
      "Postgres endpoint references",
    ],
    [
      staging.render.databaseRoles.runtime,
      production.render.databaseRoles.runtime,
      "runtime database roles",
    ],
    [
      staging.render.databaseRoles.migration,
      production.render.databaseRoles.migration,
      "migration database roles",
    ],
    [
      staging.render.databaseRoles.backup,
      production.render.databaseRoles.backup,
      "backup identities",
    ],
    [
      staging.render.api.runtimeVariables.RAG_DEBUGGER_WEB_ORIGIN,
      production.render.api.runtimeVariables.RAG_DEBUGGER_WEB_ORIGIN,
      "web origins",
    ],
    [
      staging.render.api.runtimeVariables.RAG_DEBUGGER_PUBLIC_API_BASE_URL,
      production.render.api.runtimeVariables.RAG_DEBUGGER_PUBLIC_API_BASE_URL,
      "API origins",
    ],
    [
      staging.render.api.runtimeVariables.RAG_DEBUGGER_SESSION_COOKIE_NAME,
      production.render.api.runtimeVariables.RAG_DEBUGGER_SESSION_COOKIE_NAME,
      "session cookie names",
    ],
  ];
  for (const [stagingValue, productionValue, label] of pairs) {
    distinct(stagingValue, productionValue, label);
  }
  assert(
    staging.github.secrets.every(
      (secret) => !production.github.secrets.includes(secret),
    ),
    "staging and production GitHub secret names must differ",
  );
  for (const [index, secret] of staging.render.secrets.entries()) {
    distinct(
      secret.identity,
      production.render.secrets[index].identity,
      `${secret.name} provider identities`,
    );
    distinct(
      secret.store,
      production.render.secrets[index].store,
      `${secret.name} provider stores`,
    );
  }
  assertNoCredentialValues(spec);
  const serialized = JSON.stringify(spec);
  assert(
    !/(?:gh[pousr]_|github_pat_|-----BEGIN [A-Z ]+PRIVATE KEY-----)/.test(
      serialized,
    ) && !/postgres(?:ql)?:\/\/[^:/"]+:[^@/"]+@/.test(serialized),
    "infrastructure contract contains credential-shaped material",
  );
  validateWorkflowBoundaries(spec, workflows);
}

async function main() {
  const specPath = path.join(repositoryRoot, "infra/private-alpha.json");
  const spec = JSON.parse(await readFile(specPath, "utf8"));
  const workflows = [];
  for (const name of (await readdir(workflowDirectory)).filter((entry) =>
    entry.endsWith(".yml"),
  )) {
    const relativePath = `.github/workflows/${name}`;
    const source = await readFile(
      path.join(repositoryRoot, relativePath),
      "utf8",
    );
    try {
      workflows.push({ path: relativePath, value: await parseYaml(source) });
    } catch (error) {
      throw new Error(`${relativePath} is not valid YAML: ${error.message}`, {
        cause: error,
      });
    }
  }
  validateInfrastructure(spec, workflows);
  console.log(
    `Infrastructure validation passed for staging, production, and ${workflows.length} workflows.`,
  );
}

const invokedPath = process.argv[1] && path.resolve(process.argv[1]);
if (invokedPath === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(`Infrastructure validation failed: ${error.message}`);
    process.exitCode = 1;
  });
}
