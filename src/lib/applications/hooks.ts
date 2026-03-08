import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { getApplications, createApplication, getApplicationById } from "./server";

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

export function useApplication(id: string) {
  return useQuery({
    queryKey: ["applications", id],
    queryFn: () => getApplicationById({ data: { id } }),
  });
}
