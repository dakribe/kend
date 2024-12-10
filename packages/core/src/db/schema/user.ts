import * as D from "drizzle-orm/pg-core";
import { db } from "../drizzle";

export const user = D.pgTable("user", {
	id: D.uuid().primaryKey().defaultRandom(),
	name: D.varchar({ length: 32 }).notNull(),
	email: D.text().notNull(),
});

type InsertUser = typeof user.$inferInsert;

export async function createUser(params: InsertUser) {
	return await db.insert(user).values(params).returning();
}
