import { createFileRoute, redirect, Outlet } from "@tanstack/react-router";
import { getSession } from "#/lib/auth/get-session";
import { ApplicationModalProvider } from "#/lib/application-modal-context";
import { ApplicationModal } from "#/components/applications/application-modal";
import { useApplicationModal } from "#/lib/application-modal-context";
import { useEffect } from "react";
import { PlusIcon } from "lucide-react";

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
          <div className="font-semibold">Kend</div>
          <button
            type="button"
            onClick={() => setOpen(true)}
            className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors border rounded-md px-3 py-1.5"
          >
            <PlusIcon className="w-4 h-4" />
            <span>New</span>
            <kbd className="pointer-events-none inline-flex h-5 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium text-muted-foreground">
              <span className="text-xs">⌘</span>K
            </kbd>
          </button>
        </header>
        <main>
          <Outlet />
        </main>
      </div>
      <ApplicationModal />
    </>
  );
}
