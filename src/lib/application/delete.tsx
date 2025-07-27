import { Button } from "@/components/ui/button";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Trash } from "lucide-react";
import { deleteApplication } from ".";
import {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
	AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { useNavigate } from "@tanstack/react-router";

interface Props {
	id: string;
}

export function DeleteApplication({ id }: Props) {
	const queryClient = useQueryClient();
	const navigate = useNavigate();
	const { mutateAsync } = useMutation({
		mutationKey: ["delete_application"],
		mutationFn: deleteApplication,
		onMutate: async (variables) => {
			await queryClient.cancelQueries({ queryKey: ["applications"] });
			const previousData = queryClient.getQueryData(["applications"]);
			queryClient.setQueryData(["applications"], (old: any[]) =>
				old?.filter((app) => app.id !== variables.data),
			);
			return { previousData };
		},
		onError: (_, __, context) => {
			queryClient.setQueryData(["applications"], context?.previousData);
		},
		onSettled: () => {
			queryClient.invalidateQueries({ queryKey: ["applications"] });
		},
		onSuccess: () => {
			navigate({ to: "/applications" });
		},
	});

	function handleDelete() {
		mutateAsync({ data: id });
	}
	return (
		<AlertDialog>
			<AlertDialogTrigger asChild>
				<Button variant="outline">
					<Trash className="size-4" />
				</Button>
			</AlertDialogTrigger>
			<AlertDialogContent>
				<AlertDialogHeader>
					<AlertDialogTitle>Delete Application</AlertDialogTitle>
					<AlertDialogDescription>
						This action cannot be undone. This will permanently delete the
						application.
					</AlertDialogDescription>
				</AlertDialogHeader>
				<AlertDialogFooter>
					<AlertDialogCancel>Cancel</AlertDialogCancel>
					<AlertDialogAction onClick={handleDelete}>Delete</AlertDialogAction>
				</AlertDialogFooter>
			</AlertDialogContent>
		</AlertDialog>
	);
}
