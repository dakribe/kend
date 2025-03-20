import { useApplication } from "@/hooks/use-application";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/app/$id")({
	component: Application,
});

function Application() {
	const { id } = Route.useParams();
	const application = useApplication(Number(id));
	return <div>{application.data?.title}</div>;
}
