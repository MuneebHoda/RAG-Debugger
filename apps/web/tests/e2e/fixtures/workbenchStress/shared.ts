export const stressIds = {
  project: "018f7a2a-6e2e-7000-a000-000000004800",
  source: "018f7a2a-6e2e-7000-a000-000000004801",
  document: "018f7a2a-6e2e-7000-a000-000000004802",
  chunk: "018f7a2a-6e2e-7000-a000-000000004803",
  run: "018f7a2a-6e2e-7000-a000-000000004804",
  trace: "018f7a2a-6e2e-7000-a000-000000004805",
  report: "018f7a2a-6e2e-7000-a000-000000004806",
  dataset: "018f7a2a-6e2e-7000-a000-000000004807",
  case: "018f7a2a-6e2e-7000-a000-000000004808",
  experiment: "018f7a2a-6e2e-7000-a000-000000004809",
  apiKey: "018f7a2a-6e2e-7000-a000-000000004810",
} as const;

export const unbrokenToken = `sha256_${"evidence".repeat(28)}`;

export const stressValues = {
  ...stressIds,
  documentPath:
    `enterprise/policies/2026/retention/${"cross-region-replication/".repeat(8)}` +
    `${unbrokenToken}.md`,
  query:
    "Explain exactly why the cross-region account-recovery evidence ranked below duplicate semantic candidates when every lexical, vector, metadata, section, citation, and answerability signal was present in the same retrieval run?",
  snippet: unbrokenToken,
  reportTitle:
    "[Release audit] **Cross-region recovery** | score_delta <= 0.0001 | " +
    unbrokenToken,
  apiKeyName: `GitHub Actions ${unbrokenToken}`,
} as const;
