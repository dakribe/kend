import { action, query, redirect } from "@solidjs/router";
import { getAuthUser } from "~/auth/session";
import { db } from "~/drizzle";
import { application as applicationTable, application } from "./sql";
import { eq } from "drizzle-orm";

export const addApplication = action(async (formData: FormData) => {
	"use server";
	const userId = await getAuthUser();
	if (!userId) throw redirect("/");

	const title = String(formData.get("title"));
	const company = String(formData.get("company"));

	const [result] = await db
		.insert(application)
		.values({ title, company, userId });
	return result;
}, "add-application");

export const getApplication = query(async (id: string) => {
	"use server";
	const [application] = await db
		.select()
		.from(applicationTable)
		.where(eq(applicationTable.id, id))
		.limit(1);
	return application;
}, "get-application");
