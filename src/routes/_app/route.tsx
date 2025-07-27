import { CreateDialog } from "@/components/create-dialog";
import { Navbar } from "@/components/navbar";
import { Toaster } from "@/components/ui/sonner";
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
		return {
			user: context.user,
		};
	},
});

function AppLayout() {
	const [openDialog, setOpenDialog] = useState(false);

	return (
		<div className="max-w-[80%] mx-auto pt-8 min-h-dvh overflow-hidden h-dvh">
			<Outlet />
			<Navbar setOpen={setOpenDialog} />
			<CreateDialog open={openDialog} setOpen={setOpenDialog} />
			<Toaster />
		</div>
	);
}
