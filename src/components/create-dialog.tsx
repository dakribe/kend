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
import { Popover, PopoverContent, PopoverTrigger } from "./ui/popover";
import { cn } from "@/lib/utils";
import { CalendarIcon } from "lucide-react";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "./ui/select";
import { Calendar } from "./ui/calendar";
import { format } from "date-fns";

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
			<DialogContent className="top-[20%] translate-y-[-20%]">
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
				status: values.status,
				appliedDate: values.appliedDate,
			},
		});
		form.reset({
			title: "",
			company: "",
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
				<div className="flex gap-4">
					<FormField
						control={form.control}
						name="appliedDate"
						render={({ field }) => (
							<FormItem className="flex-1 flex flex-col">
								<FormLabel>Date</FormLabel>
								<Popover>
									<PopoverTrigger asChild>
										<FormControl>
											<Button
												variant={"outline"}
												className={cn(
													"w-[240px] pl-3 text-left font-normal",
													!field.value && "text-muted-foreground",
												)}
											>
												{field.value ? (
													format(field.value, "PPP")
												) : (
													<span>Pick a date</span>
												)}
												<CalendarIcon className="ml-auto h-4 w-4 opacity-50" />
											</Button>
										</FormControl>
									</PopoverTrigger>
									<PopoverContent className="w-auto p-0" align="start">
										<Calendar
											mode="single"
											selected={field.value}
											onSelect={field.onChange}
											disabled={(date) =>
												date > new Date() || date < new Date("1900-01-01")
											}
											captionLayout="dropdown"
										/>
									</PopoverContent>
								</Popover>
								<FormMessage />
							</FormItem>
						)}
					/>
					<FormField
						control={form.control}
						name="status"
						render={({ field }) => (
							<FormItem className="flex-1">
								<FormLabel>Status</FormLabel>
								<Select
									onValueChange={field.onChange}
									defaultValue={field.value}
								>
									<FormControl>
										<SelectTrigger className="w-full">
											<SelectValue placeholder="Applied" />
										</SelectTrigger>
									</FormControl>
									<SelectContent>
										<SelectItem value="Applied">Applied</SelectItem>
										<SelectItem value="Interviewing">Interviewing</SelectItem>
										<SelectItem value="Rejected">Rejected</SelectItem>
									</SelectContent>
								</Select>
								<FormMessage />
							</FormItem>
						)}
					/>
				</div>
				<Button>Create</Button>
			</form>
		</Form>
	);
}
