import { createFileRoute, redirect } from "@tanstack/react-router";
import { getSession } from "#/lib/auth/get-session";

export const Route = createFileRoute("/_authed")({
  async beforeLoad() {
    const session = await getSession();

    if (!session?.user) {
      throw redirect({ to: "/" });
    }

    return { session };
  },
});
