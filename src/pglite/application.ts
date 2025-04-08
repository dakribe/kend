import { eq } from "drizzle-orm";
import { db } from "./pglite";
import { ApplicationInsert, applicationTable } from "./schema";
import { useQuery } from "@tanstack/react-query";

export async function getApplications() {
	const applications = await db.select().from(applicationTable);
	return applications;
}

export function useApplications() {
	return useQuery({
		queryKey: ["applications"],
		queryFn: () => getApplications(),
	});
}

export async function createApplication(params: ApplicationInsert) {
	await db.insert(applicationTable).values(params);
}

export async function deleteApplication(id: string) {
	await db.delete(applicationTable).where(eq(applicationTable.id, id));
}
