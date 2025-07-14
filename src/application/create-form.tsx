import { action, reload } from "@solidjs/router";
import { db } from "~/drizzle";
import { application } from "./sql";

const addApplication = action(async (formData: FormData) => {
	"use server";

	const company = String(formData.get("company"));
	const title = String(formData.get("title"));

	await db.insert(application).values({ company, title }).returning();
	return reload({ revalidate: "get-applications" });
}, "add-application");

export function CreateForm() {
	return (
		<div>
			<form action={addApplication} method="post">
				<input type="text" name="company" placeholder="Company" />
				<input type="text" name="title" placeholder="Title" />
				<button type="submit">Create</button>
			</form>
		</div>
	);
}
