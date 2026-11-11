//! Google's OAuth endpoints: the consent URL, code exchange, refresh and revocation.

use serde::Deserialize;
use url::Url;

use crate::http::{agent, json};
use crate::{ClientCredentials, Result};

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const REVOKE_URL: &str = "https://oauth2.googleapis.com/revoke";

/// Read, label and send mail, without permanent deletion. One scope covers everything kend does.
pub(crate) const GMAIL_SCOPE: &str = "https://www.googleapis.com/auth/gmail.modify";

#[derive(Deserialize)]
pub(crate) struct TokenResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub scope: String,
}

impl TokenResponse {
    pub fn grants(&self, scope: &str) -> bool {
        self.scope.split(' ').any(|s| s == scope)
    }
}

pub(crate) fn auth_url(
    client: &ClientCredentials,
    redirect_uri: &str,
    challenge: &str,
    state: &str,
) -> Url {
    let params = [
        ("client_id", client.id),
        ("redirect_uri", redirect_uri),
        ("response_type", "code"),
        ("scope", GMAIL_SCOPE),
        ("code_challenge", challenge),
        ("code_challenge_method", "S256"),
        ("state", state),
        // A refresh token, so kend stays signed in across launches.
        ("access_type", "offline"),
        ("prompt", "consent"),
    ];
    // AUTH_URL is a valid constant, so parsing can't fail.
    #[allow(clippy::expect_used)]
    Url::parse_with_params(AUTH_URL, params).expect("valid auth URL")
}

pub(crate) fn exchange_code(
    client: &ClientCredentials,
    code: &str,
    redirect_uri: &str,
    verifier: &str,
) -> Result<TokenResponse> {
    json(agent().post(TOKEN_URL).send_form([
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("code_verifier", verifier),
        ("client_id", client.id),
        ("client_secret", client.secret),
    ])?)
}

pub(crate) fn refresh(client: &ClientCredentials, refresh_token: &str) -> Result<TokenResponse> {
    json(agent().post(TOKEN_URL).send_form([
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", client.id),
        ("client_secret", client.secret),
    ])?)
}

pub(crate) fn revoke(token: &str) -> Result<()> {
    let response = agent().post(REVOKE_URL).send_form([("token", token)])?;
    json::<serde_json::Value>(response).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_consent_url_asks_for_a_pkce_protected_offline_code() {
        let client = ClientCredentials {
            id: "id-123",
            secret: "never-in-url",
        };
        let url = auth_url(&client, "http://127.0.0.1:5000", "chal", "st");
        let q: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(q["client_id"], "id-123");
        assert_eq!(q["redirect_uri"], "http://127.0.0.1:5000");
        assert_eq!(q["code_challenge"], "chal");
        assert_eq!(q["code_challenge_method"], "S256");
        assert_eq!(q["state"], "st");
        assert_eq!(q["scope"], GMAIL_SCOPE);
        assert_eq!(q["access_type"], "offline");
        assert!(!url.as_str().contains("never-in-url"));
    }
}
