import { Outlet, createRootRoute } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";

export const Route = createRootRoute({
	component: RootComponent,
	notFoundComponent: () => <div>404 Not Found.</div>,
});

function RootComponent() {
	return (
		<>
			<div>Hello "__root"!</div>
			<Outlet />
			<TanStackRouterDevtools />
		</>
	);
}
