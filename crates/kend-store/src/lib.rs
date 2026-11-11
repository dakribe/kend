//! kend's local mail store: one SQLite database that the UI reads from directly, so the inbox
//! opens instantly and works offline. Gmail sync writes into it in the background.
//!
//! - Mailbox pages use keyset pagination over a covering index, so page 100 costs the same
//!   as page 1.
//! - Full-text search is SQLite FTS5, kept in step with `messages` by triggers.
//! - Opens are counted per message, for frecency ranking later.

mod accounts;
mod messages;
mod model;
mod schema;
mod search;

use std::path::{Path, PathBuf};

use rusqlite::Connection;

pub use model::{AccountId, Address, Cursor, Label, LabelKind, Message, MessageId, MessageSummary, NewMessage};

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("couldn't create the data folder: {0}")]
    Io(#[from] std::io::Error),
    #[error("no folder for app data on this system")]
    NoDataDir,
    #[error("the database is from a newer kend (schema {found}, this build knows {known})")]
    TooNew { found: i64, known: i64 },
}

pub struct Store {
    conn: Connection,
}

impl Store {
    /// `~/Library/Application Support/kend/mail.sqlite` on macOS.
    pub fn default_path() -> Result<PathBuf> {
        Ok(dirs::data_dir().ok_or(Error::NoDataDir)?.join("kend").join("mail.sqlite"))
    }

    /// Opens (or creates) the database at `path` and brings its schema up to date.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self> {
        // WAL lets the UI read while sync writes; NORMAL sync is safe under WAL and much faster.
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )?;
        schema::migrate(&mut conn)?;
        Ok(Self { conn })
    }
}
