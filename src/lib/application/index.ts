import { createServerFn } from "@tanstack/react-start";
import { db } from "../drizzle";
import { jobApplication as jobApplicationTable } from "../drizzle/schema";
import { authMiddleware } from "../auth/middleware";
import { and, eq } from "drizzle-orm";
import z from "zod";

export const CreateApplicationSchema = z.object({
	company: z.string(),
	title: z.string(),
	status: z.string(),
	appliedDate: z.date(),
});

export const createApplication = createServerFn()
	.validator(CreateApplicationSchema)
	.middleware([authMiddleware])
	.handler(async ({ context, data }) => {
		const userId = context.user.id;
		const { title, company, status, appliedDate } = data;
		const [jobApplication] = await db
			.insert(jobApplicationTable)
			.values({ title, company, userId, status, appliedDate })
			.returning();
		return jobApplication;
	});

export const getApplications = createServerFn()
	.middleware([authMiddleware])
	.handler(async ({ context }) => {
		const userId = context.user.id!;
		const applications = await db.query.jobApplication.findMany({
			where: eq(jobApplicationTable.userId, userId),
		});

		return applications;
	});

export const getApplicationById = createServerFn()
	.validator((id: string) => id)
	.middleware([authMiddleware])
	.handler(async ({ data }) => {
		const id = data;
		const application = await db.query.jobApplication.findFirst({
			where: eq(jobApplicationTable.id, id),
		});

		return application;
	});

export const bookmarkApplication = createServerFn()
	.validator((id: string) => id)
	.middleware([authMiddleware])
	.handler(async ({ context, data }) => {
		await db.update(jobApplicationTable).set({ bookmarked: true });
	});

export const getBookmarkedApplications = createServerFn()
	.middleware([authMiddleware])
	.handler(async ({ context }) => {
		const userId = context.user.id;
		const applications = await db
			.select()
			.from(jobApplicationTable)
			.where(
				and(
					eq(jobApplicationTable.userId, userId!),
					eq(jobApplicationTable.bookmarked, true),
				),
			);

		return applications;
	});
