export const DEMO_CREDENTIALS = {
  email: "demo@corpuslab.ai",
  password: "CorpusLab#2026",
  workspaceName: "Corpus Demo Workspace",
};

export const AUTH_STORAGE_KEY = "corpuslab.auth.session";

export type AuthSession = {
  email: string;
  workspaceName: string;
  issuedAt: string;
};

export function authenticateDemoUser(email: string, password: string) {
  return (
    email.trim().toLowerCase() === DEMO_CREDENTIALS.email &&
    password === DEMO_CREDENTIALS.password
  );
}

export function createAuthSession(
  email: string,
  workspaceName = DEMO_CREDENTIALS.workspaceName,
) {
  const session: AuthSession = {
    email: email.trim().toLowerCase(),
    workspaceName: workspaceName.trim() || DEMO_CREDENTIALS.workspaceName,
    issuedAt: new Date().toISOString(),
  };

  window.localStorage.setItem(AUTH_STORAGE_KEY, JSON.stringify(session));
  window.dispatchEvent(new Event("corpuslab-auth-change"));
  return session;
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
