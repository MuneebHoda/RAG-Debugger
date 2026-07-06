import { ChevronRight } from "lucide-react";
import { Link, useLocation } from "react-router-dom";

import styles from "./WorkbenchShell.module.css";
import { resolveWorkbenchBreadcrumbs } from "./workbenchNavigation";

export function WorkbenchBreadcrumbs() {
  const location = useLocation();
  const breadcrumbs = resolveWorkbenchBreadcrumbs(
    location.pathname,
    location.search,
  );

  return (
    <nav className={styles.breadcrumbs} aria-label="Breadcrumb">
      <ol>
        {breadcrumbs.map((breadcrumb, index) => {
          const current = index === breadcrumbs.length - 1;
          return (
            <li key={`${breadcrumb.label}-${index}`}>
              {index > 0 ? <ChevronRight aria-hidden="true" size={13} /> : null}
              {breadcrumb.to && !current ? (
                <Link to={breadcrumb.to}>{breadcrumb.label}</Link>
              ) : (
                <span aria-current={current ? "page" : undefined}>
                  {breadcrumb.label}
                </span>
              )}
            </li>
          );
        })}
      </ol>
    </nav>
  );
}
