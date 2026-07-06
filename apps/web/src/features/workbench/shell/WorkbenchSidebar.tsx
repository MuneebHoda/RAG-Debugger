import { ShieldCheck, X } from "lucide-react";
import { forwardRef } from "react";
import { Link, useLocation } from "react-router-dom";

import { CorpusLabLogo } from "../../../components/brand/CorpusLabLogo";
import styles from "./WorkbenchShell.module.css";
import {
  isWorkbenchNavItemActive,
  WORKBENCH_NAV_GROUPS,
} from "./workbenchNavigation";

export const WorkbenchSidebar = forwardRef<
  HTMLElement,
  {
    isOpen: boolean;
    onClose: (restoreFocus?: boolean) => void;
    onNavigate: () => void;
  }
>(function WorkbenchSidebar({ isOpen, onClose, onNavigate }, ref) {
  const location = useLocation();

  return (
    <aside
      ref={ref}
      id="workbench-navigation"
      className={isOpen ? styles.sidebarOpen : styles.sidebar}
      aria-label="Workspace navigation"
    >
      <div className={styles.brandRow}>
        <CorpusLabLogo />
        <button
          aria-label="Close navigation"
          className={styles.closeNav}
          type="button"
          onClick={() => onClose(true)}
        >
          <X aria-hidden="true" size={19} />
        </button>
      </div>

      <nav className={styles.nav} aria-label="Product workflow">
        {WORKBENCH_NAV_GROUPS.map((group) => (
          <div className={styles.navGroup} key={group.label}>
            <span className={styles.navGroupLabel}>{group.label}</span>
            {group.items.map((item) => {
              const active = isWorkbenchNavItemActive(
                item.id,
                location.pathname,
                location.search,
              );
              return (
                <Link
                  aria-current={active ? "page" : undefined}
                  className={active ? styles.activeNavItem : styles.navItem}
                  key={item.id}
                  to={item.to}
                  onClick={onNavigate}
                >
                  <item.icon aria-hidden="true" size={18} />
                  <span>{item.label}</span>
                </Link>
              );
            })}
          </div>
        ))}
      </nav>

      <div className={styles.privacyNote}>
        <ShieldCheck aria-hidden="true" size={16} />
        <span>Private corpus controls active</span>
      </div>
    </aside>
  );
});
