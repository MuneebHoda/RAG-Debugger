import { ArrowRight, CheckCircle2 } from "lucide-react";
import { FormEvent, useState } from "react";
import { Link } from "react-router-dom";
import { useNavigate } from "react-router-dom";

import { Button } from "../../components/ui/Button";
import { createAuthSession } from "./authSession";
import styles from "./AuthPages.module.css";

const benefits = [
  "Create a private CorpusLab workspace",
  "Ingest documents and compare retrieval modes",
  "Run evals and export evidence reports",
];

export function SignupPage() {
  const navigate = useNavigate();
  const [email, setEmail] = useState("");
  const [workspaceName, setWorkspaceName] = useState("");
  const [password, setPassword] = useState("");

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    createAuthSession(email, workspaceName);
    navigate("/app", { replace: true });
  }

  return (
    <>
      <div className={styles.heading}>
        <p className={styles.eyebrow}>Start free</p>
        <h1>Create your CorpusLab workspace.</h1>
        <p>
          Bring a corpus, index evidence, and give your team a shared system for
          RAG quality.
        </p>
      </div>

      <form className={styles.form} onSubmit={handleSubmit}>
        <label>
          Work email
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
          Workspace name
          <input
            autoComplete="organization"
            name="workspace"
            placeholder="Acme AI Platform"
            required
            type="text"
            value={workspaceName}
            onChange={(event) => setWorkspaceName(event.currentTarget.value)}
          />
        </label>
        <label>
          Password
          <input
            autoComplete="new-password"
            name="password"
            placeholder="Create password"
            required
            type="password"
            value={password}
            onChange={(event) => setPassword(event.currentTarget.value)}
          />
        </label>
        <div className={styles.submit}>
          <Button type="submit">
            Create workspace <ArrowRight aria-hidden="true" size={17} />
          </Button>
        </div>
      </form>

      <ul className={styles.list}>
        {benefits.map((benefit) => (
          <li key={benefit}>
            <CheckCircle2 aria-hidden="true" size={16} />
            {benefit}
          </li>
        ))}
      </ul>
      <p className={styles.footer}>
        Already have an account?{" "}
        <Link className={styles.link} to="/login">
          Sign in
        </Link>
      </p>
    </>
  );
}
