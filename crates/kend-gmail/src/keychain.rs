//! The signed-in accounts in the macOS Keychain. Refresh tokens are stored nowhere else.
//!
//! All accounts share one Keychain item holding a JSON list, so launching reads the Keychain
//! once however many accounts there are. Before kend had several accounts the item held one
//! account on its own; that form is still read.

use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::{Account, Error, Result};

const SERVICE: &str = "dev.kend.gmail";
const USER: &str = "account";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Stored {
    email: String,
    refresh_token: String,
}

/// What the item holds: the list, or the single account an older kend saved.
#[derive(Deserialize)]
#[serde(untagged)]
enum Saved {
    List(Vec<Stored>),
    One(Stored),
}

/// Sign-ins finish on their own threads; this keeps one read-change-write from undoing another.
static WRITING: Mutex<()> = Mutex::new(());

pub(crate) fn load() -> Result<Vec<Account>> {
    Ok(read()?.into_iter().map(|s| Account::new(s.email, s.refresh_token)).collect())
}

/// Adds the account, or replaces the saved one with the same address (signing in again).
pub(crate) fn save(account: &Account) -> Result<()> {
    let _writing = WRITING.lock().unwrap_or_else(|p| p.into_inner());
    let stored = Stored { email: account.email.clone(), refresh_token: account.refresh_token.clone() };
    write(&added(read()?, stored))
}

/// Forgets one account, leaving the others signed in.
pub(crate) fn delete(email: &str) -> Result<()> {
    let _writing = WRITING.lock().unwrap_or_else(|p| p.into_inner());
    write(&removed(read()?, email))
}

fn read() -> Result<Vec<Stored>> {
    match entry()?.get_password() {
        Ok(json) => parse(&json),
        Err(keyring_core::Error::NoEntry) => Ok(Vec::new()),
        Err(err) => Err(err.into()),
    }
}

fn write(accounts: &[Stored]) -> Result<()> {
    if accounts.is_empty() {
        return match entry()?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(err) => Err(err.into()),
        };
    }
    entry()?.set_password(&serde_json::to_string(accounts)?)?;
    Ok(())
}

fn parse(json: &str) -> Result<Vec<Stored>> {
    Ok(match serde_json::from_str(json)? {
        Saved::List(list) => list,
        Saved::One(one) => vec![one],
    })
}

/// Addresses compare ignoring case, as Gmail's do. A new account goes last.
fn added(mut accounts: Vec<Stored>, account: Stored) -> Vec<Stored> {
    match accounts.iter_mut().find(|s| s.email.eq_ignore_ascii_case(&account.email)) {
        Some(existing) => *existing = account,
        None => accounts.push(account),
    }
    accounts
}

fn removed(mut accounts: Vec<Stored>, email: &str) -> Vec<Stored> {
    accounts.retain(|s| !s.email.eq_ignore_ascii_case(email));
    accounts
}

fn entry() -> Result<keyring_core::Entry> {
    static STORE: OnceLock<Result<(), String>> = OnceLock::new();
    STORE.get_or_init(set_store).clone().map_err(Error::Keychain)?;
    Ok(keyring_core::Entry::new(SERVICE, USER)?)
}

#[cfg(target_os = "macos")]
fn set_store() -> Result<(), String> {
    let store = apple_native_keyring_store::keychain::Store::new().map_err(|e| e.to_string())?;
    keyring_core::set_default_store(store);
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn set_store() -> Result<(), String> {
    Err("kend stores credentials in the macOS Keychain, which this platform doesn't have".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored(email: &str, token: &str) -> Stored {
        Stored { email: email.into(), refresh_token: token.into() }
    }

    #[test]
    fn the_single_account_an_older_kend_saved_still_loads() {
        let old = r#"{"email":"me@example.com","refresh_token":"t"}"#;
        assert_eq!(parse(old).unwrap(), [stored("me@example.com", "t")]);
        let list = serde_json::to_string(&[stored("a@x.com", "1"), stored("b@x.com", "2")]).unwrap();
        assert_eq!(parse(&list).unwrap(), [stored("a@x.com", "1"), stored("b@x.com", "2")]);
    }

    #[test]
    fn signing_in_again_replaces_the_account_and_sign_out_removes_only_it() {
        let accounts = added(vec![stored("a@x.com", "1")], stored("b@x.com", "2"));
        let accounts = added(accounts, stored("A@x.com", "3"));
        assert_eq!(accounts, [stored("A@x.com", "3"), stored("b@x.com", "2")]);
        assert_eq!(removed(accounts, "a@X.com"), [stored("b@x.com", "2")]);
    }
}
