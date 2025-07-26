import { Button } from "@/components/ui/button";
import { JobApplication } from "@/lib/drizzle/schema";
import { createFileRoute, Link } from "@tanstack/react-router";
import { ArrowLeft } from "lucide-react";

export const Route = createFileRoute("/_app/application/$id")({
	component: Application,
	loader: async ({ context, params: { id } }) => {
		const applications = context.queryClient.getQueryData<JobApplication[]>([
			"applications",
		]);
		const application = applications?.find((app) => app.id === id);

		return {
			application,
		};
	},
});

function Application() {
	const { application } = Route.useLoaderData();

	return (
		<div>
			<Link to="/applications">
				<Button variant="ghost">
					<ArrowLeft className="size-4" />
					Back to Applications
				</Button>
			</Link>
			<p>{application?.company}</p>
			<p>{application?.title}</p>
		</div>
	);
}
