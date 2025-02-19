import { createResource, createSignal, For } from "solid-js";
import { A } from "@solidjs/router";
import {
	createApplication,
	deleteApplication,
	getApplications,
} from "../drizzle/applications";

export function Root() {
	const [title, setTitle] = createSignal("");
	const [company, setCompany] = createSignal("");
	const [applications, { refetch }] = createResource(getApplications);

	const handleAdd = (e: MouseEvent) => {
		e.preventDefault();
		createApplication({
			title: title(),
			company: company(),
		});
		refetch();
	};

	const handleDelete = (id: string) => {
		deleteApplication(id);
		refetch();
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
					<>
						<A href={`/${app.id}`}>
							<p>
								{app.title} - {app.company}
							</p>
						</A>
						<button onClick={() => handleDelete(app.id)}>Delete</button>
					</>
				)}
			</For>
		</div>
	);
}
