import { CommandMenu } from "@/components/command-menu";
import { CreateDialog } from "@/components/create-dialog";
import { Navbar } from "@/components/navbar";
import { Toaster } from "@/components/ui/sonner";
import { useKeyboardShortcuts } from "@/hooks/keyboard-shortcut";
import { getApplications } from "@/lib/application";
import { getUser } from "@/lib/user/get-user";
import { createFileRoute, Outlet, redirect } from "@tanstack/react-router";
import { useState } from "react";

export const Route = createFileRoute("/_app")({
	component: AppLayout,
	beforeLoad: async () => {
		const user = await getUser();
		return {
			user,
		};
	},
	loader: async ({ context }) => {
		if (!context.user.id) {
			throw redirect({ to: "/login" });
		}
		const applications = await context.queryClient.ensureQueryData({
			queryKey: ["applications"],
			queryFn: getApplications,
		});

		return {
			user: context.user,
			applications,
		};
	},
});

function AppLayout() {
	const { applications } = Route.useLoaderData();
	const [openDialog, setOpenDialog] = useState(false);
	useKeyboardShortcuts();

	return (
		<div className="max-w-[80%] mx-auto pt-8 min-h-dvh overflow-hidden h-dvh">
			<Outlet />
			<Navbar setOpen={setOpenDialog} />
			<CreateDialog open={openDialog} setOpen={setOpenDialog} />
			<CommandMenu applications={applications} />
			<Toaster />
		</div>
	);
}
