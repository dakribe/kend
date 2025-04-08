import { PGlite } from "@electric-sql/pglite";
import { drizzle } from "drizzle-orm/pglite";

const pglite = new PGlite("idb://kend-data");
export const db = drizzle(pglite);
