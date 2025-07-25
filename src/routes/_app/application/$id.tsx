import { JobApplication } from "@/lib/drizzle/schema";
import { createFileRoute, Link } from "@tanstack/react-router";

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
			<Link to={"/applications"}></Link>
			<p>{application?.company}</p>
			<p>{application?.title}</p>
		</div>
	);
}
