import { createServerFn } from "@tanstack/react-start";
import { db } from "../drizzle";
import { jobApplication as jobApplicationTable } from "../drizzle/schema";
import { authMiddleware } from "../auth/middleware";
import { eq } from "drizzle-orm";
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
