import { migrate } from "@/pglite/migrate";
import { Link, Outlet, createRootRoute } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";

export const Route = createRootRoute({
	component: RootComponent,
	loader: () => migrate(),
	notFoundComponent: () => <div>404 Not Found.</div>,
});

function RootComponent() {
	return (
		<>
			<div>
				<Link to="/">Home</Link>
				<Link to="/create">Create</Link>
			</div>
			<Outlet />
			<TanStackRouterDevtools />
		</>
	);
}
