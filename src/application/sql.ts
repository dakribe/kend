import { pgTable, text, uuid } from "drizzle-orm/pg-core";
import { user } from "~/user/sql";

export const application = pgTable("application", {
	id: uuid("id").defaultRandom().primaryKey(),
	company: text("company").notNull(),
	title: text("title").notNull(),

	userId: uuid("user_id")
		.references(() => user.id)
		.notNull(),
});

export type InsertApplication = typeof application.$inferInsert;
