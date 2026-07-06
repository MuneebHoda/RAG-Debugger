import { useQuery } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { Outlet, useNavigate } from "react-router-dom";

import { RouteErrorBoundary } from "../components/workbench/RouteErrorBoundary";
import {
  clearAuthSession,
  readAuthSession,
  type AuthSession,
} from "../features/auth/authSession";
import { WorkbenchSidebar } from "../features/workbench/shell/WorkbenchSidebar";
import styles from "../features/workbench/shell/WorkbenchShell.module.css";
import { WorkbenchTopbar } from "../features/workbench/shell/WorkbenchTopbar";
import "../features/workbench/workbench.css";
import { logout } from "../lib/api/auth";
import { getProductConfig } from "../lib/api/config";
import { getHealth } from "../lib/api/health";
import { getOverview } from "../lib/api/overview";

export function WorkbenchLayout() {
  const navigate = useNavigate();
  const [mobileNavOpen, setMobileNavOpen] = useState(false);
  const menuButtonRef = useRef<HTMLButtonElement>(null);
  const navigationRef = useRef<HTMLElement>(null);
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

  useEffect(() => {
    if (!mobileNavOpen) return undefined;

    const previousOverflow = document.body.style.overflow;
    const frame = window.requestAnimationFrame(() => {
      const activeLink =
        navigationRef.current?.querySelector<HTMLElement>(
          'a[aria-current="page"]',
        ) ?? navigationRef.current?.querySelector<HTMLElement>("a");
      activeLink?.focus();
    });
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      setMobileNavOpen(false);
      menuButtonRef.current?.focus();
    };

    document.body.style.overflow = "hidden";
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.cancelAnimationFrame(frame);
      document.body.style.overflow = previousOverflow;
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [mobileNavOpen]);

  function closeNavigation(restoreFocus = false) {
    setMobileNavOpen(false);
    if (restoreFocus) menuButtonRef.current?.focus();
  }

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
    <div className={styles.shell} data-workbench-shell>
      <button
        aria-label="Close navigation"
        className={mobileNavOpen ? styles.scrimVisible : styles.scrim}
        type="button"
        onClick={() => closeNavigation(true)}
      />
      <WorkbenchSidebar
        ref={navigationRef}
        isOpen={mobileNavOpen}
        onClose={closeNavigation}
        onNavigate={() => closeNavigation()}
      />

      <div className={styles.workspace}>
        <WorkbenchTopbar
          email={session?.email ?? "CorpusLab account"}
          healthLabel={healthLabel}
          menuButtonRef={menuButtonRef}
          mobileNavOpen={mobileNavOpen}
          workspaceName={
            configQuery.data?.product.workspace_name ?? "Corpus Workspace"
          }
          onLogout={handleLogout}
          onOpenNavigation={() => setMobileNavOpen(true)}
        />

        <main className={styles.main}>
          <RouteErrorBoundary>
            <Outlet />
          </RouteErrorBoundary>
        </main>
      </div>
    </div>
  );
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
