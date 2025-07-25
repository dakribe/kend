import { createMiddleware } from "@tanstack/react-start";
import { auth } from "./auth";
import { getWebRequest, setResponseStatus } from "@tanstack/react-start/server";

export const authMiddleware = createMiddleware({ type: "function" }).server(
	async ({ next }) => {
		const { headers } = getWebRequest();
		const session = await auth.api.getSession({
			headers,
		});

		if (!session) {
			setResponseStatus(401);
			throw new Error("Unauthorized");
		}

		return await next({
			context: {
				user: session.user,
			},
		});
	},
);
