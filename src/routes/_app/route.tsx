import { getUser } from "@/lib/user/get-user";
import {
	createFileRoute,
	Link,
	Outlet,
	redirect,
} from "@tanstack/react-router";

export const Route = createFileRoute("/_app")({
	component: AppLayout,
	beforeLoad: async () => {
		const user = await getUser();
		return {
			user,
		};
	},
	loader: async ({ context }) => {
		if (!context.user.id) {
			throw redirect({ to: "/login" });
		}
		return {
			user: context.user,
		};
	},
});

function AppLayout() {
	return (
		<div>
			<div>
				<Link to="/dashboard"></Link>
			</div>
			<Outlet />
		</div>
	);
}
