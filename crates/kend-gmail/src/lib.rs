//! Gmail access for kend: sign-in, tokens and (later) the Gmail API.
//!
//! Sign-in follows RFC 8252, OAuth for native apps: the system browser, a loopback redirect,
//! PKCE and a `state` check. The refresh token lives only in the macOS Keychain.

mod account;
mod config;
mod error;
mod http;
mod keychain;
mod loopback;
mod oauth;
mod pkce;

pub use account::{
    AccessToken, Account, CancelToken, access_token, saved_account, sign_in, sign_out,
};
pub use config::ClientCredentials;
pub use error::{Error, Result};
