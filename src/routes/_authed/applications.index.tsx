import { createFileRoute, Link } from "@tanstack/react-router";
import { useApplications } from "#/lib/applications/hooks";

export const Route = createFileRoute("/_authed/applications/")({
  component: RouteComponent,
});

function RouteComponent() {
  const { data, isLoading } = useApplications();

  if (isLoading) {
    return <div>Loading...</div>;
  }

  return (
    <div className="p-8">
      <h1 className="text-2xl font-semibold mb-6">Applications</h1>
      <div className="space-y-3">
        {data?.map((application) => (
          <Link
            key={application.id}
            to="/applications/$applicationId"
            params={{ applicationId: application.id }}
            className="block rounded-lg border p-4 hover:bg-muted/30 transition-colors"
          >
            <p className="font-medium">{application.jobTitle}</p>
            <p className="text-sm text-muted-foreground">{application.companyName}</p>
          </Link>
        ))}
      </div>
    </div>
  );
}
