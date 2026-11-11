//! Accounts, their labels and their sync position.

use rusqlite::{OptionalExtension, params};

use crate::{AccountId, Label, LabelKind, Result, Store};

impl Store {
    /// The account's id, adding it on first sight.
    pub fn upsert_account(&self, email: &str) -> Result<AccountId> {
        let id = self.conn.query_row(
            "INSERT INTO accounts (email) VALUES (?1)
             ON CONFLICT (email) DO UPDATE SET email = excluded.email
             RETURNING id",
            [email],
            |r| r.get(0),
        )?;
        Ok(AccountId(id))
    }

    /// Removes the account and, by cascade, all of its mail.
    pub fn delete_account(&self, account: AccountId) -> Result<()> {
        self.conn.execute("DELETE FROM accounts WHERE id = ?1", [account.0])?;
        Ok(())
    }

    /// Removes the account's messages and sync position, keeping the account. Used before a
    /// fresh full sync, when the server's change history no longer reaches back far enough.
    pub fn clear_messages(&mut self, account: AccountId) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM messages WHERE account_id = ?1", [account.0])?;
        tx.execute(
            "UPDATE accounts SET history_id = NULL, full_sync_start = NULL, backfill_before = NULL, backfill_done = 0
             WHERE id = ?1",
            [account.0],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Gmail's history id from the last sync, the point the next one continues from.
    pub fn history_id(&self, account: AccountId) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT history_id FROM accounts WHERE id = ?1", [account.0], |r| r.get(0))
            .optional()?
            .flatten())
    }

    pub fn set_history_id(&self, account: AccountId, history_id: &str) -> Result<()> {
        self.conn.execute("UPDATE accounts SET history_id = ?2 WHERE id = ?1", params![account.0, history_id])?;
        Ok(())
    }

    /// Where an unfinished full sync started, if one is under way.
    pub fn full_sync_start(&self, account: AccountId) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT full_sync_start FROM accounts WHERE id = ?1", [account.0], |r| r.get(0))
            .optional()?
            .flatten())
    }

    /// Records that a full sync began at `history_id`.
    pub fn begin_full_sync(&self, account: AccountId, history_id: &str) -> Result<()> {
        self.conn.execute("UPDATE accounts SET full_sync_start = ?2 WHERE id = ?1", params![account.0, history_id])?;
        Ok(())
    }

    /// Ends a full sync: later syncs replay changes from where it started.
    /// Older mail is then loaded from the oldest inbox message the full sync fetched.
    pub fn finish_full_sync(&self, account: AccountId) -> Result<()> {
        self.conn.execute(
            "UPDATE accounts SET history_id = full_sync_start, full_sync_start = NULL, backfill_done = 0,
                 backfill_before = (SELECT min(received_at) FROM message_labels
                                    WHERE account_id = ?1 AND label_id = 'INBOX')
             WHERE id = ?1 AND full_sync_start IS NOT NULL",
            [account.0],
        )?;
        Ok(())
    }

    /// How far back older mail has been loaded: `None` before it starts, and whether it's done.
    pub fn backfill(&self, account: AccountId) -> Result<(Option<i64>, bool)> {
        Ok(self
            .conn
            .query_row("SELECT backfill_before, backfill_done FROM accounts WHERE id = ?1", [account.0], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .optional()?
            .unwrap_or((None, false)))
    }

    pub fn set_backfill(&self, account: AccountId, before: Option<i64>, done: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE accounts SET backfill_before = ?2, backfill_done = ?3 WHERE id = ?1",
            params![account.0, before, done],
        )?;
        Ok(())
    }

    /// Replaces the account's labels with `labels`.
    pub fn replace_labels(&mut self, account: AccountId, labels: &[Label]) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM labels WHERE account_id = ?1", [account.0])?;
        {
            let mut insert = tx.prepare("INSERT INTO labels (account_id, id, name, kind) VALUES (?1, ?2, ?3, ?4)")?;
            for label in labels {
                let kind = match label.kind {
                    LabelKind::System => "system",
                    LabelKind::User => "user",
                };
                insert.execute(params![account.0, label.id, label.name, kind])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// System labels first, then the user's, each by name.
    pub fn labels(&self, account: AccountId) -> Result<Vec<Label>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, name, kind FROM labels WHERE account_id = ?1
             ORDER BY kind = 'user', name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([account.0], |r| {
            let kind: String = r.get(2)?;
            Ok(Label {
                id: r.get(0)?,
                name: r.get(1)?,
                kind: if kind == "system" { LabelKind::System } else { LabelKind::User },
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_account_is_added_once() {
        let store = Store::open_in_memory().unwrap();
        let a = store.upsert_account("me@example.com").unwrap();
        assert_eq!(store.upsert_account("me@example.com").unwrap(), a);
        assert_ne!(store.upsert_account("other@example.com").unwrap(), a);
    }

    #[test]
    fn the_history_id_round_trips() {
        let store = Store::open_in_memory().unwrap();
        let a = store.upsert_account("me@example.com").unwrap();
        assert_eq!(store.history_id(a).unwrap(), None);
        store.set_history_id(a, "123456").unwrap();
        assert_eq!(store.history_id(a).unwrap().as_deref(), Some("123456"));
    }

    #[test]
    fn a_full_sync_hands_its_start_to_the_history_id() {
        let store = Store::open_in_memory().unwrap();
        let a = store.upsert_account("me@example.com").unwrap();
        store.begin_full_sync(a, "500").unwrap();
        assert_eq!(store.full_sync_start(a).unwrap().as_deref(), Some("500"));
        assert_eq!(store.history_id(a).unwrap(), None);
        store.finish_full_sync(a).unwrap();
        assert_eq!(store.history_id(a).unwrap().as_deref(), Some("500"));
        assert_eq!(store.full_sync_start(a).unwrap(), None);
    }

    #[test]
    fn labels_are_replaced_and_sorted() {
        let mut store = Store::open_in_memory().unwrap();
        let a = store.upsert_account("me@example.com").unwrap();
        let label = |id: &str, name: &str, kind| Label { id: id.into(), name: name.into(), kind };
        store.replace_labels(a, &[label("x", "old", LabelKind::User)]).unwrap();
        store
            .replace_labels(
                a,
                &[
                    label("Label_2", "receipts", LabelKind::User),
                    label("SENT", "SENT", LabelKind::System),
                    label("INBOX", "INBOX", LabelKind::System),
                    label("Label_1", "Family", LabelKind::User),
                ],
            )
            .unwrap();
        let names: Vec<_> = store.labels(a).unwrap().into_iter().map(|l| l.name).collect();
        assert_eq!(names, ["INBOX", "SENT", "Family", "receipts"]);
    }
}
