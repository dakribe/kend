import { pgTable, uuid } from "drizzle-orm/pg-core";

export const applications = pgTable("applications", {
	id: uuid(),
});
