import { createFileRoute } from "@tanstack/react-router";
import { Button } from "#/components/ui/button";
import { authClient } from "#/lib/auth/auth-client";

export const Route = createFileRoute("/")({ component: App });

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
