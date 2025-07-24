import { drizzle } from "drizzle-orm/postgres-js";
import postgres from "postgres";
import { account, session, user, verification } from "./schema";

const schema = {
	user,
	session,
	verification,
	account,
};

const client = postgres(process.env.DATABASE_URL!);
export const db = drizzle(client, { schema });
