import { drizzle } from "drizzle-orm/neon-http";
import { usersTable } from "../user/user.sql.js";

const schema = {
	usersTable,
};

export const db = drizzle(process.env.DATABASE_URL as string, {
	schema,
});
