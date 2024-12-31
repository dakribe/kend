import { pgTable, uuid, varchar } from "drizzle-orm/pg-core";

export const user = pgTable("user", {
	id: uuid("id").defaultRandom().primaryKey(),
	username: varchar("username", { length: 32 }).notNull(),
	password: varchar("password", { length: 255 }).notNull(),
});

export type InsertUser = typeof user.$inferInsert;
export type User = typeof user.$inferSelect;
