import { useForm } from "react-hook-form";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import {
	Form,
	FormControl,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
} from "./ui/form";
import { Input } from "./ui/input";
import { Button } from "./ui/button";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { createApplication, CreateApplicationSchema } from "@/lib/application";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "./ui/dialog";
import { useEffect } from "react";

interface DialogProps {
	open: boolean;
	setOpen: React.Dispatch<React.SetStateAction<boolean>>;
}

export function CreateDialog({ open, setOpen }: DialogProps) {
	useEffect(() => {
		const down = (e: KeyboardEvent) => {
			if (
				e.key === "c" &&
				!(e.target instanceof HTMLInputElement) &&
				!(e.target instanceof HTMLTextAreaElement)
			) {
				e.preventDefault();
				setOpen((open) => !open);
			}
		};
		document.addEventListener("keydown", down);
		return () => document.removeEventListener("keydown", down);
	}, []);
	return (
		<Dialog open={open} onOpenChange={setOpen}>
			<DialogContent className="top-[20%] translate-y-[-20%] data-[state=open]:animate-none data-[state=closed]:animate-none">
				<DialogHeader>
					<DialogTitle>Create Application</DialogTitle>
					<DialogDescription>
						Create and track a new job application
					</DialogDescription>
				</DialogHeader>
				<CreateApplicationForm setOpen={setOpen} />
			</DialogContent>
		</Dialog>
	);
}

interface Props {
	setOpen: React.Dispatch<React.SetStateAction<boolean>>;
}

function CreateApplicationForm({ setOpen }: Props) {
	const form = useForm<z.infer<typeof CreateApplicationSchema>>({
		resolver: zodResolver(CreateApplicationSchema),
	});

	const queryClient = useQueryClient();
	const { mutateAsync } = useMutation({
		mutationKey: ["create"],
		mutationFn: createApplication,
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ["applications"] });
		},
	});

	function onSubmit(values: z.infer<typeof CreateApplicationSchema>) {
		mutateAsync({
			data: {
				title: values.title,
				company: values.company,
				location: values.location,
			},
		});
		form.reset({
			title: "",
			company: "",
			location: "",
		});
		setOpen(false);
		toast("Application created!");
	}
	return (
		<Form {...form}>
			<form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
				<FormField
					control={form.control}
					name="company"
					render={({ field }) => (
						<FormItem>
							<FormLabel>Company</FormLabel>
							<FormControl>
								<Input placeholder="Jane Street" {...field} />
							</FormControl>
							<FormMessage />
						</FormItem>
					)}
				/>
				<FormField
					control={form.control}
					name="title"
					render={({ field }) => (
						<FormItem>
							<FormLabel>Title</FormLabel>
							<FormControl>
								<Input placeholder="Software Engineer" {...field} />
							</FormControl>
							<FormMessage />
						</FormItem>
					)}
				/>
				<FormField
					control={form.control}
					name="location"
					render={({ field }) => (
						<FormItem>
							<FormLabel>Location</FormLabel>
							<FormControl>
								<Input placeholder="Remote" {...field} />
							</FormControl>
						</FormItem>
					)}
				/>
				<Button>Create</Button>
			</form>
		</Form>
	);
}
