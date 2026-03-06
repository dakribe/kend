import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { getApplications, createApplication } from "./server";

export function useApplications() {
  return useQuery({
    queryKey: ["applications"],
    queryFn: getApplications,
  });
}

export function useCreateApplication() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: createApplication,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["applications"] });
    },
  });
}
