import { pgTable, text, uuid } from "drizzle-orm/pg-core";

export const application = pgTable("application", {
	id: uuid("id").defaultRandom().primaryKey(),
	company: text("company").notNull(),
	title: text("title").notNull(),
});
