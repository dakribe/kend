import { createFileRoute, redirect, Outlet, Link, useLocation } from "@tanstack/react-router";
import { getSession } from "#/lib/auth/get-session";
import { ApplicationModalProvider } from "#/lib/application-modal-context";
import { ApplicationModal } from "#/components/applications/application-modal";
import { useApplicationModal } from "#/lib/application-modal-context";
import { authClient } from "#/lib/auth/auth-client";
import { Button } from "#/components/ui/button";
import { useEffect } from "react";
import { LogOutIcon, PlusIcon } from "lucide-react";

export const Route = createFileRoute("/_authed")({
  async beforeLoad() {
    const session = await getSession();

    if (!session?.user) {
      throw redirect({ to: "/" });
    }

    return { session };
  },
  component: AuthedLayout,
});

function AuthedLayout() {
  return (
    <ApplicationModalProvider>
      <AuthedLayoutInner />
    </ApplicationModalProvider>
  );
}

function AuthedLayoutInner() {
  const { setOpen } = useApplicationModal();
  const location = useLocation();
  
  async function handleLogout() {
    await authClient.signOut();
    window.location.href = "/";
  }

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setOpen(true);
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [setOpen]);

  return (
    <>
      <div className="min-h-screen bg-background">
        <header className="border-b px-6 py-3 flex items-center justify-between">
          <div className="flex items-center gap-6">
            <Link
              to="/home"
              className={`text-sm hover:text-foreground ${
                location.pathname === "/home" ? "text-foreground font-medium" : "text-muted-foreground"
              }`}
            >
              Overview
            </Link>
            <Link
              to="/applications"
              className={`text-sm hover:text-foreground ${
                location.pathname === "/applications" ? "text-foreground font-medium" : "text-muted-foreground"
              }`}
            >
              Applications
            </Link>
          </div>
          <div className="flex items-center gap-2">
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => setOpen(true)}
              className="text-muted-foreground hover:text-foreground"
            >
              <PlusIcon className="w-4 h-4" />
              <span>New</span>
              <kbd className="pointer-events-none inline-flex h-5 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium text-muted-foreground">
                <span className="text-xs">⌘</span>K
              </kbd>
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={handleLogout}
              className="text-muted-foreground hover:text-foreground"
            >
              <LogOutIcon className="w-4 h-4" />
              <span>Logout</span>
            </Button>
          </div>
        </header>
        <main>
          <Outlet />
        </main>
      </div>
      <ApplicationModal />
    </>
  );
}
