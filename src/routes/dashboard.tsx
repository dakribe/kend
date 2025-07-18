import {
	action,
	createAsync,
	query,
	redirect,
	RouteDefinition,
} from "@solidjs/router";
import { eq } from "drizzle-orm";
import { For, Show } from "solid-js";
import { CreateForm } from "~/application/create-form";
import { application as applicationTable } from "~/application/sql";
import { getAuthUser, logoutSession } from "~/auth/session";
import { db } from "~/drizzle";
import { getUser } from "~/user/user";

export const route: RouteDefinition = {
	preload: () => getUser(),
};

const logout = action(async () => {
	"use server";
	await logoutSession();
	throw redirect("/");
});

const getAll = query(async () => {
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

export default function Dashboard() {
	const user = createAsync(() => getUser());
	const applications = createAsync(() => getAll());

	return (
		<div>
			<h1>Dashboard</h1>
			<p>{user()?.name}</p>
			<p>{user()?.email}</p>
			<Show when={applications()} fallback={<p>No applications</p>}>
				<For each={applications()}>{(app) => <p>{app.title}</p>}</For>
			</Show>
			<CreateForm />
			<form action={logout} method="post">
				<button type="submit">Sign Out</button>
			</form>
		</div>
	);
}
