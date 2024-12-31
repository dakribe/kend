import { eq } from "drizzle-orm";
import { db } from "../drizzle/index.js";
import { user, type InsertUser } from "./user.sql.js";

export async function createUser(params: InsertUser) {
	return await db.insert(user).values(params).returning();
}

export async function getUserById(id: string) {
	return await db.select().from(user).where(eq(user.id, id));
}

export async function getUserbyUsername(username: string) {
	return await db.select().from(user).where(eq(user.username, username));
}
