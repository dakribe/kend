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
		refetchOnWindowFocus: false,
		refetchOnMount: false,
		refetchOnReconnect: false,
		staleTime: Infinity,
	});

	return (
		<>
			<p className="font-bold text-2xl">Applications</p>
			<DataTable columns={columns} data={applications} />
		</>
	);
}
