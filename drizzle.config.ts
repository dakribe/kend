import { defineConfig } from "drizzle-kit";

export default defineConfig({
	out: "./src/drizzle",
	schema: "./src/pglite/schema.ts",
	dialect: "postgresql",
});
