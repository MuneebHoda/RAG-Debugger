import { AlertTriangle, GitBranch, Plus, Search } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { Link } from "react-router-dom";

import { listTraces } from "../../../lib/api/traces";
import { WorkbenchEmptyState } from "../../../components/workbench/WorkbenchEmptyState";
import { WorkbenchPageHeader } from "../../../components/workbench/WorkbenchPageHeader";
import { WorkbenchToolbar } from "../../../components/workbench/WorkbenchToolbar";
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
      <WorkbenchPageHeader
        actions={
          <Link to="/app/retrieval">
            <Plus aria-hidden="true" size={16} /> New retrieval test
          </Link>
        }
        description="Inspect saved retrieval runs, deterministic diagnoses, evidence, and rerun comparisons."
        section="Debug"
        title="Trace Debugger"
        titleId="runs-title"
      />

      <WorkbenchToolbar className={styles.toolbar} label="Run filters">
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
      </WorkbenchToolbar>

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
        <WorkbenchEmptyState
          description="Run a diagnostic question, inspect its evidence, then choose Debug this run to preserve the diagnosis."
          icon={GitBranch}
          primaryAction={{ label: "Test retrieval", to: "/app/retrieval" }}
          secondaryAction={{ label: "Continue guided demo", to: "/app" }}
          title="No saved retrieval runs"
        />
      ) : (
        <TraceList runs={runs} />
      )}
    </section>
  );
}
