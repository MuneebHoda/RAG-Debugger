import {
  AlertCircle,
  Database,
  KeyRound,
  ServerCog,
  ShieldCheck,
  Trash2,
} from "lucide-react";
import { useEffect, useState } from "react";

import {
  createApiKey,
  listApiKeys,
  revokeApiKey,
  type ApiKey,
} from "../lib/api/apiKeys";
import { getProductConfig, type ProductConfig } from "../lib/api/config";

export function SettingsPage() {
  const [config, setConfig] = useState<ProductConfig | null>(null);
  const [apiKeys, setApiKeys] = useState<ApiKey[]>([]);
  const [apiKeyName, setApiKeyName] = useState("GitHub Actions");
  const [createdSecret, setCreatedSecret] = useState<string | null>(null);
  const [isCreatingKey, setIsCreatingKey] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    Promise.all([
      getProductConfig(controller.signal),
      listApiKeys(controller.signal),
    ])
      .then(([nextConfig, nextApiKeys]) => {
        setConfig(nextConfig);
        setApiKeys(nextApiKeys);
      })
      .catch((cause: unknown) => {
        if (!controller.signal.aborted) {
          setError(cause instanceof Error ? cause.message : "Request failed");
        }
      });

    return () => controller.abort();
  }, []);

  return (
    <section className="settings-page" aria-labelledby="settings-title">
      <header className="page-header">
        <div>
          <p className="eyebrow">Runtime configuration</p>
          <h1 id="settings-title">Settings</h1>
          <p>
            Inspect safe product configuration published by the API. Deployment
            secrets and database URLs remain server-only.
          </p>
        </div>
      </header>

      {error ? (
        <div className="alert" role="alert">
          <AlertCircle aria-hidden="true" size={18} />
          <span>{error}</span>
        </div>
      ) : null}

      <section className="settings-grid">
        <ConfigPanel
          icon={ServerCog}
          title="Product"
          items={[
            ["Name", config?.product.name ?? "--"],
            ["Workspace", config?.product.workspace_name ?? "--"],
            ["Deployment", config?.product.deployment_mode ?? "--"],
            ["API base", config?.ui.api_base_url ?? "--"],
          ]}
        />
        <ConfigPanel
          icon={Database}
          title="Ingestion"
          items={[
            [
              "Max files/request",
              String(config?.ingestion.max_files_per_request ?? "--"),
            ],
            ["Max file", formatBytes(config?.ingestion.max_file_bytes ?? 0)],
            [
              "Max request",
              formatBytes(config?.ingestion.max_request_bytes ?? 0),
            ],
            [
              "Extensions",
              config?.ingestion.supported_extensions.join(", ") ?? "--",
            ],
          ]}
        />
        <ConfigPanel
          icon={Database}
          title="Chunking"
          items={[
            ["Strategy", config?.chunking.strategy ?? "--"],
            ["Target tokens", String(config?.chunking.target_tokens ?? "--")],
            ["Overlap tokens", String(config?.chunking.overlap_tokens ?? "--")],
            [
              "Preview chunks",
              String(config?.ingestion.preview_chunk_limit ?? "--"),
            ],
          ]}
        />
        <ConfigPanel
          icon={ShieldCheck}
          title="Retrieval & Privacy"
          items={[
            ["Default mode", config?.retrieval.default_mode ?? "--"],
            ["Max top-k", String(config?.retrieval.max_top_k ?? "--")],
            [
              "Min evidence",
              String(config?.retrieval.min_evidence_score ?? "--"),
            ],
            ["Embedding model", config?.embedding.model.model_name ?? "--"],
            ["Provider", config?.embedding.model.provider ?? "--"],
            ["Privacy posture", "chunk text and embeddings stay in storage"],
          ]}
        />
      </section>

      <section className="panel config-panel" aria-labelledby="api-keys-title">
        <div className="panel-heading">
          <h2 id="api-keys-title">CI API Keys</h2>
          <KeyRound aria-hidden="true" size={18} />
        </div>
        <p>
          Generate workspace-scoped keys for GitHub Actions and CI eval gates.
          Secrets are shown once and stored only as hashes by the API.
        </p>
        {createdSecret ? (
          <div className="secret-box" aria-label="Created API key secret">
            <span>Copy this secret now</span>
            <code>{createdSecret}</code>
          </div>
        ) : null}
        <div className="config-grid">
          <label>
            Key name
            <input
              value={apiKeyName}
              onChange={(event) => setApiKeyName(event.currentTarget.value)}
            />
          </label>
          <button
            className="primary-button"
            disabled={!apiKeyName.trim() || isCreatingKey}
            type="button"
            onClick={() => void handleCreateApiKey()}
          >
            <KeyRound aria-hidden="true" size={16} />
            {isCreatingKey ? "Creating..." : "Create CI key"}
          </button>
        </div>
        <div className="table-list">
          {apiKeys.length === 0 ? (
            <p>No API keys yet.</p>
          ) : (
            apiKeys.map((apiKey) => (
              <article className="table-row" key={apiKey.id}>
                <strong>{apiKey.name}</strong>
                <span>{apiKey.prefix}...</span>
                <small>
                  {apiKey.revoked_at
                    ? "revoked"
                    : apiKey.last_used_at
                      ? `last used ${new Date(apiKey.last_used_at).toLocaleString()}`
                      : "not used yet"}
                </small>
                {!apiKey.revoked_at ? (
                  <button
                    aria-label={`Revoke ${apiKey.name}`}
                    className="secondary-button compact"
                    type="button"
                    onClick={() => void handleRevokeApiKey(apiKey.id)}
                  >
                    <Trash2 aria-hidden="true" size={15} />
                    Revoke
                  </button>
                ) : null}
              </article>
            ))
          )}
        </div>
      </section>
    </section>
  );

  async function handleCreateApiKey() {
    setIsCreatingKey(true);
    setError(null);
    try {
      const created = await createApiKey({
        name: apiKeyName.trim(),
        scopes: ["ci_eval_runs"],
      });
      setCreatedSecret(created.secret);
      setApiKeys((current) => [created.api_key, ...current]);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "API key failed");
    } finally {
      setIsCreatingKey(false);
    }
  }

  async function handleRevokeApiKey(apiKeyId: string) {
    setError(null);
    try {
      await revokeApiKey(apiKeyId);
      setApiKeys((current) =>
        current.map((apiKey) =>
          apiKey.id === apiKeyId
            ? { ...apiKey, revoked_at: new Date().toISOString() }
            : apiKey,
        ),
      );
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Revoke failed");
    }
  }
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
    <article className="panel config-panel">
      <div className="panel-heading">
        <h2>{title}</h2>
        <Icon aria-hidden="true" size={18} />
      </div>
      <dl>
        {items.map(([label, value]) => (
          <div key={label}>
            <dt>{label}</dt>
            <dd>{value}</dd>
          </div>
        ))}
      </dl>
    </article>
  );
}

function formatBytes(bytes: number) {
  if (bytes === 0) {
    return "--";
  }
  if (bytes < 1024 * 1024) {
    return `${Math.round(bytes / 1024)} KB`;
  }
  return `${Math.round(bytes / 1024 / 1024)} MB`;
}
