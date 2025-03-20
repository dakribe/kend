import { applicationTable } from "@/drizzle/drizzle";
import { useQuerySingle } from "./use-query";
import { eq } from "drizzle-orm";
import { ApplicationSelect } from "@/schema/application";

export const useApplication = (id: number) => {
	return useQuerySingle(
		(orm) =>
			orm
				.select()
				.from(applicationTable)
				.where(eq(applicationTable.id, id))
				.toSQL(),
		ApplicationSelect,
	);
};
