import { generateCodeVerifier, generateState } from "arctic";
import { setCookie } from "vinxi/http";
import { google } from "~/auth/google";

export async function GET() {
	const state = generateState();
	const codeVerifier = generateCodeVerifier();
	const url = google.createAuthorizationURL(state, codeVerifier, [
		"openid",
		"profile",
		"email",
	]);

	setCookie("google_oauth_state", state, {
		path: "/",
		httpOnly: true,
		secure: process.env.NODE_ENV === "production",
		maxAge: 60 * 10,
		sameSite: "lax",
	});

	setCookie("google_code_verifier", codeVerifier, {
		path: "/",
		httpOnly: true,
		secure: process.env.NODE_ENV === "production",
		maxAge: 60 * 10,
		sameSite: "lax",
	});

	return new Response(null, {
		status: 302,
		headers: {
			Location: url.toString(),
		},
	});
}
