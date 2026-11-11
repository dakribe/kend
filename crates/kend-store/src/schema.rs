//! Migrations, tracked in SQLite's `user_version`. Append new steps; never edit old ones.

use rusqlite::Connection;

use crate::{Error, Result};

const MIGRATIONS: &[&str] = &[
    // 1: accounts, labels, messages, label membership and full-text search.
    "
    CREATE TABLE accounts (
        id         INTEGER PRIMARY KEY,
        email      TEXT NOT NULL UNIQUE,
        -- Gmail's history id from the last sync; NULL until the first full sync.
        history_id TEXT
    );

    CREATE TABLE labels (
        account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
        id         TEXT NOT NULL,
        name       TEXT NOT NULL,
        kind       TEXT NOT NULL CHECK (kind IN ('system', 'user')),
        PRIMARY KEY (account_id, id)
    ) WITHOUT ROWID;

    CREATE TABLE messages (
        id             INTEGER PRIMARY KEY,
        account_id     INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
        gmail_id       TEXT NOT NULL,
        thread_id      TEXT NOT NULL,
        from_name      TEXT NOT NULL,
        from_email     TEXT NOT NULL,
        to_addrs       TEXT NOT NULL,
        subject        TEXT NOT NULL,
        snippet        TEXT NOT NULL,
        body_text      TEXT,
        body_html      TEXT,
        received_at    INTEGER NOT NULL,
        -- Copies of the UNREAD and STARRED labels, so list rows need no extra lookup.
        unread         INTEGER NOT NULL,
        starred        INTEGER NOT NULL,
        -- Frecency inputs for ranking search results.
        open_count     INTEGER NOT NULL DEFAULT 0,
        last_opened_at INTEGER,
        UNIQUE (account_id, gmail_id)
    );
    CREATE INDEX messages_by_thread ON messages (account_id, thread_id);

    -- Which labels each message carries. The primary key is the mailbox list's covering
    -- index: one label's messages, newest first, found without touching another table.
    CREATE TABLE message_labels (
        account_id  INTEGER NOT NULL,
        label_id    TEXT NOT NULL,
        received_at INTEGER NOT NULL,
        message_id  INTEGER NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
        PRIMARY KEY (account_id, label_id, received_at, message_id)
    ) WITHOUT ROWID;
    CREATE INDEX message_labels_by_message ON message_labels (message_id);

    -- External-content FTS: the text lives once, in `messages`.
    -- `account_id` is stored but not indexed, so a search filters by account before ranking.
    CREATE VIRTUAL TABLE messages_fts USING fts5 (
        subject, from_name, from_email, to_addrs, snippet, body_text, account_id UNINDEXED,
        content = 'messages', content_rowid = 'id',
        tokenize = 'unicode61 remove_diacritics 2'
    );
    -- `rank` weights, per column: subject and sender count most, body text least.
    INSERT INTO messages_fts (messages_fts, rank) VALUES ('rank', 'bm25(8.0, 5.0, 5.0, 2.0, 1.0, 1.0, 0.0)');
    CREATE TRIGGER messages_fts_insert AFTER INSERT ON messages BEGIN
        INSERT INTO messages_fts (rowid, subject, from_name, from_email, to_addrs, snippet, body_text, account_id)
        VALUES (new.id, new.subject, new.from_name, new.from_email, new.to_addrs, new.snippet, new.body_text, new.account_id);
    END;
    CREATE TRIGGER messages_fts_delete AFTER DELETE ON messages BEGIN
        INSERT INTO messages_fts (messages_fts, rowid, subject, from_name, from_email, to_addrs, snippet, body_text, account_id)
        VALUES ('delete', old.id, old.subject, old.from_name, old.from_email, old.to_addrs, old.snippet, old.body_text, old.account_id);
    END;
    CREATE TRIGGER messages_fts_update
    AFTER UPDATE OF subject, from_name, from_email, to_addrs, snippet, body_text ON messages BEGIN
        INSERT INTO messages_fts (messages_fts, rowid, subject, from_name, from_email, to_addrs, snippet, body_text, account_id)
        VALUES ('delete', old.id, old.subject, old.from_name, old.from_email, old.to_addrs, old.snippet, old.body_text, old.account_id);
        INSERT INTO messages_fts (rowid, subject, from_name, from_email, to_addrs, snippet, body_text, account_id)
        VALUES (new.id, new.subject, new.from_name, new.from_email, new.to_addrs, new.snippet, new.body_text, new.account_id);
    END;
    ",
    // 2: an unfinished full sync's starting point, so an interrupted one resumes.
    "
    ALTER TABLE accounts ADD COLUMN full_sync_start TEXT;
    ",
];

pub(crate) fn migrate(conn: &mut Connection) -> Result<()> {
    let known = MIGRATIONS.len() as i64;
    let found: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if found > known {
        return Err(Error::TooNew { found, known });
    }
    let tx = conn.transaction()?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(found as usize) {
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", i as i64 + 1)?;
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrating_twice_is_a_no_op() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();
        migrate(&mut conn).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, MIGRATIONS.len() as i64);
    }

    #[test]
    fn a_newer_database_is_refused() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", 999).unwrap();
        assert!(matches!(
            migrate(&mut conn),
            Err(Error::TooNew { found: 999, .. })
        ));
    }
}
