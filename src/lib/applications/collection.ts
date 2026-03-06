import { createCollection } from "@tanstack/react-db";
import { queryCollectionOptions } from "@tanstack/query-db-collection";
import z from "zod";
import { getApplications } from "./server";
import { getContext } from "#/integrations/tanstack-query/root-provider";

const queryClient = getContext().queryClient;

export const jobApplicationSchema = z.object({
  id: z.string(),
  userId: z.string(),
  companyName: z.string(),
  jobTitle: z.string(),
  status: z.enum(["wishlist", "applied", "interviewing", "offered", "rejected", "withdrawn"]),
  platform: z.string().optional(),
  applicationDate: z.date(),
  jobUrl: z.string().optional(),
  location: z.string().optional(),
  salaryRange: z.string().optional(),
  resumeVersion: z.string().optional(),
  notes: z.string().optional(),
  createdAt: z.date(),
  updatedAt: z.date(),
});

const applicationCollection = createCollection(
  queryCollectionOptions({
    queryKey: ["applications"],
    queryClient,
    getKey: (item: { id: string }) => item.id,
    queryFn: async () => await getApplications(),
  }),
);

export { applicationCollection };
