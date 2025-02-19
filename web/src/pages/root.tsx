import { createResource, createSignal, For } from "solid-js";
import { db } from "../drizzle/db";
import {
	ApplicationParams,
	applications as applicationsTable,
} from "../drizzle/schema";
import { A } from "@solidjs/router";

async function createApplication(params: ApplicationParams) {
	await db.insert(applicationsTable).values(params);
}

async function getApplications() {
	const applications = await db.select().from(applicationsTable);
	return applications;
}

export function Root() {
	const [title, setTitle] = createSignal("");
	const [company, setCompany] = createSignal("");
	const [applications] = createResource(getApplications);

	const handleAdd = (e: MouseEvent) => {
		e.preventDefault();
		createApplication({
			title: title(),
			company: company(),
		});
	};

	return (
		<div>
			<input
				onInput={(e) => setTitle(e.currentTarget.value)}
				placeholder="Title"
			/>
			<input
				onInput={(e) => setCompany(e.currentTarget.value)}
				placeholder="Company"
			/>
			<button onClick={(e) => handleAdd(e)}>Add</button>
			<For each={applications()}>
				{(app) => (
					<A href={`/${app.id}`}>
						<p>
							{app.title} - {app.company}
						</p>
					</A>
				)}
			</For>
		</div>
	);
}
