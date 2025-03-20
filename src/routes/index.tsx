import { CreateApplication } from "@/components/create-application";
import { useApplications } from "@/hooks/use-applications";
import { createFileRoute, Link } from "@tanstack/react-router";

export const Route = createFileRoute("/")({
	component: Index,
});

function Index() {
	const applications = useApplications();
	return (
		<div className="p-2">
			{applications.empty ? (
				<p>No applications</p>
			) : applications.data ? (
				<>
					{applications.data.map((app) => (
						<Link
							to="/app/$id"
							params={{
								id: String(app.id),
							}}
							key={app.id}
						>
							<p>
								{app.company} -- {app.title}
							</p>
						</Link>
					))}
				</>
			) : (
				<></>
			)}
			<CreateApplication />
		</div>
	);
}
