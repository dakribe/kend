import { db } from "./pglite";
import migrations from "./migrations.json";
import { MigrationConfig } from "drizzle-orm/migrator";

export async function migrate() {
	db.dialect.migrate(migrations, db.session, {
		migrationsTable: "drizzle_migrations",
	} satisfies Omit<MigrationConfig, "migrationsFolder">);
}
