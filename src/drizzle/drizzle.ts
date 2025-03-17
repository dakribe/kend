import { integer, pgTable, varchar } from "drizzle-orm/pg-core";

export const applicationTable = pgTable("application", {
	id: integer().primaryKey().generatedByDefaultAsIdentity(),
	title: varchar({ length: 255 }).notNull(),
	company: varchar({ length: 255 }).notNull(),
});
