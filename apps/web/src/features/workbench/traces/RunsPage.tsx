import {
  AlertTriangle,
  ArrowRight,
  GitBranch,
  Plus,
  Search,
} from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { Link } from "react-router-dom";

import { listTraces, type TraceSummary } from "../../../lib/api/traces";
import { formatDateTime } from "../../../lib/dateTime";
import styles from "./RunsPage.module.css";

type RunFilter = "all" | "attention" | "strong";

export function RunsPage() {
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState<RunFilter>("all");
  const runsQuery = useQuery({
    queryKey: ["traces"],
    queryFn: ({ signal }) => listTraces(signal),
  });
  const runs = useMemo(
    () => filterRuns(runsQuery.data ?? [], search, filter),
    [runsQuery.data, search, filter],
  );

  return (
    <section className={styles.page} aria-labelledby="runs-title">
      <header className={styles.header}>
        <div>
          <p>Improve</p>
          <h1 id="runs-title">Runs</h1>
          <span>Saved retrieval tests, diagnoses, and comparisons.</span>
        </div>
        <Link to="/app/retrieval">
          <Plus aria-hidden="true" size={16} /> New retrieval test
        </Link>
      </header>

      <div className={styles.toolbar}>
        <label className={styles.search}>
          <Search aria-hidden="true" size={16} />
          <input
            aria-label="Search runs"
            placeholder="Search questions"
            value={search}
            onChange={(event) => setSearch(event.currentTarget.value)}
          />
        </label>
        <select
          aria-label="Filter runs"
          value={filter}
          onChange={(event) =>
            setFilter(event.currentTarget.value as RunFilter)
          }
        >
          <option value="all">All runs</option>
          <option value="attention">Needs attention</option>
          <option value="strong">Strong evidence</option>
        </select>
      </div>

      {runsQuery.isLoading ? (
        <div className={styles.empty}>Loading runs…</div>
      ) : runsQuery.isError ? (
        <div className={styles.error} role="alert">
          <AlertTriangle aria-hidden="true" size={22} />
          <strong>Runs could not be loaded</strong>
          <button type="button" onClick={() => void runsQuery.refetch()}>
            Retry
          </button>
        </div>
      ) : runsQuery.data?.length === 0 ? (
        <div className={styles.empty}>
          <GitBranch aria-hidden="true" size={24} />
          <strong>No saved runs yet</strong>
          <span>Test a question, then choose Debug this run.</span>
          <Link to="/app/retrieval">Open Test retrieval</Link>
        </div>
      ) : (
        <RunTable runs={runs} />
      )}
    </section>
  );
}

function RunTable({ runs }: { runs: TraceSummary[] }) {
  if (runs.length === 0) {
    return (
      <div className={styles.empty}>
        <Search aria-hidden="true" size={22} />
        <strong>No runs match this view</strong>
      </div>
    );
  }

  return (
    <div className={styles.table} role="table" aria-label="Saved runs">
      <div className={styles.tableHeader} role="row">
        <span>Question</span>
        <span>Mode</span>
        <span>Evidence</span>
        <span>Latency</span>
        <span>Created</span>
        <span />
      </div>
      {runs.map((run) => (
        <Link className={styles.row} key={run.id} to={`/app/traces/${run.id}`}>
          <span className={styles.query}>
            <strong>{run.query}</strong>
            <small>
              {run.failure_labels.length === 0
                ? "No failure signals"
                : `${run.failure_labels.length} signals · ${run.rerun_count} comparisons`}
            </small>
          </span>
          <span className={styles.pill}>{run.retrieval_mode}</span>
          <span className={styles[run.evidence_strength]}>
            {run.evidence_strength}
          </span>
          <span className={styles.cell}>{run.latency_ms} ms</span>
          <span className={styles.cell}>{formatDateTime(run.created_at)}</span>
          <ArrowRight aria-hidden="true" size={16} />
        </Link>
      ))}
    </div>
  );
}

function filterRuns(runs: TraceSummary[], search: string, filter: RunFilter) {
  const normalizedSearch = search.trim().toLocaleLowerCase();
  return runs.filter((run) => {
    const matchesSearch =
      normalizedSearch.length === 0 ||
      run.query.toLocaleLowerCase().includes(normalizedSearch);
    const matchesFilter =
      filter === "all" ||
      (filter === "attention" &&
        (run.failure_labels.length > 0 || run.evidence_strength === "weak")) ||
      (filter === "strong" && run.evidence_strength === "strong");
    return matchesSearch && matchesFilter;
  });
}
