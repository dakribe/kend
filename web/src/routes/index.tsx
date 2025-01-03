import { Button } from "@/components/ui/button";
import { createFileRoute, Link } from "@tanstack/react-router";

export const Route = createFileRoute("/")({
	component: Index,
});

function Index() {
	return (
		<div>
			<h3>Welcome Home!</h3>
			<Link to="/login">
				<Button>Sign In</Button>
			</Link>
		</div>
	);
}
