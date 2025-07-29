import { useState, useEffect } from "react";
import {
	CommandDialog,
	CommandEmpty,
	CommandGroup,
	CommandInput,
	CommandItem,
	CommandList,
} from "./ui/command";
import { JobApplication } from "@/lib/drizzle/schema";
import { useNavigate } from "@tanstack/react-router";

interface Props {
	applications: JobApplication[];
}

export function CommandMenu({ applications }: Props) {
	const [open, setOpen] = useState(false);

	useEffect(() => {
		const down = (e: KeyboardEvent) => {
			if (e.key === "k" && (e.metaKey || e.ctrlKey)) {
				e.preventDefault();
				setOpen((open) => !open);
			}
		};
		document.addEventListener("keydown", down);
		return () => document.removeEventListener("keydown", down);
	}, []);

	const navigate = useNavigate();

	const handleApplicationSelect = (app: JobApplication) => {
		setOpen(false);
		navigate({
			to: "/application/$id",
			params: {
				id: app.id,
			},
		});
	};

	return (
		<CommandDialog
			open={open}
			onOpenChange={setOpen}
			className="top-[20%] translate-y-[-20%] data-[state=open]:animate-none data-[state=closed]:animate-none"
		>
			<CommandInput placeholder="Type a command or search..." />
			<CommandList>
				<CommandEmpty>No results found.</CommandEmpty>
				<CommandGroup heading="Applications">
					{applications.map((app) => (
						<CommandItem
							key={app.id}
							onSelect={() => handleApplicationSelect(app)}
						>
							<p>
								{app.company}{" "}
								<span className="text-muted-foreground">{app.title}</span>
							</p>
						</CommandItem>
					))}
				</CommandGroup>
			</CommandList>
		</CommandDialog>
	);
}
