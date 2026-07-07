import { Database, ServerCog, ShieldCheck, Users } from "lucide-react";

import { WorkbenchPanel } from "../../../../components/workbench/WorkbenchPanel";
import { getCurrentUser } from "../../../../lib/api/auth";
import { getProductConfig } from "../../../../lib/api/config";
import styles from "../SettingsPage.module.css";

export function WorkspaceSettingsPanel({
  user,
}: {
  user: Awaited<ReturnType<typeof getCurrentUser>>["user"] | undefined;
}) {
  return (
    <WorkbenchPanel
      className={styles.panel}
      description="Your active team and access level."
      icon={Users}
      title="Workspace"
    >
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
    </WorkbenchPanel>
  );
}

export function RuntimeSettingsPanel({
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

export function PrivacySettingsPanel() {
  return (
    <WorkbenchPanel
      className={styles.panel}
      description="What CorpusLab stores and what remains outside the system."
      icon={ShieldCheck}
      title="Privacy posture"
    >
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
    </WorkbenchPanel>
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
    <WorkbenchPanel className={styles.panel} icon={Icon} title={title}>
      <dl className={styles.definitionList}>
        {items.map(([label, value]) => (
          <Definition key={label} label={label} value={value} />
        ))}
      </dl>
    </WorkbenchPanel>
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
