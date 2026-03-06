import { createFileRoute, redirect } from "@tanstack/react-router";
import { Button } from "#/components/ui/button";
import { authClient } from "#/lib/auth/auth-client";
import { getSession } from "#/lib/auth/get-session";

export const Route = createFileRoute("/")({
  async beforeLoad() {
    const session = await getSession();
    if (session?.user) {
      throw redirect({ to: "/home" });
    }
  },
  component: App,
});

function App() {
  return (
    <div>
      <Button
        onClick={() => authClient.signIn.social({ provider: "google", callbackURL: "/home" })}
      >
        Sign In
      </Button>
    </div>
  );
}
