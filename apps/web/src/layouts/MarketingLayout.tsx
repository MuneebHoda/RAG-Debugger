import { Menu } from "lucide-react";
import { Link, Outlet } from "react-router-dom";

import { CorpusLabLogo } from "../components/brand/CorpusLabLogo";
import { ButtonLink } from "../components/ui/Button";
import styles from "./MarketingLayout.module.css";

const navItems = [
  { to: "/features", label: "Features" },
  { to: "/pricing", label: "Pricing" },
  { to: "/login", label: "Login" },
];

export function MarketingLayout() {
  return (
    <div className={styles.layout}>
      <header className={styles.header}>
        <Link to="/" className={styles.brand} aria-label="CorpusLab home">
          <CorpusLabLogo />
        </Link>
        <nav className={styles.nav} aria-label="Public navigation">
          {navItems.map((item) => (
            <Link key={item.to} to={item.to}>
              {item.label}
            </Link>
          ))}
        </nav>
        <div className={styles.actions}>
          <ButtonLink to="/app" variant="ghost">
            Open app
          </ButtonLink>
          <ButtonLink to="/signup">Start free</ButtonLink>
        </div>
        <button className={styles.menuButton} type="button" aria-label="Menu">
          <Menu aria-hidden="true" size={20} />
        </button>
      </header>
      <Outlet />
      <footer className={styles.footer}>
        <CorpusLabLogo />
        <span>Evidence-first RAG operations for serious product teams.</span>
      </footer>
    </div>
  );
}
