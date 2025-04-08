import { useApplications } from "@/pglite/application";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/")({
	component: RouteComponent,
});

function RouteComponent() {
	const { data } = useApplications();
	return (
		<div>
			{data?.map((application) => (
				<div key={application.id}>
					<p>{application.company}</p>
				</div>
			))}
		</div>
	);
}
