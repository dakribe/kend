import { Hono } from "hono";
import { greeting, name } from "@kend/core";

const app = new Hono();

const message = greeting(name);

app.get("/", (c) => {
  return c.json({ message });
});

export default app;
