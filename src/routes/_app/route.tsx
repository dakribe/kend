import { CreateDialog } from "@/components/create-dialog";
import { Toaster } from "@/components/ui/sonner";
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
		<div className="max-w-[80%] mx-auto">
			<div>
				<Link to="/dashboard">Dashboard</Link>
				<Link to="/applications">Applications</Link>
			</div>
			<Outlet />
			<CreateDialog />
			<Toaster />
		</div>
	);
}
