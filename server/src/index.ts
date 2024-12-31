import { serve } from "@hono/node-server";
import { Hono } from "hono";
import { logger } from "hono/logger";
import { user } from "./user/user.routes.js";

const app = new Hono();
app.use(logger());

app.get("/", (c) => {
	return c.text("Hello Kend!");
});

app.route("/user", user);

const port = 3000;
console.log(`Server is running on http://localhost:${port}`);

serve({
	fetch: app.fetch,
	port,
});
