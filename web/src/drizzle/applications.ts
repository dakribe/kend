import { eq } from "drizzle-orm";
import { db } from "./db";
import { ApplicationParams, applications as applicationsTable } from "./schema";

export async function createApplication(params: ApplicationParams) {
	await db.insert(applicationsTable).values(params);
}

export async function getApplications() {
	const applications = await db.select().from(applicationsTable);
	return applications;
}

export async function deleteApplication(id: string) {
	await db.delete(applicationsTable).where(eq(applicationsTable.id, id));
}
