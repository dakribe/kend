import { Button } from "@/components/ui/button";
import { authClient } from "@/lib/auth/auth-client";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/")({
	component: Home,
});

function Home() {
	return (
		<div>
			<Button
				onClick={() =>
					authClient.signIn.social({
						provider: "google",
						callbackURL: "/dashboard",
					})
				}
			>
				Sign In with Google
			</Button>
		</div>
	);
}
