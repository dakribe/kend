import {
	pgTable,
	text,
	timestamp,
	date,
	boolean,
	uuid,
	varchar,
	index,
} from "drizzle-orm/pg-core";

export const user = pgTable(
	"user",
	{
		id: text("id").primaryKey(),
		name: text("name").notNull(),
		email: text("email").notNull().unique(),
		emailVerified: boolean("email_verified")
			.$defaultFn(() => false)
			.notNull(),
		image: text("image"),
		createdAt: timestamp("created_at")
			.$defaultFn(() => /* @__PURE__ */ new Date())
			.notNull(),
		updatedAt: timestamp("updated_at")
			.$defaultFn(() => /* @__PURE__ */ new Date())
			.notNull(),
	},
	(table) => [index("email_idx").on(table.email)],
);

export const session = pgTable(
	"session",
	{
		id: text("id").primaryKey(),
		expiresAt: timestamp("expires_at").notNull(),
		token: text("token").notNull().unique(),
		createdAt: timestamp("created_at").notNull(),
		updatedAt: timestamp("updated_at").notNull(),
		ipAddress: text("ip_address"),
		userAgent: text("user_agent"),
		userId: text("user_id")
			.notNull()
			.references(() => user.id, { onDelete: "cascade" }),
	},
	(table) => [
		index("session_user_id_idx").on(table.userId),
		index("token_idx").on(table.token),
	],
);

export const account = pgTable(
	"account",
	{
		id: text("id").primaryKey(),
		accountId: text("account_id").notNull(),
		providerId: text("provider_id").notNull(),
		userId: text("user_id")
			.notNull()
			.references(() => user.id, { onDelete: "cascade" }),
		accessToken: text("access_token"),
		refreshToken: text("refresh_token"),
		idToken: text("id_token"),
		accessTokenExpiresAt: timestamp("access_token_expires_at"),
		refreshTokenExpiresAt: timestamp("refresh_token_expires_at"),
		scope: text("scope"),
		password: text("password"),
		createdAt: timestamp("created_at").notNull(),
		updatedAt: timestamp("updated_at").notNull(),
	},
	(table) => [index("user_id_idx").on(table.userId)],
);

export const verification = pgTable(
	"verification",
	{
		id: text("id").primaryKey(),
		identifier: text("identifier").notNull(),
		value: text("value").notNull(),
		expiresAt: timestamp("expires_at").notNull(),
		createdAt: timestamp("created_at").$defaultFn(
			() => /* @__PURE__ */ new Date(),
		),
		updatedAt: timestamp("updated_at").$defaultFn(
			() => /* @__PURE__ */ new Date(),
		),
	},
	(table) => [index("identifier_idx").on(table.identifier)],
);

export const jobApplication = pgTable("job_application", {
	id: uuid("id").defaultRandom().primaryKey(),
	company: varchar("company", { length: 255 }).notNull(),
	title: varchar("title", { length: 255 }).notNull(),
	appliedDate: date("applied_date", { mode: "date" }).defaultNow().notNull(),
	bookmarked: boolean("bookmarked").default(false).notNull(),
	status: varchar("status").default("applied").notNull(),
	location: varchar("location").notNull(),

	userId: text("user_id").references(() => user.id),
});

export type JobApplication = typeof jobApplication.$inferSelect;
export type InsertJobApplication = typeof jobApplication.$inferInsert;
