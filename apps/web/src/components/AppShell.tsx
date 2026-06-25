import {
  Activity,
  Database,
  FileBarChart,
  FlaskConical,
  LayoutDashboard,
  Search,
  Settings,
  ShieldCheck,
} from "lucide-react";
import type { PropsWithChildren } from "react";
import { useEffect, useState } from "react";
import { NavLink } from "react-router-dom";

import {
  getHealth,
  getProductConfig,
  type ProductConfig,
} from "../lib/apiClient";

const navItems = [
  { to: "/", label: "Overview", icon: LayoutDashboard },
  { to: "/sources", label: "Sources", icon: Database },
  { to: "/retrieval", label: "Retrieval", icon: Search },
  { to: "/evals", label: "Evals", icon: FlaskConical },
  { to: "/reports", label: "Reports", icon: FileBarChart },
  { to: "/settings", label: "Settings", icon: Settings },
];

export function AppShell({ children }: PropsWithChildren) {
  const [config, setConfig] = useState<ProductConfig | null>(null);
  const [apiStatus, setApiStatus] = useState<"checking" | "online" | "offline">(
    "checking",
  );

  useEffect(() => {
    const controller = new AbortController();
    Promise.all([
      getProductConfig(controller.signal).then(setConfig),
      getHealth(controller.signal).then(() => setApiStatus("online")),
    ]).catch(() => {
      if (!controller.signal.aborted) {
        setApiStatus("offline");
      }
    });

    return () => controller.abort();
  }, []);

  return (
    <div className="app-shell">
      <aside className="sidebar" aria-label="Primary navigation">
        <div className="brand">
          <span className="brand-mark">RD</span>
          <span>
            <strong>{config?.product.name ?? "CorpusLab"}</strong>
            <small>
              {config?.product.workspace_name ?? "Corpus Workspace"}
            </small>
          </span>
        </div>
        <nav className="nav-list">
          {navItems.map((item) => (
            <NavLink
              key={item.label}
              to={item.to}
              className={({ isActive }) =>
                isActive ? "nav-item active" : "nav-item"
              }
            >
              <item.icon aria-hidden="true" size={18} />
              <span>{item.label}</span>
            </NavLink>
          ))}
        </nav>
        <div className="privacy-note">
          <ShieldCheck aria-hidden="true" size={16} />
          <span>Privacy-first corpus debugging</span>
        </div>
      </aside>
      <main className="main-content">
        <div className="top-status-bar" aria-label="Workspace status">
          <div>
            <Activity aria-hidden="true" size={16} />
            <span>API {apiStatus}</span>
          </div>
          <div>
            <Database aria-hidden="true" size={16} />
            <span>
              {config?.product.deployment_mode ?? "hybrid"} deployment
            </span>
          </div>
          <div>
            <Search aria-hidden="true" size={16} />
            <span>{config?.retrieval.default_mode ?? "hybrid"} retrieval</span>
          </div>
        </div>
        {children}
      </main>
    </div>
  );
}
