import { Check, Copy, KeyRound, Trash2 } from "lucide-react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import {
  createApiKey,
  revokeApiKey,
  type ApiKey,
  type ApiKeyScope,
} from "../../../../lib/api/apiKeys";
import type { CurrentProject } from "../../../../lib/api/projects";
import { WorkbenchPanel } from "../../../../components/workbench/WorkbenchPanel";
import { formatDateTime } from "../../../../lib/dateTime";
import styles from "../SettingsPage.module.css";

export function ApiKeysSettingsPanel({
  apiKeys,
  project,
}: {
  apiKeys: ApiKey[];
  project?: CurrentProject;
}) {
  const queryClient = useQueryClient();
  const [name, setName] = useState("GitHub Actions");
  const [scope, setScope] = useState<ApiKeyScope>("ci_eval_runs");
  const [createdSecret, setCreatedSecret] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [copyError, setCopyError] = useState<string | null>(null);
  const createMutation = useMutation({
    mutationFn: () => createApiKey({ name: name.trim(), scopes: [scope] }),
    onSuccess: (created) => {
      setCreatedSecret(created.secret);
      setCopied(false);
      setCopyError(null);
      void queryClient.invalidateQueries({ queryKey: ["api-keys"] });
    },
  });
  const revokeMutation = useMutation({
    mutationFn: (apiKeyId: string) => revokeApiKey(apiKeyId),
    onSuccess: () =>
      void queryClient.invalidateQueries({ queryKey: ["api-keys"] }),
  });

  return (
    <WorkbenchPanel
      className={styles.panel}
      description="Create workspace-scoped credentials for CI gates or trace ingestion."
      icon={KeyRound}
      title="API keys"
      titleId="api-keys-title"
    >
      <div className={styles.keyGuidance}>
        <strong>GitHub Actions setup</strong>
        <p>
          Create a key for automated Eval Lab gates, then save the one-time
          secret in GitHub Actions as <code>CORPUSLAB_API_KEY</code>.
        </p>
        <p>
          CorpusLab stores only a one-way hash. The full secret cannot be shown
          again after you leave this page.
        </p>
      </div>

      {project ? (
        <div className={styles.keyGuidance}>
          <strong>Trace ingestion project</strong>
          <p>
            Use project ID <code>{project.id}</code> with the native contract or{" "}
            <code>x-corpuslab-project-id</code> OTLP header.
          </p>
          <p>
            Project privacy policy:{" "}
            {project.privacy_mode
              .replaceAll(/([a-z])([A-Z])/g, "$1 $2")
              .toLowerCase()}
            .
          </p>
        </div>
      ) : null}

      {createdSecret ? (
        <div
          aria-label="Created API key secret"
          aria-live="polite"
          className={styles.secretBox}
        >
          <span>This secret is shown once</span>
          <code>{createdSecret}</code>
          <button
            type="button"
            onClick={() => {
              setCopyError(null);
              if (typeof navigator.clipboard?.writeText !== "function") {
                setCopied(false);
                setCopyError(
                  "Clipboard access is unavailable. Copy the secret manually.",
                );
                return;
              }
              void navigator.clipboard
                .writeText(createdSecret)
                .then(() => setCopied(true))
                .catch(() => {
                  setCopied(false);
                  setCopyError(
                    "The API key secret could not be copied. Copy it manually.",
                  );
                });
            }}
          >
            {copied ? (
              <Check aria-hidden="true" size={15} />
            ) : (
              <Copy aria-hidden="true" size={15} />
            )}
            {copied ? "Copied" : "Copy"}
          </button>
          {copyError ? (
            <p className={styles.error} role="alert">
              {copyError}
            </p>
          ) : null}
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
        <label>
          Key purpose
          <select
            value={scope}
            onChange={(event) =>
              setScope(event.currentTarget.value as ApiKeyScope)
            }
          >
            <option value="ci_eval_runs">CI Eval Lab gates</option>
            <option value="trace_ingest">Trace ingestion</option>
          </select>
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

      {createMutation.isError || revokeMutation.isError ? (
        <p className={styles.error} role="alert">
          {createMutation.error instanceof Error
            ? createMutation.error.message
            : revokeMutation.error instanceof Error
              ? revokeMutation.error.message
              : "The API key change could not be saved."}
        </p>
      ) : null}

      <div className={styles.keyList}>
        {apiKeys.map((apiKey) => (
          <article className={styles.keyRow} key={apiKey.id}>
            <span>
              <strong>{apiKey.name}</strong>
              <small>
                {apiKey.prefix}… · scope{" "}
                {apiKey.scopes.join(", ").replaceAll("_", " ")}
              </small>
              <small>Created {formatDateTime(apiKey.created_at)}</small>
              <small>
                {apiKey.revoked_at
                  ? `Revoked ${formatDateTime(apiKey.revoked_at)}`
                  : apiKey.last_used_at
                    ? `Last used ${formatDateTime(apiKey.last_used_at)}`
                    : "Not used yet"}
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
    </WorkbenchPanel>
  );
}
