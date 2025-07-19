import { query, redirect, createAsync } from "@solidjs/router";
import { eq } from "drizzle-orm";
import { Show, For } from "solid-js";
import { application as applicationTable } from "~/application/sql";
import { getAuthUser } from "~/auth/session";
import { db } from "~/drizzle";

const getApplications = query(async () => {
	const userId = await getAuthUser();
	if (!userId) {
		throw redirect("/");
	}

	const applications = await db
		.select()
		.from(applicationTable)
		.where(eq(applicationTable.userId, userId));
	return applications;
}, "applications");

export default function Applications() {
	const applications = createAsync(() => getApplications());
	return (
		<div>
			<h1>Applications</h1>
			<Show when={applications()} fallback={<p>No applications</p>}>
				<For each={applications()}>
					{(app) => (
						<a href={`/applications/${app.id}`}>
							<p>{app.title}</p>
						</a>
					)}
				</For>
			</Show>
		</div>
	);
}
