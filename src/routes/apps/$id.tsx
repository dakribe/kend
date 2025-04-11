import { Application } from "@/pglite/schema";
import { useQueryClient } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/apps/$id")({
	component: RouteComponent,
});

function RouteComponent() {
	const id = Route.useParams().id;
	const queryClient = useQueryClient();
	const applications = queryClient.getQueryData<Application[]>([
		"applications",
	]);

	const application = applications?.find(
		(application) => application.id === id,
	);

	return (
		<div>
			<p>Title: {application?.title}</p>
			<p>Company: {application?.company}</p>
			<p>Id: {application?.id}</p>
			<p>Client Id: {application?.clientId}</p>
		</div>
	);
}
