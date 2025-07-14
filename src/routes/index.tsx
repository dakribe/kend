import { Title } from "@solidjs/meta";
import { createAsync, query } from "@solidjs/router";
import { For, Show } from "solid-js";
import { CreateForm } from "~/application/create-form";
import { db } from "~/drizzle";

const getApplications = query(async () => {
	"use server";
	const applications = await db.query.application.findMany();
	return applications;
}, "get-applications");

export default function Home() {
	const applications = createAsync(() => getApplications());
	return (
		<main>
			<Title>Hello World</Title>
			<CreateForm />
			<Show when={applications()?.length} fallback={<p>No applications</p>}>
				<For each={applications()}>{(app) => <p>{app.title}</p>}</For>
			</Show>
		</main>
	);
}
