# Auth And Workspaces

CorpusLab now has a local Postgres-backed auth boundary that prepares the product for hosted/team use without taking on billing, SSO, invitations, or enterprise admin flows yet.

## What Exists

- Local signup and login.
- Opaque HttpOnly session cookies.
- Current-user lookup through `GET /api/v1/auth/me`.
- Organizations, workspaces, users, and workspace memberships.
- Workspace-scoped API keys for CI automation.
- Bootstrap demo identity from environment values.
- Protected workbench APIs by default.

Public marketing pages, `/healthz`, `/readyz`, `/api/v1/config`, and auth routes stay unauthenticated. Workbench APIs require a valid session. CI eval run creation requires an API key with the `ci_eval_runs` scope.

## Environment

Local non-secret defaults live in `.env.example`:

```text
RAG_DEBUGGER_AUTH_PROVIDER=local
RAG_DEBUGGER_SESSION_COOKIE_NAME=corpuslab_session
RAG_DEBUGGER_SESSION_TTL_HOURS=168
RAG_DEBUGGER_SESSION_COOKIE_SECURE=false
RAG_DEBUGGER_BOOTSTRAP_EMAIL=demo@corpuslab.ai
RAG_DEBUGGER_BOOTSTRAP_PASSWORD=
RAG_DEBUGGER_BOOTSTRAP_USER_NAME=Demo User
RAG_DEBUGGER_BOOTSTRAP_ORGANIZATION=CorpusLab Demo Organization
RAG_DEBUGGER_BOOTSTRAP_WORKSPACE=Corpus Demo Workspace
```

Generate a password for each local checkout, then put it in the ignored `.env`
file:

```sh
openssl rand -base64 32
```

The API requires `RAG_DEBUGGER_BOOTSTRAP_PASSWORD` and rejects missing or empty
values. It never sends that password through `GET /api/v1/config`, and the login
page neither displays nor prefills credentials. Deployments should provide the
password through their secret manager or environment injection mechanism.

Use `RAG_DEBUGGER_SESSION_COOKIE_SECURE=true` behind HTTPS. Keep it `false` for local `http://127.0.0.1` development.

## API Flow

Signup:

```http
POST /api/v1/auth/signup
```

Creates a user, organization, workspace, owner membership, and session cookie.

Login:

```http
POST /api/v1/auth/login
```

Verifies the password hash and returns the authenticated user context.

Logout:

```http
POST /api/v1/auth/logout
```

Revokes the current session and clears the browser cookie.

Current user:

```http
GET /api/v1/auth/me
```

Returns user, organization, workspace, and role when the session cookie is valid.

Current workspace:

```http
GET /api/v1/workspaces/current
```

Returns the active organization/workspace/role context for the workbench.

## Storage Model

The migration `migrations/20260625170000_hosted_ci_eval_workflows.sql` adds:

- `organizations`
- `workspaces`
- `users`
- `workspace_memberships`
- `auth_sessions`
- `api_keys`
- `ci_eval_runs`

Passwords are hashed with Argon2. Sessions and API key secrets are hashed before storage. API key prefixes are stored for display and support workflows.

`migrations/20260726120000_workspace_evidence_isolation.sql` attaches Eval Lab datasets and legacy eval runs to workspaces. `migrations/20260727120000_trace_embedding_workspace_isolation.sql` adds workspace ownership and workspace/date indexes for retrieval runs and traces. Projects, sources, documents, chunks, evidence search, embedding reads and writes, retrieval candidates and saved runs, traces and reruns, datasets, cases, experiments, CI runs, reports, demo progress, and overview reads are authorized at the repository boundary. Protected-route middleware validates the session in every runtime environment and inserts the resulting `AuthenticatedUser` for handlers; tests use real memberships and session cookies rather than bypassing this boundary.

Corpus, embedding, retrieval, and evidence operations currently span all projects owned by the active workspace because sessions do not yet carry an active-project selection. Source and document filters narrow that set and are still ownership checked. Trace creation verifies both the trace project and its optional source retrieval run. Cross-workspace and nonexistent runs, traces, and other resources return equivalent unresolved-ID or resource-specific `404` responses.

Legacy ownership is privacy conservative. Projects, Eval records, retrieval runs, and traces with one uniquely attributable workspace are backfilled. Records in a single-workspace installation are assigned to that workspace. Ambiguous records in a multi-workspace installation remain unowned and are quarantined from normal reads until deliberately repaired. Embedding ownership is derived from the owning chunk's document, source, and project rather than duplicated on each vector row.

## Provider Boundary

`RAG_DEBUGGER_AUTH_PROVIDER=local` is the only implemented provider in this pass. The code shape keeps auth behavior behind API helpers and repository methods so a hosted provider can later validate identity/session state without rewriting route handlers.

Future provider work should add:

- External identity validation.
- Invitations.
- SSO/SAML.
- SCIM.
- Detailed RBAC.
- Audit events.
- Organization admin UI.

## Security Notes

- Never log full API keys or session tokens.
- Show API key secrets once.
- Revoke keys rather than deleting audit history.
- Keep CI keys workspace-scoped.
- Prefer short-lived sessions in hosted deployments.
- Require HTTPS before setting secure cookies.
- Keep bootstrap passwords and database credentials out of committed files.
