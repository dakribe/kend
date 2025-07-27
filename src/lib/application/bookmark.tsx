import { Button } from "@/components/ui/button";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Bookmark } from "lucide-react";
import { bookmarkApplication } from ".";

interface Props {
	id: string;
	bookmarked: boolean;
}

export function ToggleBookmark({ id, bookmarked }: Props) {
	const queryClient = useQueryClient();
	const { mutateAsync: toggle } = useMutation({
		mutationKey: ["toggle_bookmark"],
		mutationFn: bookmarkApplication,
		onMutate: async () => {
			await queryClient.cancelQueries({ queryKey: ["application", id] });

			const previousData = queryClient.getQueryData(["application", id]);

			queryClient.setQueryData(["application", id], (old: any) =>
				old ? { ...old, bookmarked: !old.bookmarked } : old,
			);

			return { previousData };
		},
		onError: (_, __, context) => {
			if (context?.previousData) {
				queryClient.setQueryData(["application", id], context.previousData);
			}
		},
		onSettled: () => {
			queryClient.invalidateQueries({ queryKey: ["application", id] });
			queryClient.invalidateQueries({ queryKey: ["applications"] });
		},
	});

	function handleToggle() {
		toggle({ data: id });
	}
	return (
		<Button onClick={handleToggle} variant="outline">
			<Bookmark
				className={`size-4 ${bookmarked ? "text-yellow-500 fill-yellow-500" : ""}`}
			/>
		</Button>
	);
}
