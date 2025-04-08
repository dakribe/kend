import { queryClient } from "@/main";
import { Application } from "@/pglite/schema";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/$id")({
	component: RouteComponent,
});

function RouteComponent() {
	const id = Route.useParams().id;
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
		</div>
	);
}
