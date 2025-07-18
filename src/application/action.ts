
import { action, redirect } from "@solidjs/router";
import { getAuthUser } from "~/auth/session";
import { db } from "~/drizzle";
import { application } from "./sql";

export const addApplication = action(async (formData: FormData) => {
	"use server";
	console.log("called");
	const userId = await getAuthUser();
	if (!userId) throw redirect("/");

	const title = String(formData.get("title"));
	const company = String(formData.get("company"));

	const [result] = await db
		.insert(application)
		.values({ title, company: company, userId })
		.returning();
	return result;
}, "add-application");
