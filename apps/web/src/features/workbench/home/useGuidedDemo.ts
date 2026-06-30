import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { getDemoStatus, loadDemo } from "../../../lib/api/demo";
import { indexEmbeddings } from "../../../lib/api/embeddings";

export const demoKeys = {
  status: ["guided-demo"] as const,
};

export function useGuidedDemo() {
  const queryClient = useQueryClient();
  const statusQuery = useQuery({
    queryKey: demoKeys.status,
    queryFn: ({ signal }) => getDemoStatus(signal),
  });
  const loadMutation = useMutation({
    mutationFn: loadDemo,
    onSuccess: (response) => {
      queryClient.setQueryData(demoKeys.status, response.status);
      invalidateWorkspace(queryClient);
    },
  });
  const indexMutation = useMutation({
    mutationFn: async () => {
      const sourceId = statusQuery.data?.source_id;
      if (!sourceId) throw new Error("Load the sample corpus before indexing.");
      return indexEmbeddings({ source_ids: [sourceId] });
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: demoKeys.status });
      invalidateWorkspace(queryClient);
    },
  });

  return { statusQuery, loadMutation, indexMutation };
}

function invalidateWorkspace(queryClient: ReturnType<typeof useQueryClient>) {
  void queryClient.invalidateQueries({ queryKey: ["overview"] });
  void queryClient.invalidateQueries({ queryKey: ["sources"] });
}
