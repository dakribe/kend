import { Navbar } from "@/components/navbar";
import { migrate } from "@/pglite/migrate";
import { Link, Outlet, createRootRoute } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";

export const Route = createRootRoute({
	component: RootComponent,
	loader: async () => await migrate(),
	notFoundComponent: () => <div>404 Not Found.</div>,
});

function RootComponent() {
	return (
		<>
			<div className="mx-auto max-w-[85%] bg-accent min-h-screen">
				<Link to="/">Home</Link>
				<Link to="/create">Create</Link>
				<Link to="/apps/">Applications</Link>
				<Outlet />
				<Navbar />
			</div>
			<TanStackRouterDevtools />
		</>
	);
}
