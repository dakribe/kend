import { CreateApplication } from "@/components/create-application";
import { useApplications } from "@/hooks/use-applications";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/")({
	component: Index,
});

function Index() {
	const applications = useApplications();
	return (
		<div className="p-2">
			<h3>Welcome Home!</h3>
			{applications.empty ? (
				<p>No applications</p>
			) : applications.data ? (
				<>
					{applications.data.map((app) => (
						<p>
							{app.company} -- {app.title}
						</p>
					))}
				</>
			) : (
				<></>
			)}
			<CreateApplication />
		</div>
	);
}
