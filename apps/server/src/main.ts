import { Hono } from "hono";
import { name } from "@kend/name";

const app = new Hono();

app.get("/", (c) => {
  return c.text(`Hello ${name}`);
});

Deno.serve(app.fetch);
