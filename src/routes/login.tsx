import { Button } from "@/components/ui/button";
import {
	Card,
	CardHeader,
	CardTitle,
	CardDescription,
	CardContent,
} from "@/components/ui/card";
import { getUser } from "@/lib/auth/auth";
import { authClient } from "@/lib/auth/auth-client";
import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute("/login")({
	component: Login,
	beforeLoad: async () => {
		const user = await getUser();
		return {
			user,
		};
	},
	loader: async ({ context }) => {
		if (context.user.id) {
			throw redirect({ to: "/dashboard" });
		}
		return;
	},
});

function Login() {
	const { signIn } = authClient;

	return (
		<div className="min-h-svh flex flex-col justify-center items-center">
			<h1 className="text-2xl font-bold mb-8">Kend</h1>
			<Card className="w-80">
				<CardHeader>
					<CardTitle>Welcome</CardTitle>
					<CardDescription>
						Sign in or create an account with Google
					</CardDescription>
				</CardHeader>
				<CardContent>
					<Button
						className="w-full"
						onClick={() =>
							signIn.social({
								provider: "google",
							})
						}
					>
						Continue with Google
					</Button>
				</CardContent>
			</Card>
		</div>
	);
}
