import { defineConfig } from "drizzle-kit";

export default defineConfig({
	out: "./src/pglite/",
	schema: "./src/pglite/schema.ts",
	dialect: "postgresql",
});
