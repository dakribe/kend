import { createFileRoute, Outlet } from "@tanstack/react-router";

export const Route = createFileRoute("/_authed/applications")({
  component: ApplicationsLayout,
});

function ApplicationsLayout() {
  return (
    <Outlet />
  );
}
