import { AlertTriangle, CheckCircle2, GitBranch, Timer } from "lucide-react";

const stats = [
  { label: "Trace health", value: "Ready", icon: CheckCircle2 },
  { label: "Open failures", value: "0", icon: AlertTriangle },
  { label: "Eval branches", value: "0", icon: GitBranch },
  { label: "P95 latency", value: "--", icon: Timer },
];

export function DashboardPage() {
  return (
    <section className="dashboard" aria-labelledby="dashboard-title">
      <header className="page-header">
        <div>
          <p className="eyebrow">Hybrid local/cloud scaffold</p>
          <h1 id="dashboard-title">RAG Debugger</h1>
          <p>
            Diagnose retrieval failures across sources, chunks, traces, evals,
            and model configs.
          </p>
        </div>
      </header>

      <div className="stat-grid" aria-label="Project status">
        {stats.map((stat) => (
          <article className="stat-card" key={stat.label}>
            <stat.icon aria-hidden="true" size={20} />
            <span>{stat.label}</span>
            <strong>{stat.value}</strong>
          </article>
        ))}
      </div>

      <section className="workbench" aria-label="Debugger workbench">
        <div className="panel">
          <h2>Trace Timeline</h2>
          <p>
            Incoming RAG traces will show retrieval, reranking, generation, and
            eval spans here.
          </p>
        </div>
        <div className="panel">
          <h2>Failure Labels</h2>
          <p>
            Missing document, bad chunking, bad embedding, bad ranking, prompt
            drift, and hallucination.
          </p>
        </div>
      </section>
    </section>
  );
}
