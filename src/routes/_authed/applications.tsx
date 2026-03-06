import { createFileRoute } from "@tanstack/react-router";
import { applicationCollection } from "#/lib/applications/collection";
import { eq } from "@tanstack/db";
import { useLiveQuery } from "@tanstack/react-db";

export const Route = createFileRoute("/_authed/applications")({
  component: RouteComponent,
});

function RouteComponent() {
  const { session } = Route.useRouteContext();

  const { data } = useLiveQuery((q) =>
    q
      .from({ applications: applicationCollection })
      .where(({ applications }) => eq(applications.userId, session.user.id)),
  );

  return (
    <div>
      <p>Applications</p>
      {data.map((application) => (
        <p>{application.jobTitle}</p>
      ))}
    </div>
  );
}
