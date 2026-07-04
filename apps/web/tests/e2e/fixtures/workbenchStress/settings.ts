import type { ApiKey } from "../../../../src/lib/api/apiKeys";

import { stressIds, stressValues, unbrokenToken } from "./shared";

export const stressApiKey = {
  id: stressIds.apiKey,
  name: stressValues.apiKeyName,
  prefix: `clab_${unbrokenToken}`,
  scopes: ["ci_eval_runs"],
  created_at: "2026-07-04T08:00:00Z",
  last_used_at: "2026-07-04T08:01:00Z",
  revoked_at: null,
} satisfies ApiKey;
