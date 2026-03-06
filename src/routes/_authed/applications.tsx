import { createFileRoute } from "@tanstack/react-router";
import { useApplications } from "#/lib/applications/hooks";

export const Route = createFileRoute("/_authed/applications")({
  component: RouteComponent,
});

function RouteComponent() {
  const { data, isLoading } = useApplications();

  if (isLoading) {
    return <div>Loading...</div>;
  }

  return (
    <div>
      <p>Applications</p>
      {data?.map((application) => (
        <p key={application.id}>{application.jobTitle}</p>
      ))}
    </div>
  );
}
