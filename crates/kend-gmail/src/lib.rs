//! Gmail access for kend: sign-in, tokens and the Gmail API.
//!
//! Sign-in follows RFC 8252, OAuth for native apps: the system browser, a loopback redirect,
//! PKCE and a `state` check. The refresh token lives only in the macOS Keychain.

mod account;
mod api;
mod config;
mod error;
mod http;
mod keychain;
mod loopback;
mod oauth;
mod pkce;

pub use account::{AccessToken, Account, CancelToken, access_token, saved_account, sign_in, sign_out};
pub use api::{Gmail, GmailLabel, GmailMessage, History, HistoryMessage, HistoryRecord, MessageList, MessageRef};
pub use config::ClientCredentials;
pub use error::{Error, Result};
