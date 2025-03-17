import { defineConfig } from "drizzle-kit";

export default defineConfig({
	out: "./src/drizzle/",
	schema: "./src/drizzle/drizzle.ts",
	dialect: "postgresql",
});
