# kend

A fast, local-first email client for macOS, written in Rust.

- **UI:** [Rune](https://github.com/anomalyco/rune), a native GPU-rendered Rust UI framework
- **Mail:** Gmail API to start
- **Storage:** SQLite on disk, so the inbox opens instantly and works offline

## Status

Early development.

## Setup

kend signs in with a Google OAuth client of type "Desktop app":

1. Create a Google Cloud project and enable the Gmail API.
2. Create an OAuth client (Desktop app) at https://console.cloud.google.com/auth/clients.
3. Copy `.env.example` to `.env` and fill in the client ID and secret.

The values are built into the app at compile time. `.env` is ignored by git.

## How sign-in works

Sign-in follows [RFC 8252](https://datatracker.ietf.org/doc/html/rfc8252), OAuth for native apps:

- The system browser handles the Google sign-in, so kend never sees your password.
- Google redirects to a one-shot listener on `127.0.0.1` on a random port.
- PKCE and a `state` check protect the authorization code.
- The refresh token is stored only in the macOS Keychain. Access tokens stay in memory.
- kend asks for one scope, `gmail.modify`: read, label and send mail, with no permanent deletion.

## Build and run

Requires Rust (the version is pinned in `rust-toolchain.toml`).

```sh
cargo run -p kend      # open the app
cargo test             # run the tests
```
