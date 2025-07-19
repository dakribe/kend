import {
	A,
	action,
	createAsync,
	redirect,
	RouteDefinition,
} from "@solidjs/router";
import { CreateForm } from "~/application/create-form";
import { logoutSession } from "~/auth/session";
import { getUser } from "~/user/user";

export const route: RouteDefinition = {
	preload: () => getUser(),
};

const logout = action(async () => {
	"use server";
	await logoutSession();
	throw redirect("/");
});

export default function Dashboard() {
	const user = createAsync(() => getUser());

	return (
		<div>
			<h1>Dashboard</h1>
			<A href="/applications">Applications</A>
			<p>{user()?.name}</p>
			<p>{user()?.email}</p>
			<CreateForm />
			<form action={logout} method="post">
				<button type="submit">Sign Out</button>
			</form>
		</div>
	);
}
