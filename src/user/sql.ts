import { pgTable, text, uuid } from "drizzle-orm/pg-core";

export const user = pgTable("user", {
	id: uuid("id").defaultRandom().primaryKey(),
	googleId: text("google_id").unique().notNull(),
	name: text("name").notNull(),
	email: text("email").notNull(),
	picture: text("picture").notNull(),
});
