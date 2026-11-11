# kend

A fast, local-first email client for macOS, written in Rust.

- **UI:** [Rune](https://github.com/anomalyco/rune), a native GPU-rendered Rust UI framework
- **Mail:** Gmail API to start
- **Storage:** SQLite on disk, so the inbox opens instantly and works offline

## Status

Early development.

## Build and run

Requires Rust (the version is pinned in `rust-toolchain.toml`).

```sh
cargo run -p kend      # open the app
cargo test             # run the tests
```
