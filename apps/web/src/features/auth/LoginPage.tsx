import { ArrowRight, Building2, GitBranch } from "lucide-react";
import { FormEvent, useState } from "react";
import { Link } from "react-router-dom";
import { useLocation, useNavigate } from "react-router-dom";

import { Button } from "../../components/ui/Button";
import {
  authenticateDemoUser,
  createAuthSession,
  DEMO_CREDENTIALS,
} from "./authSession";
import styles from "./AuthPages.module.css";

export function LoginPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const [email, setEmail] = useState(DEMO_CREDENTIALS.email);
  const [password, setPassword] = useState(DEMO_CREDENTIALS.password);
  const [error, setError] = useState<string | null>(null);

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);

    if (!authenticateDemoUser(email, password)) {
      setError("Use the demo credentials shown below to open the workbench.");
      return;
    }

    createAuthSession(email);
    const from =
      (location.state as { from?: { pathname?: string } } | null)?.from
        ?.pathname ?? "/app";
    navigate(from, { replace: true });
  }

  return (
    <>
      <div className={styles.heading}>
        <p className={styles.eyebrow}>Welcome back</p>
        <h1>Sign in to CorpusLab.</h1>
        <p>
          Open your workbench, review retrieval runs, and continue improving
          corpus quality with your team.
        </p>
      </div>

      <div className={styles.credentialBox} aria-label="Demo credentials">
        <span>Demo credentials</span>
        <code>{DEMO_CREDENTIALS.email}</code>
        <code>{DEMO_CREDENTIALS.password}</code>
      </div>

      {error ? (
        <div className={styles.error} role="alert">
          {error}
        </div>
      ) : null}

      <form className={styles.form} onSubmit={handleSubmit}>
        <label>
          Email
          <input
            autoComplete="email"
            name="email"
            placeholder="you@company.com"
            required
            type="email"
            value={email}
            onChange={(event) => setEmail(event.currentTarget.value)}
          />
        </label>
        <label>
          Password
          <input
            autoComplete="current-password"
            name="password"
            placeholder="Enter password"
            required
            type="password"
            value={password}
            onChange={(event) => setPassword(event.currentTarget.value)}
          />
        </label>
        <div className={styles.row}>
          <label className={styles.check}>
            <input name="remember" type="checkbox" />
            Keep me signed in
          </label>
          <Link className={styles.link} to="/signup">
            Need access?
          </Link>
        </div>
        <div className={styles.submit}>
          <Button type="submit">
            Open workbench <ArrowRight aria-hidden="true" size={17} />
          </Button>
        </div>
      </form>

      <div className={styles.divider}>or continue with</div>
      <div className={styles.ssoGrid}>
        <button className={styles.ssoButton} type="button">
          <GitBranch aria-hidden="true" size={16} /> GitHub
        </button>
        <button className={styles.ssoButton} type="button">
          <Building2 aria-hidden="true" size={16} /> SSO
        </button>
      </div>
      <p className={styles.footer}>
        New to CorpusLab?{" "}
        <Link className={styles.link} to="/signup">
          Create a workspace
        </Link>
      </p>
    </>
  );
}
