/// <reference types="vite/client" />
import type { ReactNode } from "react";
import {
	Outlet,
	HeadContent,
	Scripts,
	createRootRouteWithContext,
} from "@tanstack/react-router";
import appCss from "@/styles/app.css?url";
import { QueryClient } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";

export const Route = createRootRouteWithContext<{
	queryClient: QueryClient;
}>()({
	head: () => ({
		meta: [
			{
				charSet: "utf-8",
			},
			{
				name: "viewport",
				content: "width=device-width, initial-scale=1",
			},
			{
				title: "TanStack Start Starter",
			},
		],
		links: [
			{
				rel: "stylesheet",
				href: appCss,
			},
		],
	}),
	component: RootComponent,
	notFoundComponent: () => <div>Not found</div>,
});

function RootComponent() {
	return (
		<RootDocument>
			<Outlet />
			<ReactQueryDevtools buttonPosition="bottom-right" />
			<TanStackRouterDevtools position="bottom-left" />
		</RootDocument>
	);
}

function RootDocument({ children }: Readonly<{ children: ReactNode }>) {
	return (
		<html>
			<head>
				<HeadContent />
			</head>
			<body className="dark">
				{children}
				<Scripts />
			</body>
		</html>
	);
}
