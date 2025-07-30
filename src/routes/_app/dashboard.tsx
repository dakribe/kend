import { Button } from "@/components/ui/button";
import { Stats } from "@/lib/application/stats";
import { authClient } from "@/lib/auth/auth-client";
import {
	createFileRoute,
	useLoaderData,
	useNavigate,
} from "@tanstack/react-router";

export const Route = createFileRoute("/_app/dashboard")({
	component: Dashboard,
	head: () => ({
		meta: [
			{
				title: "Dashboard | Kend",
			},
		],
	}),
});

function Dashboard() {
	const { user, applications } = useLoaderData({ from: "/_app" });
	const navigate = useNavigate();

	async function handleSignOut() {
		await authClient.signOut();
		navigate({ to: "/login" });
	}

	return (
		<>
			<p className="font-bold text-2xl">Hello, {user.name}</p>
			<Stats applications={applications} />
			<Button onClick={handleSignOut}>Sign Out</Button>
		</>
	);
}
