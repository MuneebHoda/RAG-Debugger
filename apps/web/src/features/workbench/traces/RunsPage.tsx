import { AlertTriangle, GitBranch, Plus, Search } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { Link } from "react-router-dom";

import { listTraces } from "../../../lib/api/traces";
import { TraceList } from "./components/TraceList";
import styles from "./RunsPage.module.css";
import { filterRuns, type RunFilter } from "./utils/runFilters";

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
        <TraceList runs={runs} />
      )}
    </section>
  );
}
