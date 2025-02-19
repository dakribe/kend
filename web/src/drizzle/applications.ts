import { db } from "./db";
import { ApplicationParams, applications as applicationsTable } from "./schema";

export async function createApplication(params: ApplicationParams) {
	await db.insert(applicationsTable).values(params);
}

export async function getApplications() {
	const applications = await db.select().from(applicationsTable);
	return applications;
}
