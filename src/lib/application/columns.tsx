import { ColumnDef } from "@tanstack/react-table";
import { JobApplication } from "../drizzle/schema";
import { Bookmark } from "lucide-react";

export const columns: ColumnDef<JobApplication>[] = [
	{
		accessorKey: "company",
		header: "Company",
	},
	{
		accessorKey: "title",
		header: "Title",
	},
	{
		accessorKey: "status",
		header: "Status",
	},
	{
		accessorKey: "appliedDate",
		header: "Applied Date",
		cell: ({ row }) => {
			const date = row.getValue("appliedDate") as Date;
			const formattedDate = date.toLocaleDateString("en-us");
			return formattedDate;
		},
	},
	{
		accessorKey: "bookmarked",
		header: "",
		cell: ({ row }) => {
			return (
				<Bookmark
					className={`size-4 ${row.getValue("bookmarked") ? "text-yellow-500 fill-yellow-500" : ""}`}
				/>
			);
		},
	},
];
