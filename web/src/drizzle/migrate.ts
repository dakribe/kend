import type { MigrationConfig } from "drizzle-orm/migrator";
import { db } from "./db";
import migrations from "./migrations.json";

export async function migrate() {
	// dialect and session will appear to not exist...but they do
	db.dialect.migrate(migrations, db.session, {
		migrationsTable: "drizzle_migrations",
	} satisfies Omit<MigrationConfig, "migrationsFolder">);
}
