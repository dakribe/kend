import { Button } from "@/components/ui/button";
import { authClient } from "@/lib/auth/auth-client";
import {
	createFileRoute,
	useLoaderData,
	useRouter,
} from "@tanstack/react-router";

export const Route = createFileRoute("/_app/dashboard")({
	component: Dashboard,
});

function Dashboard() {
	const { user } = useLoaderData({ from: "/_app" });
	const router = useRouter();

	function handleSignOut() {
		authClient.signOut();
		router.navigate({ to: "/login" });
	}

	return (
		<div>
			<p>{user.name}</p>
			<Button onClick={handleSignOut}>Sign Out</Button>
		</div>
	);
}
