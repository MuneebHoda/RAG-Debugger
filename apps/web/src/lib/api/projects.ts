import { requestJson } from "./client";

export interface CurrentProject {
  id: string;
  name: string;
  privacy_mode: "LocalOnly" | "RedactedCloudSync" | "ExplicitSnippetSync";
  created_at: string;
  updated_at: string;
}

export function getCurrentProject(
  signal?: AbortSignal,
): Promise<CurrentProject> {
  return requestJson<CurrentProject>("/api/v1/projects/current", { signal });
}
