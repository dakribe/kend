import { date, pgTable, uuid, varchar } from "drizzle-orm/pg-core";
import { user } from "../user/user.sql.js";

export const session = pgTable("session", {
	id: uuid("uuid").defaultRandom().primaryKey(),
	userId: uuid("user_id")
		.references(() => user.id)
		.notNull(),
	token: varchar("token").notNull(),
	expiresAt: date("expires_at").notNull(),
	ipAddress: varchar("ip_address"),
	userAgent: varchar("user_agent"),
	createdAt: date("created_at").defaultNow().notNull(),
	updatedAt: date("updated_at").defaultNow().notNull(),
});

export const account = pgTable("account", {
	id: uuid("uuid").defaultRandom().primaryKey(),
	userId: uuid("user_id")
		.references(() => user.id)
		.notNull(),
	accountId: varchar("account_id").notNull(),
	providerId: varchar("provider_id").notNull(),
	accessToken: varchar("access_token"),
	refreshToken: varchar("refresh_token"),
	accessTokenExpiresAt: date("access_token_expires_at"),
	refreshTokenExpiresAt: date("refresh_token_expires_at"),
	scope: varchar("scope"),
	idToken: varchar("id_token"),
	password: varchar("password"),
	createdAt: date("created_at").defaultNow().notNull(),
	updatedAt: date("updated_at").defaultNow().notNull(),
});

export const verification = pgTable("verification", {
	id: uuid("uuid").defaultRandom().primaryKey(),
	identifier: varchar("identifier").notNull(),
	value: varchar("value").notNull(),
	expiresAt: date("expires_at").notNull(),
	createdAt: date("created_at").defaultNow().notNull(),
	updatedAt: date("updated_at").defaultNow().notNull(),
});
