import {
	Card,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { JobApplication } from "../drizzle/schema";

interface Props {
	applications: JobApplication[];
}
export function Stats({ applications }: Props) {
	const totalApplications = applications.length;

	const currentlyInterviewing = applications.filter((app) => {
		return app.status === "Interviewing";
	}).length;

	const rejectionCount = applications.filter((app) => {
		return app.status === "Rejected";
	}).length;

	const activeCount = applications.filter((app) => {
		return app.status !== "Rejected";
	}).length;

	return (
		<div className="grid grid-cols-4 gap-4">
			<Card>
				<CardHeader>
					<CardDescription>Total Applications</CardDescription>
					<CardTitle>{totalApplications}</CardTitle>
				</CardHeader>
			</Card>
			<Card>
				<CardHeader>
					<CardDescription>Currently Interviewing</CardDescription>
					<CardTitle>{currentlyInterviewing}</CardTitle>
				</CardHeader>
			</Card>
			<Card>
				<CardHeader>
					<CardDescription>Rejections</CardDescription>
					<CardTitle>{rejectionCount}</CardTitle>
				</CardHeader>
			</Card>
			<Card>
				<CardHeader>
					<CardDescription>Active Applications</CardDescription>
					<CardTitle>{activeCount}</CardTitle>
				</CardHeader>
			</Card>
		</div>
	);
}
