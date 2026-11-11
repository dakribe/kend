use std::fmt;

/// The Google OAuth client kend signs in with, built in at compile time (see `build.rs`).
///
/// A desktop app can't keep its client secret: Google expects it to be extractable, and PKCE,
/// not the secret, is what protects the sign-in.
#[derive(Clone, Copy)]
pub struct ClientCredentials {
    pub id: &'static str,
    pub(crate) secret: &'static str,
}

impl ClientCredentials {
    /// The client from `.env` or the build environment, or `None` when it wasn't set.
    pub fn built_in() -> Option<Self> {
        match (
            option_env!("KEND_GOOGLE_CLIENT_ID"),
            option_env!("KEND_GOOGLE_CLIENT_SECRET"),
        ) {
            (Some(id), Some(secret)) => Some(Self { id, secret }),
            _ => None,
        }
    }
}

impl fmt::Debug for ClientCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientCredentials")
            .field("id", &self.id)
            .field("secret", &"<redacted>")
            .finish()
    }
}
