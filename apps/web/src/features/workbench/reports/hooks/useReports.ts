import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  createDebugReportFromExperiment,
  createDebugReportFromTrace,
  exportDebugReportMarkdown,
  getDebugReport,
  listDebugReports,
  type DebugReportPrivacyMode,
} from "../../../../lib/api/reports";

export const reportKeys = {
  all: ["debug-reports"] as const,
  detail: (reportId: string) => ["debug-reports", reportId] as const,
};

export type CreateReportInput =
  | {
      sourceType: "trace";
      sourceId: string;
      privacyMode: DebugReportPrivacyMode;
    }
  | {
      sourceType: "experiment";
      sourceId: string;
      privacyMode: DebugReportPrivacyMode;
    };

export function useDebugReports() {
  return useQuery({
    queryKey: reportKeys.all,
    queryFn: ({ signal }) => listDebugReports(signal),
  });
}

export function useDebugReport(reportId: string | undefined) {
  return useQuery({
    queryKey: reportKeys.detail(reportId ?? "missing"),
    queryFn: ({ signal }) => getDebugReport(reportId!, signal),
    enabled: Boolean(reportId),
  });
}

export function useCreateDebugReport() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateReportInput) => {
      const request = { privacy_mode: input.privacyMode };
      return input.sourceType === "trace"
        ? createDebugReportFromTrace(input.sourceId, request)
        : createDebugReportFromExperiment(input.sourceId, request);
    },
    onSuccess: (report) => {
      queryClient.setQueryData(reportKeys.detail(report.id), report);
      void queryClient.invalidateQueries({ queryKey: reportKeys.all });
    },
  });
}

export function useCopyReportMarkdown() {
  return useMutation({
    mutationFn: async (reportId: string) => {
      const markdown = await exportDebugReportMarkdown(reportId);
      await navigator.clipboard.writeText(markdown);
      return markdown;
    },
  });
}
