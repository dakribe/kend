import { SignInForm } from "~/components/signin-form";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "~/components/ui/card";

export default function SignIn() {
	return (
		<div className="flex items-center justify-center min-w-dvw min-h-screen">
			<Card>
				<CardHeader>
					<CardTitle>Sign In</CardTitle>
					<CardDescription>
						Enter your email to sign in or create an account.
					</CardDescription>
				</CardHeader>
				<CardContent>
					<SignInForm />
				</CardContent>
			</Card>
		</div>
	);
}
