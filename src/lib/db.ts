import { drizzle } from "drizzle-orm/postgres-js";
import postgres from "postgres";
import { account, session, user, verification, jobApplication, applicationEvent } from "./schema";

const schema = {
  user,
  session,
  account,
  verification,
  jobApplication,
  applicationEvent,
};

const client = postgres(process.env.DATABASE_URL!);
export const db = drizzle(client, { schema });
