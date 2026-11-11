pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("couldn't open the browser: {0}")]
    Browser(std::io::Error),
    #[error("sign-in was cancelled")]
    Cancelled,
    #[error("sign-in timed out")]
    TimedOut,
    #[error("Google sign-in was declined ({0})")]
    Declined(String),
    #[error("kend needs access to read and send mail; allow every permission when you sign in")]
    MissingScope,
    #[error("the sign-in has expired or was revoked; sign in again")]
    SignedOut,
    #[error("Google returned an error: {0}")]
    Google(String),
    #[error("network error: {0}")]
    Network(#[from] ureq::Error),
    #[error("local sign-in server failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Keychain error: {0}")]
    Keychain(String),
    #[error("unexpected data: {0}")]
    Decode(#[from] serde_json::Error),
}

impl From<keyring_core::Error> for Error {
    fn from(err: keyring_core::Error) -> Self {
        Error::Keychain(err.to_string())
    }
}
