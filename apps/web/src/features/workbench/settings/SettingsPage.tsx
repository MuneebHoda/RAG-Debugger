import {
  AlertCircle,
  KeyRound,
  ServerCog,
  ShieldCheck,
  Users,
  type LucideIcon,
} from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { useSearchParams } from "react-router-dom";

import { WorkbenchPageHeader } from "../../../components/workbench/WorkbenchPageHeader";
import { WorkbenchWorkflowGuide } from "../../../components/workbench/WorkbenchWorkflowGuide";
import { listApiKeys } from "../../../lib/api/apiKeys";
import { getCurrentUser } from "../../../lib/api/auth";
import { getProductConfig } from "../../../lib/api/config";
import { getCurrentProject } from "../../../lib/api/projects";
import { ApiKeysSettingsPanel } from "./components/ApiKeysSettingsPanel";
import {
  PrivacySettingsPanel,
  RuntimeSettingsPanel,
  WorkspaceSettingsPanel,
} from "./components/SettingsPanels";
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
  const projectQuery = useQuery({
    queryKey: ["current-project"],
    queryFn: ({ signal }) => getCurrentProject(signal),
    enabled: activeTab === "api-keys",
  });
  const error =
    configQuery.error ??
    userQuery.error ??
    keysQuery.error ??
    projectQuery.error;

  return (
    <section className={styles.page} aria-labelledby="settings-title">
      <WorkbenchPageHeader
        description="Manage workspace access, automation credentials, runtime configuration, and privacy posture."
        section="Admin"
        title="Settings"
        titleId="settings-title"
      />

      <WorkbenchWorkflowGuide
        currentStep="ci"
        impact="Workspace settings control who can operate CorpusLab and which automation can run Eval Lab gates safely."
        nextAction={
          activeTab === "api-keys"
            ? { label: "Create CI key", href: "#api-keys-title" }
            : { label: "Open API keys", to: "/app/settings?tab=api-keys" }
        }
        purpose="Settings is the admin step for workspace membership, CI credentials, runtime limits, and privacy posture."
      />

      <div
        className={styles.tabs}
        role="tablist"
        aria-label="Settings sections"
      >
        <SettingsTabButton
          active={activeTab === "workspace"}
          icon={Users}
          label="Workspace"
          onClick={() => setSearchParams({ tab: "workspace" })}
        />
        <SettingsTabButton
          active={activeTab === "api-keys"}
          icon={KeyRound}
          label="API keys"
          onClick={() => setSearchParams({ tab: "api-keys" })}
        />
        <SettingsTabButton
          active={activeTab === "runtime"}
          icon={ServerCog}
          label="Runtime"
          onClick={() => setSearchParams({ tab: "runtime" })}
        />
        <SettingsTabButton
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
        <WorkspaceSettingsPanel user={userQuery.data?.user} />
      ) : null}
      {activeTab === "api-keys" ? (
        <ApiKeysSettingsPanel
          apiKeys={keysQuery.data ?? []}
          project={projectQuery.data}
        />
      ) : null}
      {activeTab === "runtime" ? (
        <RuntimeSettingsPanel config={configQuery.data} />
      ) : null}
      {activeTab === "privacy" ? <PrivacySettingsPanel /> : null}
    </section>
  );
}

function SettingsTabButton({
  active,
  icon: Icon,
  label,
  onClick,
}: {
  active: boolean;
  icon: LucideIcon;
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
