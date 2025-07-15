import { db } from "~/drizzle";
import { user as userTable } from "./sql";
import { eq } from "drizzle-orm";
import { query, redirect } from "@solidjs/router";
import { getAuthUser } from "~/auth/session";

export type User = typeof userTable.$inferSelect;
export type InsertUser = typeof userTable.$inferInsert;

export async function createUser(values: InsertUser) {
	const [user] = await db.insert(userTable).values(values).returning();
	return user;
}

export async function getUserByGoogleId(id: string) {
	const [user] = await db
		.select()
		.from(userTable)
		.where(eq(userTable.googleId, id));
	return user;
}

export const getUser = query(async () => {
	"use server";
	const userId = await getAuthUser();
	if (!userId) throw redirect("/");
	const user = await db.query.user.findFirst({
		where: eq(userTable.id, userId),
	});
	if (!user) throw redirect("/");
	return user;
}, "user");
