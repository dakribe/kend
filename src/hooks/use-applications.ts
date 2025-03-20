import { applicationTable } from "@/drizzle/drizzle";
import { useQuery } from "./use-query";
import { ApplicationSelect } from "@/schema/application";

export const useApplications = () => {
	return useQuery(
		(orm) => orm.select().from(applicationTable).toSQL(),
		ApplicationSelect,
	);
};
