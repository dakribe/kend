import { defineConfig } from "drizzle-kit";

export default defineConfig({
	dialect: "postgresql",
	schema: "./src/drizzle/schema.ts",
	dbCredentials: {
		url: "idb://my-pgdata",
	},
});
