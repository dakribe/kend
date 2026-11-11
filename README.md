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

## Local mail store

All mail lives in one SQLite database (`~/Library/Application Support/kend/mail.sqlite`).
The UI reads from it directly, and Gmail sync writes into it in the background.

- Mailbox pages use keyset pagination over a covering index, so a page deep in the mailbox
  costs the same as the first.
- Full-text search uses SQLite FTS5, kept in sync with the messages by triggers. Subject and
  sender rank above body text.

Measured on 100,000 messages (`cargo run --release -p kend-store --example bench`, Apple silicon):

| Operation | Time |
|---|---|
| Inbox page, 50 rows (first or halfway down) | 0.05 ms |
| Search for a typical word | 0.1 ms |
| Search for a word in 40% of all mail (worst case) | 26 ms |
| Insert | ~40,000 messages/s |

## Scrolling

The message list builds only the rows near the viewport (Rune's `virtual_rows`), memoises each
row on what it shows, and reads the next 200 rows from SQLite as the list nears its end. Frame
cost stays flat however deep you scroll.

Measured headless on a 5,000-message inbox while scrolling 1,000+ rows deep
(`target/release/kend --scene large --script crates/kend/scripts/scroll.json --out /tmp/frames --every 1000`):

| | Before (all rows built, capped at 100) | After |
|---|---|---|
| Debug build, frame CPU p50 / p95 | 3.6 / 4.2 ms | 1.6 / 3.8 ms |
| Debug build, first frame | 108 ms | 14 ms |
| Release build, frame CPU p50 / p95 | 1.7 / 2.9 ms | 1.8 / 2.3 ms |
| Rows reachable | 100 | the whole mailbox |

Debug builds compile dependencies with optimizations (`[profile.dev.package."*"]`), so
`cargo run` stays smooth while kend's own code builds quickly.

## Build and run

Requires Rust (the version is pinned in `rust-toolchain.toml`).

```sh
cargo run -p kend      # open the app
cargo test             # run the tests
```
