import { createFileRoute } from "@tanstack/react-router";
import { Button } from "#/components/ui/button";
import { useApplicationModal } from "#/lib/application-modal-context";

export const Route = createFileRoute("/_authed/home")({
  component: RouteComponent,
});

function RouteComponent() {
  const { session } = Route.useRouteContext();
  const { openModal } = useApplicationModal();

  return (
    <div className="p-8">
      <div className="flex items-center justify-between mb-8">
        <div>
          <h1 className="text-2xl font-bold">Welcome, {session.user.name}</h1>
          <p className="text-muted-foreground">{session.user.email}</p>
        </div>
        <Button onClick={openModal}>New Application</Button>
      </div>
    </div>
  );
}
