import {
  Activity,
  ChevronDown,
  Database,
  HelpCircle,
  LogOut,
  Menu,
  Settings,
  UserRound,
} from "lucide-react";
import type { RefObject } from "react";
import { Link } from "react-router-dom";

import styles from "./WorkbenchShell.module.css";
import { WorkbenchBreadcrumbs } from "./WorkbenchBreadcrumbs";
import { WORKBENCH_FLOW_LABELS } from "./workbenchNavigation";

export function WorkbenchTopbar({
  email,
  healthLabel,
  menuButtonRef,
  mobileNavOpen,
  workspaceName,
  onLogout,
  onOpenNavigation,
}: {
  email: string;
  healthLabel: string;
  menuButtonRef: RefObject<HTMLButtonElement | null>;
  mobileNavOpen: boolean;
  workspaceName: string;
  onLogout: () => void;
  onOpenNavigation: () => void;
}) {
  return (
    <header className={styles.topbar} aria-label="Workspace header">
      <div className={styles.topbarStart}>
        <button
          aria-controls="workbench-navigation"
          aria-expanded={mobileNavOpen}
          aria-label="Open navigation"
          className={styles.menuButton}
          ref={menuButtonRef}
          type="button"
          onClick={onOpenNavigation}
        >
          <Menu aria-hidden="true" size={20} />
        </button>
        <details className={styles.menu}>
          <summary className={styles.workspacePicker}>
            <Database aria-hidden="true" size={16} />
            <span>{workspaceName}</span>
            <ChevronDown aria-hidden="true" size={15} />
          </summary>
          <div className={styles.menuPanel}>
            <strong>Current workspace</strong>
            <span>{workspaceName}</span>
            <Link to="/app/settings">Manage workspace</Link>
          </div>
        </details>
        <WorkbenchBreadcrumbs />
      </div>

      <div className={styles.topbarEnd}>
        <Link
          aria-label={`Workspace health: ${healthLabel}`}
          className={styles.healthStatus}
          to="/app"
        >
          <Activity aria-hidden="true" size={16} />
          <span>{healthLabel}</span>
        </Link>
        <details className={styles.menu}>
          <summary className={styles.iconButton} aria-label="Open help">
            <HelpCircle aria-hidden="true" size={18} />
          </summary>
          <div className={`${styles.menuPanel} ${styles.helpPanel}`}>
            <strong>CorpusLab workflow</strong>
            <span>{WORKBENCH_FLOW_LABELS.join(" → ")}</span>
            <Link to="/app">Open guided setup</Link>
          </div>
        </details>
        <details className={styles.menu}>
          <summary className={styles.userButton} aria-label="Open user menu">
            <span className={styles.avatar}>
              <UserRound aria-hidden="true" size={16} />
            </span>
            <span className={styles.userEmail}>{email}</span>
            <ChevronDown aria-hidden="true" size={15} />
          </summary>
          <div className={styles.menuPanel}>
            <strong>{email}</strong>
            <Link to="/app/settings">
              <Settings aria-hidden="true" size={14} /> Settings
            </Link>
            <button type="button" onClick={onLogout}>
              <LogOut aria-hidden="true" size={14} /> Sign out
            </button>
          </div>
        </details>
      </div>
    </header>
  );
}
