import { createMiddleware } from "@tanstack/react-start";
import { auth } from "./auth";
import { getWebRequest } from "@tanstack/react-start/server";

export const authMiddleware = createMiddleware({ type: "function" }).server(
	async ({ next }) => {
		const { headers } = getWebRequest();
		const session = await auth.api.getSession({
			headers,
		});

		return await next({
			context: {
				user: {
					id: session?.user.id,
					name: session?.user.name,
					image: session?.user.image,
				},
			},
		});
	},
);
