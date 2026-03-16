import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { getApplications, createApplication, getApplicationById, deleteApplication, getApplicationStats, updateApplication, getApplicationEvents } from "./server";
import { useRouter } from "@tanstack/react-router";

export function useApplications() {
  return useQuery({
    queryKey: ["applications"],
    queryFn: getApplications,
  });
}

export function useApplicationStats() {
  return useQuery({
    queryKey: ["application-stats"],
    queryFn: getApplicationStats,
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

export function useUpdateApplication() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: { id: string; [key: string]: unknown }) => updateApplication({ data }),
    onMutate: async (newData) => {
      await queryClient.cancelQueries({ queryKey: ["applications", newData.id] });
      const previousData = queryClient.getQueryData(["applications", newData.id]);

      queryClient.setQueryData(["applications", newData.id], (old: unknown) => {
        if (!old) return old;
        return { ...old, ...newData };
      });

      return { previousData };
    },
    onError: (_err, newData, context) => {
      queryClient.setQueryData(["applications", newData.id], context?.previousData);
    },
    onSettled: (_, __, variables) => {
      queryClient.invalidateQueries({ queryKey: ["applications"] });
      queryClient.invalidateQueries({ queryKey: ["applications", variables.id] });
    },
  });
}

export function useApplicationEvents() {
  return useQuery({
    queryKey: ["application-events"],
    queryFn: getApplicationEvents,
  });
}
