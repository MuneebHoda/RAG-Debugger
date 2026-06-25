export const DEMO_CREDENTIALS = {
  email: "demo@corpuslab.ai",
  password: "CorpusLab#2026",
  workspaceName: "Corpus Demo Workspace",
};

export const AUTH_STORAGE_KEY = "corpuslab.auth.session";

export type AuthSession = {
  email: string;
  name: string;
  workspaceName: string;
  role: string;
  issuedAt: string;
};

export function createAuthSession(
  email: string,
  name = email,
  workspaceName = DEMO_CREDENTIALS.workspaceName,
  role = "owner",
) {
  const session: AuthSession = {
    email: email.trim().toLowerCase(),
    name: name.trim() || email.trim().toLowerCase(),
    workspaceName: workspaceName.trim() || DEMO_CREDENTIALS.workspaceName,
    role,
    issuedAt: new Date().toISOString(),
  };

  window.localStorage.setItem(AUTH_STORAGE_KEY, JSON.stringify(session));
  window.dispatchEvent(new Event("corpuslab-auth-change"));
  return session;
}

export function createAuthSessionFromResponse(
  response: import("../../lib/api/auth").AuthResponse,
) {
  return createAuthSession(
    response.user.user.email,
    response.user.user.name,
    response.user.workspace.name,
    response.user.role,
  );
}

export function readAuthSession(): AuthSession | null {
  const value = window.localStorage.getItem(AUTH_STORAGE_KEY);
  if (!value) {
    return null;
  }

  try {
    return JSON.parse(value) as AuthSession;
  } catch {
    clearAuthSession();
    return null;
  }
}

export function clearAuthSession() {
  window.localStorage.removeItem(AUTH_STORAGE_KEY);
  window.dispatchEvent(new Event("corpuslab-auth-change"));
}
