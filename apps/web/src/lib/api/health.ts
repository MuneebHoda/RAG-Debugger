import { requestJson } from "./client";

export interface HealthResponse {
  status: string;
}

export function getHealth(signal?: AbortSignal): Promise<HealthResponse> {
  return requestJson<HealthResponse>("/healthz", { signal });
}
