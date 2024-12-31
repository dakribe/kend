import { vValidator } from "@hono/valibot-validator";
import { Hono } from "hono";
import { object, string } from "valibot";
import { createUser, getUserById } from "./index.js";

export const user = new Hono();

const createUserSchema = object({
	username: string(),
	password: string(),
});

user.post("/", vValidator("json", createUserSchema), async (c) => {
	const data = c.req.valid("json");
	const [user] = await createUser({
		username: data.username,
		password: data.password,
	});
	return c.json(user);
});

const getUserSchema = object({
	id: string(),
});

user.get("/", vValidator("json", getUserSchema), async (c) => {
	const data = c.req.valid("json");
	const user = await getUserById(data.id);
	return c.json(user);
});
