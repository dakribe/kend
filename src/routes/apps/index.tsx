import { useApplications } from "@/pglite/application";
import { createFileRoute, Link } from "@tanstack/react-router";

export const Route = createFileRoute("/apps/")({
	component: RouteComponent,
});

function RouteComponent() {
	const { data } = useApplications();

	return (
		<div>
			{data?.map((application) => (
				<Link
					to="/apps/$id"
					params={{ id: application.id }}
					key={application.id}
				>
					<p>{application.title}</p>
					<p>{application.company}</p>
				</Link>
			))}
		</div>
	);
}
