import { Check, Copy, KeyRound, Trash2 } from "lucide-react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import {
  createApiKey,
  revokeApiKey,
  type ApiKey,
} from "../../../../lib/api/apiKeys";
import { formatDateTime } from "../../../../lib/dateTime";
import styles from "../SettingsPage.module.css";

export function ApiKeysSettingsPanel({ apiKeys }: { apiKeys: ApiKey[] }) {
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
    <section
      className={`panel ${styles.panel}`}
      aria-labelledby="api-keys-title"
    >
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
