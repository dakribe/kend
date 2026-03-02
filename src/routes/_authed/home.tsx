import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/_authed/home")({
  component: RouteComponent,
});

function RouteComponent() {
  const { session } = Route.useRouteContext();
  return (
    <div>
      <p>{session.user.name}</p>
      <p>{session.user.email}</p>
    </div>
  );
}
