import { Button } from "@/components/ui/button";
import { authClient } from "@/lib/auth/auth-client";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/")({
	component: Home,
});

function Home() {
	const { data: session } = authClient.useSession();
	if (session) {
		return (
			<div>
				<h1>Welcome, {session.user?.name || session.user?.email}!</h1>
				<Button onClick={() => authClient.signOut()}>Sign Out</Button>
			</div>
		);
	}

	return (
		<div>
			<Button
				onClick={() =>
					authClient.signIn.social({
						provider: "google",
					})
				}
			>
				Sign In with Google
			</Button>
		</div>
	);
}
