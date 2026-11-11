//! Signing in, staying signed in and signing out.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::http::{agent, json};
use crate::loopback::Loopback;
use crate::oauth::{self, GMAIL_SCOPE};
use crate::pkce::{Pkce, random_token};
use crate::{ClientCredentials, Error, Result, keychain};

/// How long to wait for the user to finish in the browser.
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// A signed-in Gmail account. Debug output never shows the token.
#[derive(Clone, PartialEq)]
pub struct Account {
    pub email: String,
    pub(crate) refresh_token: String,
}

impl Account {
    pub fn new(email: String, refresh_token: String) -> Self {
        Self {
            email,
            refresh_token,
        }
    }
}

impl fmt::Debug for Account {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Account")
            .field("email", &self.email)
            .field("refresh_token", &"<redacted>")
            .finish()
    }
}

/// A short-lived token for Gmail API calls. Kept in memory only.
pub struct AccessToken {
    pub(crate) token: String,
    expires_at: Instant,
}

impl AccessToken {
    /// False within a minute of expiry, so a request never starts with a token about to lapse.
    pub fn is_fresh(&self) -> bool {
        Instant::now() + Duration::from_secs(60) < self.expires_at
    }

    fn from_response(response: &oauth::TokenResponse) -> Self {
        Self {
            token: response.access_token.clone(),
            expires_at: Instant::now() + Duration::from_secs(response.expires_in),
        }
    }
}

impl fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccessToken")
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}

/// Lets the UI abandon a sign-in that's waiting on the browser.
#[derive(Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// The account saved by an earlier sign-in, if any. Reads the Keychain only; no network.
pub fn saved_account() -> Result<Option<Account>> {
    keychain::load()
}

/// Runs the whole browser sign-in and saves the account to the Keychain. Blocks until the
/// user finishes, cancels or times out, so call it off the UI thread.
pub fn sign_in(client: &ClientCredentials, cancel: &CancelToken) -> Result<Account> {
    let loopback = Loopback::bind()?;
    let redirect_uri = loopback.redirect_uri();
    let pkce = Pkce::new();
    let state = random_token();

    let url = oauth::auth_url(client, &redirect_uri, &pkce.challenge, &state);
    open::that_detached(url.as_str()).map_err(Error::Browser)?;

    let code = loopback.wait_for_code(&state, SIGN_IN_TIMEOUT, cancel)?;
    let tokens = oauth::exchange_code(client, &code, &redirect_uri, &pkce.verifier)?;
    if !tokens.grants(GMAIL_SCOPE) {
        return Err(Error::MissingScope);
    }
    let refresh_token = tokens
        .refresh_token
        .clone()
        .ok_or_else(|| Error::Google("no refresh token was issued".into()))?;
    let email = profile_email(&AccessToken::from_response(&tokens))?;

    let account = Account::new(email, refresh_token);
    keychain::save(&account)?;
    Ok(account)
}

/// A fresh access token for `account`. [`Error::SignedOut`] means Google revoked the sign-in.
pub fn access_token(client: &ClientCredentials, account: &Account) -> Result<AccessToken> {
    Ok(AccessToken::from_response(&oauth::refresh(
        client,
        &account.refresh_token,
    )?))
}

/// Forgets the account locally, then asks Google to revoke its token. The local sign-out
/// stands even if revocation fails (offline, say).
pub fn sign_out(account: &Account) -> Result<()> {
    keychain::delete()?;
    oauth::revoke(&account.refresh_token)
}

fn profile_email(token: &AccessToken) -> Result<String> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Profile {
        email_address: String,
    }
    let response = agent()
        .get("https://gmail.googleapis.com/gmail/v1/users/me/profile")
        .header("Authorization", format!("Bearer {}", token.token))
        .call()?;
    Ok(json::<Profile>(response)?.email_address)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_hides_tokens() {
        let account = Account::new("me@example.com".into(), "1//secret-refresh".into());
        let shown = format!("{account:?}");
        assert!(shown.contains("me@example.com") && !shown.contains("secret-refresh"));
    }
}
