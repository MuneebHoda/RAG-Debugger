import { Menu, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { Link, Outlet, useLocation } from "react-router-dom";

import { CorpusLabLogo } from "../components/brand/CorpusLabLogo";
import { ButtonLink } from "../components/ui/Button";
import {
  type LandingHeaderState,
  useLandingHeaderState,
} from "../features/marketing/header/useLandingHeaderState";
import styles from "./MarketingLayout.module.css";

const navItems = [
  { to: "/features", label: "Features" },
  { to: "/pricing", label: "Pricing" },
  { to: "/login", label: "Login" },
];

const landingHeaderClasses: Record<LandingHeaderState, string> = {
  hero: styles.landingHeaderHero,
  dark: styles.landingHeaderDark,
  light: styles.landingHeaderLight,
};

export function MarketingLayout() {
  const location = useLocation();
  const isLanding = location.pathname === "/";
  const landingHeaderState = useLandingHeaderState(isLanding, location.key);
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const menuButtonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!mobileMenuOpen) return undefined;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      setMobileMenuOpen(false);
      menuButtonRef.current?.focus();
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [mobileMenuOpen]);

  return (
    <div className={isLanding ? styles.landingLayout : styles.layout}>
      <header
        className={
          isLanding
            ? `${styles.landingHeader} ${landingHeaderClasses[landingHeaderState]}`
            : styles.header
        }
        data-landing-header-state={isLanding ? landingHeaderState : undefined}
      >
        <Link to="/" className={styles.brand} aria-label="CorpusLab home">
          <CorpusLabLogo
            tone={
              isLanding && landingHeaderState !== "light" ? "light" : "dark"
            }
          />
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
        <button
          aria-controls="marketing-mobile-nav"
          aria-expanded={mobileMenuOpen}
          aria-label={mobileMenuOpen ? "Close menu" : "Open menu"}
          className={styles.menuButton}
          ref={menuButtonRef}
          type="button"
          onClick={() => setMobileMenuOpen((current) => !current)}
        >
          {mobileMenuOpen ? (
            <X aria-hidden="true" size={20} />
          ) : (
            <Menu aria-hidden="true" size={20} />
          )}
        </button>
        <nav
          aria-label="Mobile public navigation"
          className={mobileMenuOpen ? styles.mobileNavOpen : styles.mobileNav}
          id="marketing-mobile-nav"
        >
          {navItems.map((item) => (
            <Link
              key={item.to}
              to={item.to}
              onClick={() => setMobileMenuOpen(false)}
            >
              {item.label}
            </Link>
          ))}
          <Link to="/app" onClick={() => setMobileMenuOpen(false)}>
            Open app
          </Link>
          <Link
            className={styles.mobileCta}
            to="/signup"
            onClick={() => setMobileMenuOpen(false)}
          >
            Start free
          </Link>
        </nav>
      </header>
      <Outlet />
      <footer
        className={styles.footer}
        data-landing-header-tone={isLanding ? "light" : undefined}
      >
        <CorpusLabLogo />
        <span>Evidence-first RAG operations for serious product teams.</span>
      </footer>
    </div>
  );
}
