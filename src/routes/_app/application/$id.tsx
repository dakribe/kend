import { Button } from "@/components/ui/button";
import { EditableInput } from "@/components/ui/editable-input";
import {
	getApplicationById,
	updateJobApplicationCompany,
	updateJobApplicationTitle,
} from "@/lib/application";
import { ToggleBookmark } from "@/lib/application/bookmark";
import { DeleteApplication } from "@/lib/application/delete";
import { JobApplication } from "@/lib/drizzle/schema";
import { useQuery } from "@tanstack/react-query";
import { createFileRoute, Link } from "@tanstack/react-router";
import { ArrowLeft } from "lucide-react";

export const Route = createFileRoute("/_app/application/$id")({
	component: Application,
	loader: async ({ context, params: { id } }) => {
		const applications = context.queryClient.getQueryData<JobApplication[]>([
			"applications",
		]);
		const application = applications?.find((app) => app.id === id);

		return {
			application,
		};
	},
	head: ({ loaderData }) => ({
		meta: [
			{
				title: `${loaderData?.application?.company} | Kend`,
			},
		],
	}),
});

function Application() {
	const { application: cachedApplication } = Route.useLoaderData();
	const { id } = Route.useParams();

	const { data: application, isLoading } = useQuery({
		queryKey: ["application", id],
		queryFn: () => getApplicationById({ data: id }),
		initialData: cachedApplication,
	});

	if (isLoading) {
		return <div>Loading...</div>;
	}

	if (!application) {
		return <div>Application not found</div>;
	}

	return (
		<div>
			<Link to="/applications">
				<Button variant="ghost">
					<ArrowLeft className="size-4" />
					Back to Applications
				</Button>
			</Link>
			<div className="flex justify-between pt-2">
				<EditableInput
					className="text-3xl font-bold"
					id={application.id}
					initialValue={application.company}
					mutationFn={({ id, value }) =>
						updateJobApplicationCompany({
							data: {
								applicationId: id,
								company: value,
							},
						})
					}
				/>
				<div className="flex gap-2">
					<ToggleBookmark
						id={application?.id}
						bookmarked={application?.bookmarked}
					/>
					<DeleteApplication id={application.id} />
				</div>
			</div>
			<EditableInput
				id={application.id}
				initialValue={application.title}
				mutationFn={({ id, value }) =>
					updateJobApplicationTitle({
						data: {
							applicationId: id,
							title: value,
						},
					})
				}
			/>
		</div>
	);
}
