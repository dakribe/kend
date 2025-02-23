import { boolean, date, pgTable, uuid, varchar } from "drizzle-orm/pg-core";

export const user = pgTable("user", {
	id: uuid("uuid").defaultRandom().primaryKey(),
	name: varchar("name").notNull(),
	email: varchar("email").notNull(),
	emailVerified: boolean("email_verified").notNull(),
	image: varchar("image"),
	createdAt: date("created_at").defaultNow().notNull(),
	updatedAt: date("updated_at").defaultNow().notNull(),
});
