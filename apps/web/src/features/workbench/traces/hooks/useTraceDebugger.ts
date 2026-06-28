import { useQuery } from "@tanstack/react-query";
import { useParams, useSearchParams } from "react-router-dom";

import { getTrace } from "../../../../lib/api/traces";

export const TRACE_TABS = [
  "summary",
  "evidence",
  "timeline",
  "compare",
] as const;

export type TraceTab = (typeof TRACE_TABS)[number];

export function useTraceDebugger() {
  const { traceId } = useParams<{ traceId: string }>();
  const [searchParams, setSearchParams] = useSearchParams();
  const tabParam = searchParams.get("tab");
  const activeTab: TraceTab = TRACE_TABS.includes(tabParam as TraceTab)
    ? (tabParam as TraceTab)
    : "summary";
  const traceQuery = useQuery({
    queryKey: ["trace", traceId],
    queryFn: ({ signal }) => getTrace(traceId!, signal),
    enabled: Boolean(traceId),
  });

  function selectTab(tab: TraceTab) {
    setSearchParams({ tab });
  }

  return { activeTab, selectTab, traceQuery };
}
