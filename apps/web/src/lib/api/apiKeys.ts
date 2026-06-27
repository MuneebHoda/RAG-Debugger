import { jsonRequest, requestJson } from "./client";

export type ApiKeyScope = "ci_eval_runs";

export interface ApiKey {
  id: string;
  workspace_id: string;
  name: string;
  prefix: string;
  scopes: ApiKeyScope[];
  created_at: string;
  last_used_at: string | null;
  revoked_at: string | null;
}

export interface CreateApiKeyRequest {
  name: string;
  scopes?: ApiKeyScope[];
}

export interface CreatedApiKey {
  api_key: ApiKey;
  secret: string;
}

export function listApiKeys(signal?: AbortSignal): Promise<ApiKey[]> {
  return requestJson<ApiKey[]>("/api/v1/api-keys", { signal });
}

export function createApiKey(
  request: CreateApiKeyRequest,
  signal?: AbortSignal,
): Promise<CreatedApiKey> {
  return requestJson<CreatedApiKey>(
    "/api/v1/api-keys",
    jsonRequest("POST", request, signal),
  );
}

export async function revokeApiKey(
  apiKeyId: string,
  signal?: AbortSignal,
): Promise<void> {
  await requestJson<{ revoked: boolean }>(
    `/api/v1/api-keys/${apiKeyId}`,
    jsonRequest("DELETE", {}, signal),
  );
}
