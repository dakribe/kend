import { createMiddleware } from "@tanstack/react-start";
import { getSession } from "./get-session";

const authMiddleware = createMiddleware({ type: "function" }).server(async ({ next }) => {
  const session = await getSession();

  if (!session?.user) {
    throw new Error("Unauthorized");
  }

  return next({ context: { session } });
});

export default authMiddleware;
