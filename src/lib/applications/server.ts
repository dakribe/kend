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

export const getApplicationStats = createServerFn({ method: "GET" })
  .middleware([authMiddleware])
  .handler(async ({ context }) => {
    const { session } = context;

    const applications = await db
      .select()
      .from(jobApplication)
      .where(eq(jobApplication.userId, session.user.id));

    const byStatus = {
      wishlist: 0,
      applied: 0,
      interviewing: 0,
      offered: 0,
      rejected: 0,
      withdrawn: 0,
    };

    applications.forEach((app) => {
      if (app.status in byStatus) {
        byStatus[app.status as keyof typeof byStatus]++;
      }
    });

    const recent = applications
      .sort((a, b) => new Date(b.applicationDate).getTime() - new Date(a.applicationDate).getTime())
      .slice(0, 5);

    return {
      total: applications.length,
      byStatus,
      recent: recent.map((app) => ({
        id: app.id,
        companyName: app.companyName,
        jobTitle: app.jobTitle,
        status: app.status,
        applicationDate: app.applicationDate,
      })),
    };
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

const updateApplicationSchema = z.object({
  id: z.string().uuid(),
  companyName: z.string().min(1).optional(),
  jobTitle: z.string().min(1).optional(),
  status: z.enum(["wishlist", "applied", "interviewing", "offered", "rejected", "withdrawn"]).optional(),
  platform: z.string().optional(),
  applicationDate: z.date().optional(),
  jobUrl: z.string().url("Invalid URL").optional().or(z.literal("")),
  location: z.string().optional(),
  salaryRange: z.string().optional(),
  resumeVersion: z.string().optional(),
  notes: z.string().optional(),
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

export const updateApplication = createServerFn({ method: "POST" })
  .middleware([authMiddleware])
  .inputValidator(updateApplicationSchema)
  .handler(async ({ context, data }) => {
    const { session } = context;
    const { id, ...updates } = data;

    const [updated] = await db
      .update(jobApplication)
      .set({
        ...updates,
        jobUrl: updates.jobUrl || null,
      })
      .where(and(eq(jobApplication.id, id), eq(jobApplication.userId, session.user.id)))
      .returning();

    return updated ?? null;
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
