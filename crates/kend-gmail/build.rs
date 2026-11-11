//! Bakes the Google OAuth client into the build. Values come from the environment, else from
//! `.env` at the workspace root. A build without them still compiles; the app reports that
//! Gmail isn't configured.

use std::collections::HashMap;
use std::path::PathBuf;

const KEYS: [&str; 2] = ["KEND_GOOGLE_CLIENT_ID", "KEND_GOOGLE_CLIENT_SECRET"];

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let env_file = manifest_dir.join("../../.env");
    println!("cargo::rerun-if-changed={}", env_file.display());

    let mut from_file = HashMap::new();
    if let Ok(entries) = dotenvy::from_path_iter(&env_file) {
        for entry in entries {
            match entry {
                Ok((key, value)) => {
                    from_file.insert(key, value);
                }
                Err(err) => println!("cargo::warning=skipping a malformed line in .env: {err}"),
            }
        }
    }

    for key in KEYS {
        println!("cargo::rerun-if-env-changed={key}");
        let value = std::env::var(key).ok().or_else(|| from_file.remove(key));
        if let Some(value) = value
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
        {
            println!("cargo::rustc-env={key}={value}");
        }
    }
}
