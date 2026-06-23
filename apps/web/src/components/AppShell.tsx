import { Activity, Database, FlaskConical, ShieldCheck } from "lucide-react";
import type { PropsWithChildren } from "react";
import { NavLink } from "react-router-dom";

const navItems = [
  { to: "/", label: "Traces", icon: Activity },
  { to: "/", label: "Sources", icon: Database },
  { to: "/", label: "Evals", icon: FlaskConical },
  { to: "/", label: "Privacy", icon: ShieldCheck },
];

export function AppShell({ children }: PropsWithChildren) {
  return (
    <div className="app-shell">
      <aside className="sidebar" aria-label="Primary navigation">
        <div className="brand">
          <span className="brand-mark">RD</span>
          <span>
            <strong>RAG Debugger</strong>
            <small>Retrieval diagnosis</small>
          </span>
        </div>
        <nav className="nav-list">
          {navItems.map((item) => (
            <NavLink key={item.label} to={item.to} className="nav-item">
              <item.icon aria-hidden="true" size={18} />
              <span>{item.label}</span>
            </NavLink>
          ))}
        </nav>
      </aside>
      <main className="main-content">{children}</main>
    </div>
  );
}
