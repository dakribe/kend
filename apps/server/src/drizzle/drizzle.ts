import { drizzle } from "drizzle-orm/neon-http";
import { user } from "../user/user.sql.js";
import { account, session, verification } from "../auth/auth.sql.js";

export const schema = {
	user,
	session,
	account,
	verification,
};

export const db = drizzle(process.env.DATABASE_URL as string, {
	schema,
});
