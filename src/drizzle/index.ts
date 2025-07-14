import postgres from "postgres";
import { drizzle } from "drizzle-orm/postgres-js";
import { application } from "~/application/sql";

const schema = {
	application,
};

const client = postgres(process.env.DATABASE_URL!);
export const db = drizzle(client, { schema });
