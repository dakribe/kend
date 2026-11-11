//! The signed-in account in the macOS Keychain. The refresh token is stored nowhere else.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::{Account, Error, Result};

const SERVICE: &str = "dev.kend.gmail";
const USER: &str = "account";

#[derive(Serialize, Deserialize)]
struct Stored {
    email: String,
    refresh_token: String,
}

pub(crate) fn load() -> Result<Option<Account>> {
    match entry()?.get_password() {
        Ok(json) => {
            let stored: Stored = serde_json::from_str(&json)?;
            Ok(Some(Account::new(stored.email, stored.refresh_token)))
        }
        Err(keyring_core::Error::NoEntry) => Ok(None),
        Err(err) => Err(err.into()),
    }
}

pub(crate) fn save(account: &Account) -> Result<()> {
    let stored = Stored { email: account.email.clone(), refresh_token: account.refresh_token.clone() };
    entry()?.set_password(&serde_json::to_string(&stored)?)?;
    Ok(())
}

pub(crate) fn delete() -> Result<()> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
        Err(err) => Err(err.into()),
    }
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
