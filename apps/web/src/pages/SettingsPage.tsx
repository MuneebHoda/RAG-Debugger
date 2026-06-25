import { AlertCircle, Database, ServerCog, ShieldCheck } from "lucide-react";
import { useEffect, useState } from "react";

import { getProductConfig, type ProductConfig } from "../lib/api/config";

export function SettingsPage() {
  const [config, setConfig] = useState<ProductConfig | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    getProductConfig(controller.signal)
      .then(setConfig)
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
    </section>
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
