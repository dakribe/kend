import postgres from "postgres";
import { drizzle } from "drizzle-orm/postgres-js";
import { application } from "~/application/sql";
import { user } from "~/user/sql";

const schema = {
	application,
	user,
};

const client = postgres(process.env.DATABASE_URL!);
export const db = drizzle(client, { schema });
