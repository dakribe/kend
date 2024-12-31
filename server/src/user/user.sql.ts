import { pgTable, uuid, varchar } from "drizzle-orm/pg-core";

export const user = pgTable("user", {
	id: uuid("id").defaultRandom().primaryKey(),
	username: varchar("username", { length: 32 }),
	password: varchar("password", { length: 255 }),
});
