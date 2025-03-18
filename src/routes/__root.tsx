import { createRootRoute, Outlet } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";
import { RuntimeClient } from "../services/runtime-client";
import { Pglite } from "../services/pglite";
import { PGliteProvider } from "@electric-sql/pglite-react";
import { PgliteDrizzleContext } from "../hooks/use-pglite-drizzle";

export const Route = createRootRoute({
	component: Root,
	loader: () => RuntimeClient.runPromise(Pglite),
	errorComponent: (error) => <pre>{JSON.stringify(error, null, 2)}</pre>,
});

function Root() {
	const { client, orm } = Route.useLoaderData();
	return (
		<PGliteProvider db={client}>
			<PgliteDrizzleContext.Provider value={orm}>
				<Outlet />
				<TanStackRouterDevtools position="bottom-right" />
			</PgliteDrizzleContext.Provider>
		</PGliteProvider>
	);
}
