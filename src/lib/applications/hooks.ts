import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { getApplications, createApplication, getApplicationById, deleteApplication } from "./server";
import { useRouter } from "@tanstack/react-router";

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

export function useDeleteApplication() {
  const queryClient = useQueryClient();
  const router = useRouter();

  return useMutation({
    mutationFn: (id: string) => deleteApplication({ data: { id } }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["applications"] });
      router.navigate({ to: "/applications" });
    },
  });
}
