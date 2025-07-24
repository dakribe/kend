import { getUser } from "@/lib/auth/auth";
import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute("/dashboard")({
	component: Dashboard,
	beforeLoad: async () => {
		const user = await getUser();
		return { user };
	},
	loader: async ({ context }) => {
		if (!context.user.id) {
			throw redirect({ to: "/" });
		}
		return {
			user: context.user,
		};
	},
});

function Dashboard() {
	const { user } = Route.useLoaderData();
	return (
		<div>
			<p>{user.name}</p>
		</div>
	);
}
