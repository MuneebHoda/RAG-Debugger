import {
  AlertCircle,
  Check,
  Copy,
  Database,
  KeyRound,
  ServerCog,
  ShieldCheck,
  Trash2,
  Users,
} from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useSearchParams } from "react-router-dom";

import {
  createApiKey,
  listApiKeys,
  revokeApiKey,
  type ApiKey,
} from "../lib/api/apiKeys";
import { getCurrentUser } from "../lib/api/auth";
import { getProductConfig } from "../lib/api/config";
import { formatDateTime } from "../lib/dateTime";
import styles from "./SettingsPage.module.css";

const tabs = ["workspace", "api-keys", "runtime", "privacy"] as const;
type SettingsTab = (typeof tabs)[number];

export function SettingsPage() {
  const [searchParams, setSearchParams] = useSearchParams();
  const tabParam = searchParams.get("tab");
  const activeTab: SettingsTab = tabs.includes(tabParam as SettingsTab)
    ? (tabParam as SettingsTab)
    : "workspace";
  const configQuery = useQuery({
    queryKey: ["product-config"],
    queryFn: ({ signal }) => getProductConfig(signal),
  });
  const userQuery = useQuery({
    queryKey: ["current-user"],
    queryFn: ({ signal }) => getCurrentUser(signal),
  });
  const keysQuery = useQuery({
    queryKey: ["api-keys"],
    queryFn: ({ signal }) => listApiKeys(signal),
    enabled: activeTab === "api-keys",
  });
  const error = configQuery.error ?? userQuery.error ?? keysQuery.error;

  return (
    <section className={styles.page} aria-labelledby="settings-title">
      <header className={styles.header}>
        <div>
          <p>Workspace</p>
          <h1 id="settings-title">Settings</h1>
          <span>
            Manage access first; inspect runtime detail only when needed.
          </span>
        </div>
      </header>

      <div
        className={styles.tabs}
        role="tablist"
        aria-label="Settings sections"
      >
        <Tab
          active={activeTab === "workspace"}
          icon={Users}
          label="Workspace"
          onClick={() => setSearchParams({ tab: "workspace" })}
        />
        <Tab
          active={activeTab === "api-keys"}
          icon={KeyRound}
          label="API keys"
          onClick={() => setSearchParams({ tab: "api-keys" })}
        />
        <Tab
          active={activeTab === "runtime"}
          icon={ServerCog}
          label="Runtime"
          onClick={() => setSearchParams({ tab: "runtime" })}
        />
        <Tab
          active={activeTab === "privacy"}
          icon={ShieldCheck}
          label="Privacy"
          onClick={() => setSearchParams({ tab: "privacy" })}
        />
      </div>

      {error ? (
        <div className={styles.alert} role="alert">
          <AlertCircle aria-hidden="true" size={18} />
          <span>
            {error instanceof Error
              ? error.message
              : "Settings could not be loaded."}
          </span>
        </div>
      ) : null}

      {activeTab === "workspace" ? (
        <WorkspacePanel user={userQuery.data?.user} />
      ) : null}
      {activeTab === "api-keys" ? (
        <ApiKeysPanel apiKeys={keysQuery.data ?? []} />
      ) : null}
      {activeTab === "runtime" ? (
        <RuntimePanel config={configQuery.data} />
      ) : null}
      {activeTab === "privacy" ? <PrivacyPanel /> : null}
    </section>
  );
}

function WorkspacePanel({
  user,
}: {
  user: Awaited<ReturnType<typeof getCurrentUser>>["user"] | undefined;
}) {
  return (
    <section className={styles.panel}>
      <div className={styles.panelHeading}>
        <div>
          <h2>Workspace</h2>
          <p>Your active team and access level.</p>
        </div>
        <Users aria-hidden="true" size={18} />
      </div>
      <dl className={styles.definitionList}>
        <Definition
          label="Workspace"
          value={user?.workspace.name ?? "Loading…"}
        />
        <Definition
          label="Organization"
          value={user?.organization.name ?? "Loading…"}
        />
        <Definition label="Your role" value={user?.role ?? "Loading…"} />
        <Definition
          label="Signed in as"
          value={user?.user.email ?? "Loading…"}
        />
      </dl>
    </section>
  );
}

function ApiKeysPanel({ apiKeys }: { apiKeys: ApiKey[] }) {
  const queryClient = useQueryClient();
  const [name, setName] = useState("GitHub Actions");
  const [createdSecret, setCreatedSecret] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const createMutation = useMutation({
    mutationFn: () =>
      createApiKey({ name: name.trim(), scopes: ["ci_eval_runs"] }),
    onSuccess: (created) => {
      setCreatedSecret(created.secret);
      void queryClient.invalidateQueries({ queryKey: ["api-keys"] });
    },
  });
  const revokeMutation = useMutation({
    mutationFn: (apiKeyId: string) => revokeApiKey(apiKeyId),
    onSuccess: () =>
      void queryClient.invalidateQueries({ queryKey: ["api-keys"] }),
  });

  return (
    <section className={styles.panel} aria-labelledby="api-keys-title">
      <div className={styles.panelHeading}>
        <div>
          <h2 id="api-keys-title">API keys</h2>
          <p>Create workspace-scoped credentials for CI quality gates.</p>
        </div>
        <KeyRound aria-hidden="true" size={18} />
      </div>

      {createdSecret ? (
        <div className={styles.secretBox} aria-label="Created API key secret">
          <span>This secret is shown once</span>
          <code>{createdSecret}</code>
          <button
            type="button"
            onClick={() => {
              void navigator.clipboard.writeText(createdSecret);
              setCopied(true);
            }}
          >
            {copied ? (
              <Check aria-hidden="true" size={15} />
            ) : (
              <Copy aria-hidden="true" size={15} />
            )}
            {copied ? "Copied" : "Copy"}
          </button>
        </div>
      ) : null}

      <div className={styles.createRow}>
        <label>
          Key name
          <input
            value={name}
            onChange={(event) => setName(event.currentTarget.value)}
          />
        </label>
        <button
          className={styles.primaryButton}
          disabled={!name.trim() || createMutation.isPending}
          type="button"
          onClick={() => createMutation.mutate()}
        >
          <KeyRound aria-hidden="true" size={15} /> Create key
        </button>
      </div>

      <div className={styles.keyList}>
        {apiKeys.map((apiKey) => (
          <article className={styles.keyRow} key={apiKey.id}>
            <span>
              <strong>{apiKey.name}</strong>
              <small>
                {apiKey.prefix}… ·{" "}
                {apiKey.revoked_at
                  ? "revoked"
                  : apiKey.last_used_at
                    ? `last used ${formatDateTime(apiKey.last_used_at)}`
                    : "not used yet"}
              </small>
            </span>
            {!apiKey.revoked_at ? (
              <button
                aria-label={`Revoke ${apiKey.name}`}
                type="button"
                onClick={() => revokeMutation.mutate(apiKey.id)}
              >
                <Trash2 aria-hidden="true" size={14} /> Revoke
              </button>
            ) : null}
          </article>
        ))}
        {apiKeys.length === 0 ? (
          <p className={styles.empty}>No API keys yet.</p>
        ) : null}
      </div>
    </section>
  );
}

function RuntimePanel({
  config,
}: {
  config: Awaited<ReturnType<typeof getProductConfig>> | undefined;
}) {
  return (
    <div className={styles.grid}>
      <ConfigPanel
        icon={ServerCog}
        title="Product"
        items={[
          ["Name", config?.product.name ?? "Loading…"],
          ["Deployment", config?.product.deployment_mode ?? "Loading…"],
          ["API base", config?.ui.api_base_url ?? "Loading…"],
        ]}
      />
      <ConfigPanel
        icon={Database}
        title="Ingestion"
        items={[
          [
            "Max files/request",
            String(config?.ingestion.max_files_per_request ?? "Loading…"),
          ],
          ["Max file", formatBytes(config?.ingestion.max_file_bytes ?? 0)],
          [
            "Extensions",
            config?.ingestion.supported_extensions.join(", ") ?? "Loading…",
          ],
        ]}
      />
      <ConfigPanel
        icon={Database}
        title="Chunking"
        items={[
          ["Strategy", config?.chunking.strategy ?? "Loading…"],
          [
            "Target tokens",
            String(config?.chunking.target_tokens ?? "Loading…"),
          ],
          [
            "Overlap tokens",
            String(config?.chunking.overlap_tokens ?? "Loading…"),
          ],
        ]}
      />
      <ConfigPanel
        icon={ServerCog}
        title="Retrieval"
        items={[
          ["Default mode", config?.retrieval.default_mode ?? "Loading…"],
          ["Max top-k", String(config?.retrieval.max_top_k ?? "Loading…")],
          ["Embedding model", config?.embedding.model.model_name ?? "Loading…"],
        ]}
      />
    </div>
  );
}

function PrivacyPanel() {
  return (
    <section className={styles.panel}>
      <div className={styles.panelHeading}>
        <div>
          <h2>Privacy posture</h2>
          <p>What CorpusLab stores and what remains outside the system.</p>
        </div>
        <ShieldCheck aria-hidden="true" size={18} />
      </div>
      <div className={styles.privacyList}>
        <PrivacyItem
          title="Original files are not retained"
          detail="CorpusLab stores extracted chunk text and metadata, not uploaded binaries."
        />
        <PrivacyItem
          title="Embeddings stay with your workspace"
          detail="The configured local provider does not send chunk text to a hosted model."
        />
        <PrivacyItem
          title="Secrets remain server-side"
          detail="Database URLs, password hashes, sessions, and API-key hashes are never exposed by runtime config."
        />
      </div>
    </section>
  );
}

function Tab({
  active,
  icon: Icon,
  label,
  onClick,
}: {
  active: boolean;
  icon: typeof Users;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      aria-selected={active}
      className={active ? styles.activeTab : styles.tab}
      role="tab"
      type="button"
      onClick={onClick}
    >
      <Icon aria-hidden="true" size={15} /> {label}
    </button>
  );
}

function ConfigPanel({
  icon: Icon,
  title,
  items,
}: {
  icon: typeof ServerCog;
  title: string;
  items: Array<[string, string]>;
}) {
  return (
    <section className={styles.panel}>
      <div className={styles.panelHeading}>
        <h2>{title}</h2>
        <Icon aria-hidden="true" size={18} />
      </div>
      <dl className={styles.definitionList}>
        {items.map(([label, value]) => (
          <Definition key={label} label={label} value={value} />
        ))}
      </dl>
    </section>
  );
}

function Definition({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt>{label}</dt>
      <dd>{value}</dd>
    </div>
  );
}

function PrivacyItem({ title, detail }: { title: string; detail: string }) {
  return (
    <article>
      <ShieldCheck aria-hidden="true" size={18} />
      <span>
        <strong>{title}</strong>
        <small>{detail}</small>
      </span>
    </article>
  );
}

function formatBytes(bytes: number) {
  if (bytes === 0) return "Loading…";
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${Math.round(bytes / 1024 / 1024)} MB`;
}
