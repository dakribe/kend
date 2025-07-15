import { ObjectParser } from "@pilcrowjs/object-parser";
import type { APIEvent } from "@solidjs/start/server";
import { decodeIdToken, OAuth2Tokens } from "arctic";
import { getCookie } from "vinxi/http";
import { google } from "~/auth/google";
import { createSession } from "~/auth/session";
import { createUser, getUserByGoogleId } from "~/user/user";

export async function GET(e: APIEvent) {
	const url = new URL(e.request.url);
	const code = url.searchParams.get("code");
	const state = url.searchParams.get("state");
	const storedState = getCookie("google_oauth_state") ?? null;
	const codeVerifier = getCookie("google_code_verifier") ?? null;
	if (
		code === null ||
		state === null ||
		storedState === null ||
		codeVerifier === null
	) {
		return new Response("Please restart the process.", {
			status: 400,
		});
	}
	if (state !== storedState) {
		return new Response("Please restsrt the process.", {
			status: 400,
		});
	}

	let tokens: OAuth2Tokens;
	try {
		tokens = await google.validateAuthorizationCode(code, codeVerifier);
	} catch {
		return new Response("Please restart the process", {
			status: 400,
		});
	}

	const claims = decodeIdToken(tokens.idToken());
	const claimsParser = new ObjectParser(claims);

	const googleId = claimsParser.getString("sub");
	const name = claimsParser.getString("name");
	const picture = claimsParser.getString("picture");
	const email = claimsParser.getString("email");

	const existingUser = await getUserByGoogleId(googleId);
	if (existingUser) {
		await createSession(existingUser.id);
		return new Response(null, {
			status: 302,
			headers: {
				Location: "/dashboard",
			},
		});
	}

	const user = await createUser({ googleId, name, picture, email });
	await createSession(user.id);

	return new Response(null, {
		status: 302,
		headers: {
			Location: "/dashboard",
		},
	});
}
