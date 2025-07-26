import { getApplications } from "@/lib/application";
import { columns } from "@/lib/application/columns";
import { DataTable } from "@/lib/application/data-table";
import { useQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/_app/applications")({
	component: RouteComponent,
	loader: async ({ context }) => {
		const applications = await context.queryClient.ensureQueryData({
			queryKey: ["applications"],
			queryFn: getApplications,
			staleTime: 1000 * 60 * 15,
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
		<div className="max-w-[80%] mx-auto">
			<p className="font-bold text-2xl">Applications</p>
			<DataTable columns={columns} data={applications} />
		</div>
	);
}
