import {
  Activity,
  ChevronDown,
  Database,
  FileBarChart,
  FlaskConical,
  GitBranch,
  HelpCircle,
  Home,
  LogOut,
  Menu,
  Search,
  Settings,
  ShieldCheck,
  UserRound,
  X,
} from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import {
  Link,
  NavLink,
  Outlet,
  useLocation,
  useNavigate,
} from "react-router-dom";

import { CorpusLabLogo } from "../components/brand/CorpusLabLogo";
import { RouteErrorBoundary } from "../components/workbench/RouteErrorBoundary";
import {
  clearAuthSession,
  readAuthSession,
  type AuthSession,
} from "../features/auth/authSession";
import { logout } from "../lib/api/auth";
import { getProductConfig } from "../lib/api/config";
import { getHealth } from "../lib/api/health";
import { getOverview } from "../lib/api/overview";
import styles from "./WorkbenchLayout.module.css";

const navGroups = [
  {
    label: "Build",
    items: [
      { to: "/app/sources", label: "Corpus", icon: Database },
      { to: "/app/retrieval", label: "Test retrieval", icon: Search },
    ],
  },
  {
    label: "Improve",
    items: [
      { to: "/app/traces", label: "Runs", icon: GitBranch },
      { to: "/app/evals", label: "Quality", icon: FlaskConical },
    ],
  },
  {
    label: "Share",
    items: [{ to: "/app/reports", label: "Reports", icon: FileBarChart }],
  },
  {
    label: "Workspace",
    items: [{ to: "/app/settings", label: "Settings", icon: Settings }],
  },
];

const breadcrumbLabels: Record<string, string> = {
  sources: "Corpus",
  retrieval: "Test retrieval",
  traces: "Runs",
  evals: "Quality",
  reports: "Reports",
  settings: "Settings",
};

export function WorkbenchLayout() {
  const location = useLocation();
  const navigate = useNavigate();
  const [mobileNavOpen, setMobileNavOpen] = useState(false);
  const [session, setSession] = useState<AuthSession | null>(() =>
    readAuthSession(),
  );
  const configQuery = useQuery({
    queryKey: ["product-config"],
    queryFn: ({ signal }) => getProductConfig(signal),
  });
  const healthQuery = useQuery({
    queryKey: ["health"],
    queryFn: ({ signal }) => getHealth(signal),
    refetchInterval: 30_000,
  });
  const overviewQuery = useQuery({
    queryKey: ["overview"],
    queryFn: ({ signal }) => getOverview(signal),
  });

  useEffect(() => {
    const handleAuthChange = () => setSession(readAuthSession());
    window.addEventListener("corpuslab-auth-change", handleAuthChange);
    return () =>
      window.removeEventListener("corpuslab-auth-change", handleAuthChange);
  }, []);

  function handleLogout() {
    void logout().catch(() => undefined);
    clearAuthSession();
    navigate("/login", { replace: true });
  }

  const overview = overviewQuery.data;
  const healthLabel = overview
    ? `${statusLabel(overview.health.status)} · ${overview.health.score}`
    : healthQuery.isSuccess
      ? "Systems online"
      : healthQuery.isError
        ? "API unavailable"
        : "Checking systems";

  return (
    <div className={styles.shell}>
      <button
        aria-label="Close navigation"
        className={mobileNavOpen ? styles.scrimVisible : styles.scrim}
        type="button"
        onClick={() => setMobileNavOpen(false)}
      />
      <aside
        className={mobileNavOpen ? styles.sidebarOpen : styles.sidebar}
        aria-label="Workspace navigation"
      >
        <div className={styles.brandRow}>
          <CorpusLabLogo />
          <button
            aria-label="Close navigation"
            className={styles.closeNav}
            type="button"
            onClick={() => setMobileNavOpen(false)}
          >
            <X aria-hidden="true" size={19} />
          </button>
        </div>

        <nav className={styles.nav} onClick={() => setMobileNavOpen(false)}>
          <NavLink
            end
            to="/app"
            className={({ isActive }) =>
              isActive ? styles.activeNavItem : styles.navItem
            }
          >
            <Home aria-hidden="true" size={18} />
            <span>Home</span>
          </NavLink>

          {navGroups.map((group) => (
            <div className={styles.navGroup} key={group.label}>
              <span className={styles.navGroupLabel}>{group.label}</span>
              {group.items.map((item) => (
                <NavLink
                  key={item.to}
                  to={item.to}
                  className={({ isActive }) =>
                    isActive ? styles.activeNavItem : styles.navItem
                  }
                >
                  <item.icon aria-hidden="true" size={18} />
                  <span>{item.label}</span>
                </NavLink>
              ))}
            </div>
          ))}
        </nav>

        <div className={styles.privacyNote}>
          <ShieldCheck aria-hidden="true" size={16} />
          <span>Private corpus controls active</span>
        </div>
      </aside>

      <div className={styles.workspace}>
        <header className={styles.topbar} aria-label="Workspace header">
          <div className={styles.topbarStart}>
            <button
              aria-label="Open navigation"
              className={styles.menuButton}
              type="button"
              onClick={() => setMobileNavOpen(true)}
            >
              <Menu aria-hidden="true" size={20} />
            </button>
            <details className={styles.menu}>
              <summary className={styles.workspacePicker}>
                <Database aria-hidden="true" size={16} />
                <span>
                  {configQuery.data?.product.workspace_name ??
                    "Corpus Workspace"}
                </span>
                <ChevronDown aria-hidden="true" size={15} />
              </summary>
              <div className={styles.menuPanel}>
                <strong>Current workspace</strong>
                <span>
                  {configQuery.data?.product.workspace_name ??
                    "Corpus Workspace"}
                </span>
                <Link to="/app/settings">Manage workspace</Link>
              </div>
            </details>
            <span className={styles.breadcrumb} aria-label="Current page">
              {breadcrumbFor(location.pathname)}
            </span>
          </div>

          <div className={styles.topbarEnd}>
            <Link className={styles.healthStatus} to="/app">
              <Activity aria-hidden="true" size={16} />
              <span>{healthLabel}</span>
            </Link>
            <details className={styles.menu}>
              <summary className={styles.iconButton} aria-label="Open help">
                <HelpCircle aria-hidden="true" size={18} />
              </summary>
              <div className={`${styles.menuPanel} ${styles.helpPanel}`}>
                <strong>CorpusLab workflow</strong>
                <span>Corpus → Test → Runs → Quality → Reports</span>
                <Link to="/app">Open guided setup</Link>
              </div>
            </details>
            <details className={styles.menu}>
              <summary
                className={styles.userButton}
                aria-label="Open user menu"
              >
                <span className={styles.avatar}>
                  <UserRound aria-hidden="true" size={16} />
                </span>
                <span className={styles.userEmail}>
                  {session?.email ?? "Account"}
                </span>
                <ChevronDown aria-hidden="true" size={15} />
              </summary>
              <div className={styles.menuPanel}>
                <strong>{session?.email ?? "CorpusLab account"}</strong>
                <Link to="/app/settings">
                  <Settings aria-hidden="true" size={14} /> Settings
                </Link>
                <button type="button" onClick={handleLogout}>
                  <LogOut aria-hidden="true" size={14} /> Sign out
                </button>
              </div>
            </details>
          </div>
        </header>

        <main className={styles.main}>
          <RouteErrorBoundary>
            <Outlet />
          </RouteErrorBoundary>
        </main>
      </div>
    </div>
  );
}

function breadcrumbFor(pathname: string) {
  const segment = pathname.split("/").filter(Boolean)[1];
  return segment ? (breadcrumbLabels[segment] ?? "Detail") : "Home";
}

function statusLabel(
  status:
    | "ready"
    | "needs_indexing"
    | "needs_eval_coverage"
    | "needs_documents",
) {
  return {
    ready: "Ready",
    needs_indexing: "Needs indexing",
    needs_eval_coverage: "Needs quality checks",
    needs_documents: "Needs documents",
  }[status];
}
