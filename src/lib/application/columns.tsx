import { ColumnDef } from "@tanstack/react-table";
import { JobApplication } from "../drizzle/schema";

export const columns: ColumnDef<JobApplication>[] = [
	{
		accessorKey: "company",
		header: "Company",
	},
	{
		accessorKey: "title",
		header: "Title",
	},
];
