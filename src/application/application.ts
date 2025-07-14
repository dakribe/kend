import { db } from "~/drizzle";
import { application as ApplicationTable } from "./sql";
import { action } from "@solidjs/router";

interface CreateData {
	company: string;
	title: string;
}

export const createAction = action(async (data: CreateData) => {
	const [result] = await db
		.insert(ApplicationTable)
		.values({ title: data.title, company: data.company })
		.returning();
	return result;
});
