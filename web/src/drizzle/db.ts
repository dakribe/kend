import { PGlite } from "@electric-sql/pglite";
import { drizzle } from "drizzle-orm/pglite";
import { applications } from "./schema";

const schema = {
	applications,
};

const client = new PGlite("idb://my-pgdata");
const db = drizzle(client, {
	schema,
});

export { db };
