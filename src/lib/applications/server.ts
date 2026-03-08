import { createServerFn } from "@tanstack/react-start";
import { db } from "../db";
import { jobApplication } from "../schema";
import authMiddleware from "../auth/auth-middleware";
import { and, eq } from "drizzle-orm";
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

const getApplicationByIdSchema = z.object({
  id: z.string().uuid(),
});

export const getApplicationById = createServerFn({ method: "GET" })
  .middleware([authMiddleware])
  .inputValidator(getApplicationByIdSchema)
  .handler(async ({ context, data }) => {
    const { session } = context;

    const [application] = await db
      .select()
      .from(jobApplication)
      .where(and(eq(jobApplication.id, data.id), eq(jobApplication.userId, session.user.id)))
      .limit(1);

    return application ?? null;
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

const deleteApplicationSchema = z.object({
  id: z.string().uuid(),
});

export const deleteApplication = createServerFn({ method: "POST" })
  .middleware([authMiddleware])
  .inputValidator(deleteApplicationSchema)
  .handler(async ({ context, data }) => {
    const { session } = context;

    const [deleted] = await db
      .delete(jobApplication)
      .where(and(eq(jobApplication.id, data.id), eq(jobApplication.userId, session.user.id)))
      .returning();

    return deleted ?? null;
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
