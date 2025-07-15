import { SessionConfig, useSession } from "vinxi/http";

const sessionConfig = {
	password: process.env.SESSION_SECRET as string,
	name: "session",
} as SessionConfig;

type SessionData = {
	userId?: string;
};

export async function createSession(userId: string) {
	const session = await useSession<SessionData>(sessionConfig);
	await session.update({ userId });
}

export async function getSession() {
	return await useSession<SessionData>(sessionConfig);
}

export async function logoutSession() {
	const session = await getSession();
	await session.update({ userId: undefined });
}

export async function getAuthUser() {
	const session = await getSession();
	const userId = session.data.userId;
	if (!userId) return null;
	return userId;
}
