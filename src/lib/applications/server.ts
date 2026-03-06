import { createServerFn } from "@tanstack/react-start";
import { db } from "../db";
import { jobApplication } from "../schema";
import authMiddleware from "../auth/auth-middleware";
import { eq } from "drizzle-orm";
import { z } from "zod";

export const getApplications = createServerFn({ method: "GET" })
  .middleware([authMiddleware])
  .handler(async ({ context }) => {
    const { session } = context;

    const applications = await db
      .select()
      .from(jobApplication)
      .where(eq(jobApplication.userId, session.user.id));

    return applications;
  });

const createApplicationSchema = z.object({
  companyName: z.string().min(1),
  jobTitle: z.string().min(1),
  status: z.enum(["wishlist", "applied", "interviewing", "offered", "rejected", "withdrawn"]).optional().default("applied"),
  platform: z.string().optional(),
  applicationDate: z.date().optional(),
  jobUrl: z.string().url().optional().or(z.literal("")),
  location: z.string().optional(),
  salaryRange: z.string().optional(),
  resumeVersion: z.string().optional(),
  notes: z.string().optional(),
});

export const createApplication = createServerFn({ method: "POST" })
  .middleware([authMiddleware])
  .inputValidator(createApplicationSchema)
  .handler(async ({ context, data }) => {
    const { session } = context;

    const [created] = await db
      .insert(jobApplication)
      .values({
        userId: session.user.id,
        companyName: data.companyName,
        jobTitle: data.jobTitle,
        status: data.status,
        platform: data.platform,
        applicationDate: data.applicationDate ?? new Date(),
        jobUrl: data.jobUrl || null,
        location: data.location,
        salaryRange: data.salaryRange,
        resumeVersion: data.resumeVersion,
        notes: data.notes,
      })
      .returning();

    return created;
  });
