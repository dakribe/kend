import { getApplications } from "@/lib/application";
import { columns } from "@/lib/application/columns";
import { DataTable } from "@/lib/application/data-table";
import { useQuery } from "@tanstack/react-query";
import { createFileRoute, useLoaderData } from "@tanstack/react-router";

export const Route = createFileRoute("/_app/applications")({
	component: RouteComponent,
	head: () => ({
		meta: [
			{
				title: "Applications | Kend",
			},
		],
	}),
});

function RouteComponent() {
	const { applications: loaderApplications } = useLoaderData({ from: "/_app" });
	const { data: applications = loaderApplications } = useQuery({
		queryKey: ["applications"],
		queryFn: getApplications,
		initialData: loaderApplications,
		refetchOnWindowFocus: false,
		refetchOnMount: false,
		refetchOnReconnect: false,
	});

	return (
		<>
			<p className="font-bold text-2xl">Applications</p>
			<DataTable columns={columns} data={applications} />
		</>
	);
}
