import { jsonRequest, requestJson } from "./client";

export type WorkspaceRole = "owner" | "admin" | "member" | "viewer";

export interface AuthenticatedUser {
  user: {
    id: string;
    email: string;
    name: string;
    created_at: string;
  };
  organization: {
    id: string;
    name: string;
    created_at: string;
  };
  workspace: {
    id: string;
    organization_id: string;
    name: string;
    created_at: string;
  };
  role: WorkspaceRole;
}

export interface AuthResponse {
  user: AuthenticatedUser;
}

export interface SignupRequest {
  email: string;
  password: string;
  name?: string;
  workspace_name: string;
}

export interface LoginRequest {
  email: string;
  password: string;
}

export function signup(
  request: SignupRequest,
  signal?: AbortSignal,
): Promise<AuthResponse> {
  return requestJson<AuthResponse>(
    "/api/v1/auth/signup",
    jsonRequest("POST", request, signal),
  );
}

export function login(
  request: LoginRequest,
  signal?: AbortSignal,
): Promise<AuthResponse> {
  return requestJson<AuthResponse>(
    "/api/v1/auth/login",
    jsonRequest("POST", request, signal),
  );
}

export function logout(signal?: AbortSignal): Promise<{ ok: boolean }> {
  return requestJson<{ ok: boolean }>(
    "/api/v1/auth/logout",
    jsonRequest("POST", {}, signal),
  );
}

export function getCurrentUser(signal?: AbortSignal): Promise<AuthResponse> {
  return requestJson<AuthResponse>("/api/v1/auth/me", { signal });
}
