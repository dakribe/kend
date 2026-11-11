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
        self.conn
            .execute("DELETE FROM accounts WHERE id = ?1", [account.0])?;
        Ok(())
    }

    /// Gmail's history id from the last sync, the point the next one continues from.
    pub fn history_id(&self, account: AccountId) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row(
                "SELECT history_id FROM accounts WHERE id = ?1",
                [account.0],
                |r| r.get(0),
            )
            .optional()?
            .flatten())
    }

    pub fn set_history_id(&self, account: AccountId, history_id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE accounts SET history_id = ?2 WHERE id = ?1",
            params![account.0, history_id],
        )?;
        Ok(())
    }

    /// Replaces the account's labels with `labels`.
    pub fn replace_labels(&mut self, account: AccountId, labels: &[Label]) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM labels WHERE account_id = ?1", [account.0])?;
        {
            let mut insert = tx.prepare(
                "INSERT INTO labels (account_id, id, name, kind) VALUES (?1, ?2, ?3, ?4)",
            )?;
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
                kind: if kind == "system" {
                    LabelKind::System
                } else {
                    LabelKind::User
                },
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
    fn labels_are_replaced_and_sorted() {
        let mut store = Store::open_in_memory().unwrap();
        let a = store.upsert_account("me@example.com").unwrap();
        let label = |id: &str, name: &str, kind| Label {
            id: id.into(),
            name: name.into(),
            kind,
        };
        store
            .replace_labels(a, &[label("x", "old", LabelKind::User)])
            .unwrap();
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
        let names: Vec<_> = store
            .labels(a)
            .unwrap()
            .into_iter()
            .map(|l| l.name)
            .collect();
        assert_eq!(names, ["INBOX", "SENT", "Family", "receipts"]);
    }
}
