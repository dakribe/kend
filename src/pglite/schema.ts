import { pgTable, uuid, varchar } from "drizzle-orm/pg-core";

export const applicationTable = pgTable("applications", {
	id: uuid("id").defaultRandom().primaryKey(),
	title: varchar("title").notNull(),
	company: varchar("company").notNull(),
});

export type Application = typeof applicationTable.$inferSelect;
export type ApplicationInsert = typeof applicationTable.$inferInsert;
