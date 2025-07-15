import {
	action,
	createAsync,
	redirect,
	RouteDefinition,
} from "@solidjs/router";
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
			<p>{user()?.name}</p>
			<p>{user()?.email}</p>
			<form action={logout} method="post">
				<button type="submit">Sign Out</button>
			</form>
		</div>
	);
}
