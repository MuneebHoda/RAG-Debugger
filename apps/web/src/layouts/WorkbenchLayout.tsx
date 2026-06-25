import {
  Activity,
  Database,
  FileBarChart,
  FlaskConical,
  GitBranch,
  LayoutDashboard,
  LogOut,
  PlayCircle,
  Search,
  Settings,
  ShieldCheck,
  UploadCloud,
} from "lucide-react";
import { useEffect, useState } from "react";
import { Link, NavLink, Outlet, useNavigate } from "react-router-dom";

import { CorpusLabLogo } from "../components/brand/CorpusLabLogo";
import {
  clearAuthSession,
  readAuthSession,
  type AuthSession,
} from "../features/auth/authSession";
import { getProductConfig, type ProductConfig } from "../lib/api/config";
import { getHealth } from "../lib/api/health";
import { getOverview, type OverviewResponse } from "../lib/api/overview";
import styles from "./WorkbenchLayout.module.css";

const navItems = [
  { to: "/app", label: "Overview", icon: LayoutDashboard, end: true },
  { to: "/app/sources", label: "Sources", icon: Database },
  { to: "/app/retrieval", label: "Retrieval", icon: Search },
  { to: "/app/traces", label: "Traces", icon: GitBranch },
  { to: "/app/evals", label: "Evals", icon: FlaskConical },
  { to: "/app/reports", label: "Reports", icon: FileBarChart },
  { to: "/app/settings", label: "Settings", icon: Settings },
];

export function WorkbenchLayout() {
  const navigate = useNavigate();
  const [config, setConfig] = useState<ProductConfig | null>(null);
  const [session, setSession] = useState<AuthSession | null>(() =>
    readAuthSession(),
  );
  const [apiStatus, setApiStatus] = useState<"checking" | "online" | "offline">(
    "checking",
  );
  const [overview, setOverview] = useState<OverviewResponse | null>(null);

  useEffect(() => {
    const controller = new AbortController();

    Promise.allSettled([
      getProductConfig(controller.signal),
      getHealth(controller.signal),
      getOverview(controller.signal),
    ]).then(([configResult, healthResult, overviewResult]) => {
      if (controller.signal.aborted) {
        return;
      }

      if (configResult.status === "fulfilled") {
        setConfig(configResult.value);
      }
      setApiStatus(healthResult.status === "fulfilled" ? "online" : "offline");
      if (overviewResult.status === "fulfilled") {
        setOverview(overviewResult.value);
      }
    });

    return () => controller.abort();
  }, []);

  useEffect(() => {
    const handleAuthChange = () => setSession(readAuthSession());
    window.addEventListener("corpuslab-auth-change", handleAuthChange);
    return () =>
      window.removeEventListener("corpuslab-auth-change", handleAuthChange);
  }, []);

  function handleLogout() {
    clearAuthSession();
    navigate("/login", { replace: true });
  }

  return (
    <div className={styles.shell}>
      <aside className={styles.sidebar} aria-label="Workspace navigation">
        <CorpusLabLogo />
        <nav className={styles.nav}>
          {navItems.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              end={item.end}
              className={({ isActive }) =>
                isActive ? styles.activeNavItem : styles.navItem
              }
            >
              <item.icon aria-hidden="true" size={18} />
              <span>{item.label}</span>
            </NavLink>
          ))}
        </nav>
        <div className={styles.privacyNote}>
          <ShieldCheck aria-hidden="true" size={16} />
          <span>Private corpus controls are active.</span>
        </div>
      </aside>
      <main className={styles.main}>
        <div className={styles.topbar} aria-label="Workspace status">
          <div className={styles.statusCluster}>
            <span className={styles.statusPill}>
              <Activity aria-hidden="true" size={16} /> API {apiStatus}
            </span>
            <span className={styles.statusPill}>
              <Database aria-hidden="true" size={16} />{" "}
              {config?.product.workspace_name ?? "Corpus Workspace"}
            </span>
            <span className={styles.statusPill}>
              <Search aria-hidden="true" size={16} />{" "}
              {config?.retrieval.default_mode ?? "hybrid"} retrieval
            </span>
            {overview ? (
              <>
                <span className={styles.statusPill}>
                  <ShieldCheck aria-hidden="true" size={16} />{" "}
                  {statusLabel(overview.health.status)} ·{" "}
                  {overview.health.score}/100
                </span>
                <span className={styles.statusPill}>
                  <SparklineIcon /> {overview.embedding_status.indexed_chunks}/
                  {overview.embedding_status.total_chunks} embedded
                </span>
              </>
            ) : null}
            <span className={styles.statusPill}>
              <ShieldCheck aria-hidden="true" size={16} />{" "}
              {session?.email ?? "demo session"}
            </span>
          </div>
          <div className={styles.quickActions} aria-label="Quick actions">
            <Link to="/app/sources">
              <UploadCloud aria-hidden="true" size={15} />
              Ingest documents
            </Link>
            <Link to="/app/retrieval">
              <PlayCircle aria-hidden="true" size={15} />
              Run retrieval
            </Link>
            <Link to="/app/traces">
              <GitBranch aria-hidden="true" size={15} />
              Open traces
            </Link>
            <Link to="/app/evals">
              <FlaskConical aria-hidden="true" size={15} />
              Run evals
            </Link>
            <button
              className={styles.logoutButton}
              type="button"
              onClick={handleLogout}
            >
              <LogOut aria-hidden="true" size={15} />
              Logout
            </button>
          </div>
        </div>
        <Outlet />
      </main>
    </div>
  );
}

function statusLabel(status: OverviewResponse["health"]["status"]) {
  return status
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function SparklineIcon() {
  return <span className={styles.sparklineIcon} aria-hidden="true" />;
}
