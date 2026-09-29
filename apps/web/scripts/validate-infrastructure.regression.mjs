import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { validateInfrastructure } from "./validate-infrastructure.mjs";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const validSpec = JSON.parse(
  await readFile(
    path.resolve(scriptDirectory, "../../../infra/private-alpha.json"),
    "utf8",
  ),
);

function copySpec() {
  return JSON.parse(JSON.stringify(validSpec));
}

function pullRequestWorkflow(job) {
  return [
    {
      path: ".github/workflows/test.yml",
      value: {
        on: { pull_request: null },
        permissions: { contents: "read" },
        jobs: { test: job },
      },
    },
  ];
}

test("accepts the isolated private-alpha foundation contract", () => {
  assert.doesNotThrow(() => validateInfrastructure(copySpec()));
});

test("rejects staging and production identity reuse", async (context) => {
  const cases = [
    [
      "Pages project",
      /production Pages name drifted/,
      (spec) => {
        spec.environments.production.cloudflare.pages.project =
          spec.environments.staging.cloudflare.pages.project;
      },
    ],
    [
      "Access application",
      /production Access application name drifted/,
      (spec) => {
        spec.environments.production.cloudflare.access.application =
          spec.environments.staging.cloudflare.access.application;
      },
    ],
    [
      "tunnel",
      /production tunnel name drifted/,
      (spec) => {
        spec.environments.production.cloudflare.tunnel.tunnel =
          spec.environments.staging.cloudflare.tunnel.tunnel;
      },
    ],
    [
      "connector",
      /production connector name drifted/,
      (spec) => {
        spec.environments.production.render.connector.service =
          spec.environments.staging.render.connector.service;
      },
    ],
    [
      "API service",
      /production API name drifted/,
      (spec) => {
        spec.environments.production.render.api.service =
          spec.environments.staging.render.api.service;
      },
    ],
    [
      "database",
      /production database name drifted/,
      (spec) => {
        spec.environments.production.render.database.database =
          spec.environments.staging.render.database.database;
      },
    ],
    [
      "database endpoint",
      /Postgres endpoint references must differ/,
      (spec) => {
        spec.environments.production.render.database.endpointRef =
          spec.environments.staging.render.database.endpointRef;
      },
    ],
    [
      "runtime database role",
      /runtime database roles must differ/,
      (spec) => {
        spec.environments.production.render.databaseRoles.runtime =
          spec.environments.staging.render.databaseRoles.runtime;
      },
    ],
    [
      "migration database role",
      /migration database roles must differ/,
      (spec) => {
        spec.environments.production.render.databaseRoles.migration =
          spec.environments.staging.render.databaseRoles.migration;
      },
    ],
    [
      "cookie",
      /hosted runtime safety contract drifted/,
      (spec) => {
        spec.environments.production.render.api.runtimeVariables.RAG_DEBUGGER_SESSION_COOKIE_NAME =
          spec.environments.staging.render.api.runtimeVariables.RAG_DEBUGGER_SESSION_COOKIE_NAME;
      },
    ],
    [
      "deployment identity",
      /deployment identities must differ/,
      (spec) => {
        spec.environments.production.github.deploymentIdentity =
          spec.environments.staging.github.deploymentIdentity;
      },
    ],
  ];

  for (const [name, expected, mutate] of cases) {
    await context.test(name, () => {
      const spec = copySpec();
      mutate(spec);
      assert.throws(() => validateInfrastructure(spec), expected);
    });
  }
});

test("rejects weakened Access, TLS, data, and resource controls", async (context) => {
  const cases = [
    [
      "wildcard Access hostname",
      /Access domains must be exactly/,
      (spec) => {
        spec.environments.staging.cloudflare.access.domains[0] =
          "*.staging.{operator_domain}";
      },
    ],
    [
      "disabled eager cookie",
      /must enable eager redirect cookies/,
      (spec) => {
        spec.environments.staging.cloudflare.access.eagerRedirectCookie = false;
      },
    ],
    [
      "extra Access bypass",
      /bypass methods must be exactly OPTIONS/,
      (spec) => {
        spec.environments.production.cloudflare.access.bypassMethods.push(
          "GET",
        );
      },
    ],
    [
      "non-TLS database",
      /database must require TLS/,
      (spec) => {
        spec.environments.production.render.database.tlsMode = "disable";
      },
    ],
    [
      "public API",
      /must not expose a provider public origin/,
      (spec) => {
        spec.environments.production.render.api.publicProviderOrigin = true;
      },
    ],
    [
      "source-built API",
      /published digest without a source build/,
      (spec) => {
        spec.environments.production.render.api.sourceBuild = true;
      },
    ],
    [
      "uncapped API instances",
      /must cap instances at one/,
      (spec) => {
        spec.environments.production.render.api.maxInstances = 2;
      },
    ],
    [
      "storage autoscaling",
      /storage autoscaling must remain disabled/,
      (spec) => {
        spec.environments.production.render.database.storageAutoscaling = true;
      },
    ],
    [
      "memory storage",
      /hosted runtime safety contract drifted/,
      (spec) => {
        spec.environments.production.render.api.runtimeVariables.RAG_DEBUGGER_STORAGE_BACKEND =
          "memory";
      },
    ],
    [
      "cross-region connector",
      /services must use the selected database region/,
      (spec) => {
        spec.environments.staging.render.connector.regionSource =
          "OTHER_REGION";
      },
    ],
    [
      "production clone",
      /must forbid automatic production database cloning/,
      (spec) => {
        spec.environments.staging.data.automaticProductionClone = true;
      },
    ],
  ];

  for (const [name, expected, mutate] of cases) {
    await context.test(name, () => {
      const spec = copySpec();
      mutate(spec);
      assert.throws(() => validateInfrastructure(spec), expected);
    });
  }
});

test("rejects committed credential values", () => {
  const namedValue = copySpec();
  namedValue.environments.production.render.secrets[0].value = "not-allowed";
  assert.throws(
    () => validateInfrastructure(namedValue),
    /must name a secret, never store its value/,
  );

  const credentialShape = copySpec();
  credentialShape.shared.repository = "github_pat_not-a-real-value";
  assert.throws(
    () => validateInfrastructure(credentialShape),
    /credential-shaped material/,
  );
});

test("rejects production authority in pull-request, build, or staging jobs", () => {
  const spec = copySpec();
  const productionSecret = spec.environments.production.github.secrets[0];

  assert.throws(
    () =>
      validateInfrastructure(
        spec,
        pullRequestWorkflow({
          environment: "production",
          runsOn: "ubuntu-latest",
        }),
      ),
    /cannot target production from pull_request/,
  );
  assert.throws(
    () =>
      validateInfrastructure(
        spec,
        pullRequestWorkflow({
          runsOn: "ubuntu-latest",
          env: { TOKEN: `\${{ secrets.${productionSecret} }}` },
        }),
      ),
    /cannot reference production secret/,
  );
  assert.throws(
    () =>
      validateInfrastructure(
        spec,
        pullRequestWorkflow({
          runsOn: "ubuntu-latest",
          run: "update corpuslab-production-api",
        }),
      ),
    /cannot mutate production resource/,
  );
  assert.throws(
    () =>
      validateInfrastructure(
        spec,
        pullRequestWorkflow({
          permissions: { deployments: "write" },
          runsOn: "ubuntu-latest",
        }),
      ),
    /cannot write deployments from pull_request/,
  );

  const stagingWorkflow = pullRequestWorkflow({
    environment: "staging",
    runsOn: "ubuntu-latest",
    env: { TOKEN: `\${{ secrets.${productionSecret} }}` },
  });
  stagingWorkflow[0].value.on = { push: { branches: ["main"] } };
  assert.throws(
    () => validateInfrastructure(spec, stagingWorkflow),
    /cannot reference production secret/,
  );

  const buildWorkflow = pullRequestWorkflow({
    environment: "production",
    runsOn: "ubuntu-latest",
    env: { TOKEN: `\${{ secrets.${productionSecret} }}` },
  });
  buildWorkflow[0].value.on = { push: { branches: ["main"] } };
  buildWorkflow[0].value.jobs = { build_api: buildWorkflow[0].value.jobs.test };
  assert.throws(
    () => validateInfrastructure(spec, buildWorkflow),
    /cannot reference production secret/,
  );
});
