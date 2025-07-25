import { getApplications } from "@/lib/application";
import { useQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/_app/applications")({
	component: RouteComponent,
	loader: async ({ context }) => {
		const applications = await context.queryClient.ensureQueryData({
			queryKey: ["applications"],
			queryFn: getApplications,
		});

		return {
			applications,
		};
	},
});

function RouteComponent() {
	const { applications: loaderApplications } = Route.useLoaderData();
	const { data: applications = loaderApplications } = useQuery({
		queryKey: ["applications"],
		queryFn: getApplications,
		initialData: loaderApplications,
	});

	return (
		<div>
			{applications.map((app) => (
				<>
					<p>Company: {app.company}</p>
					<p>Title: {app.title}</p>
				</>
			))}
		</div>
	);
}
