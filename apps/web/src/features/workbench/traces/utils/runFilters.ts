import type { TraceSummary } from "../../../../lib/api/traces";

export type RunFilter = "all" | "attention" | "strong";

export function filterRuns(
  runs: TraceSummary[],
  search: string,
  filter: RunFilter,
) {
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
