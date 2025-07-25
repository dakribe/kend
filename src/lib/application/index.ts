import { createServerFn } from "@tanstack/react-start";
import { db } from "../drizzle";
import { jobApplication as jobApplicationTable } from "../drizzle/schema";
import { authMiddleware } from "../auth/middleware";
import { eq } from "drizzle-orm";

interface CreateValues {
	company: string;
	title: string;
}

export const createApplication = createServerFn()
	.validator((values: CreateValues) => values)
	.middleware([authMiddleware])
	.handler(async ({ context, data }) => {
		const userId = context.user.id;
		const { title, company } = data;
		const [jobApplication] = await db
			.insert(jobApplicationTable)
			.values({ title, company, userId })
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
