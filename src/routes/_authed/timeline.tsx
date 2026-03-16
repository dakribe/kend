import { createFileRoute } from "@tanstack/react-router";
import { useApplicationEvents } from "#/lib/applications/hooks";
import { Timeline } from "#/components/timeline/timeline";

export const Route = createFileRoute("/_authed/timeline")({
  component: RouteComponent,
});

function RouteComponent() {
  const { data: events, isLoading } = useApplicationEvents();

  return (
    <div className="max-w-3xl mx-auto p-6">
      <h1 className="text-2xl font-bold mb-6">Timeline</h1>
      {isLoading ? (
        <div className="text-center py-12">
          <p className="text-muted-foreground">Loading...</p>
        </div>
      ) : (
        <Timeline events={events || []} />
      )}
    </div>
  );
}
