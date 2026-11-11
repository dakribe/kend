//! Writing synced messages, listing mailboxes and reading one message.

use rusqlite::{OptionalExtension, Row, Transaction, params};

use crate::{AccountId, Address, Cursor, Message, MessageId, MessageSummary, NewMessage, Result, Store};

/// The columns [`summary_from_row`] reads, in order, from `messages m`.
pub(crate) const SUMMARY_COLUMNS: &str = "m.id, m.gmail_id, m.thread_id, m.from_name, m.from_email, m.subject, m.snippet, m.received_at, m.unread, m.starred";

pub(crate) fn summary_from_row(r: &Row<'_>) -> rusqlite::Result<MessageSummary> {
    Ok(MessageSummary {
        id: MessageId(r.get(0)?),
        gmail_id: r.get(1)?,
        thread_id: r.get(2)?,
        from: Address { name: r.get(3)?, email: r.get(4)? },
        subject: r.get(5)?,
        snippet: r.get(6)?,
        received_at: r.get(7)?,
        unread: r.get(8)?,
        starred: r.get(9)?,
    })
}

impl Store {
    /// Adds or updates messages in one transaction. A `None` body keeps the stored one.
    pub fn upsert_messages(&mut self, account: AccountId, messages: &[NewMessage]) -> Result<Vec<MessageId>> {
        let tx = self.conn.transaction()?;
        let mut ids = Vec::with_capacity(messages.len());
        {
            let mut upsert = tx.prepare_cached(
                "INSERT INTO messages (account_id, gmail_id, thread_id, from_name, from_email, to_addrs,
                                       subject, snippet, body_text, body_html, received_at, unread, starred)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                 ON CONFLICT (account_id, gmail_id) DO UPDATE SET
                     thread_id = excluded.thread_id, from_name = excluded.from_name,
                     from_email = excluded.from_email, to_addrs = excluded.to_addrs,
                     subject = excluded.subject, snippet = excluded.snippet,
                     body_text = coalesce(excluded.body_text, body_text),
                     body_html = coalesce(excluded.body_html, body_html),
                     received_at = excluded.received_at,
                     unread = excluded.unread, starred = excluded.starred
                 RETURNING id",
            )?;
            for m in messages {
                let has = |label: &str| m.label_ids.iter().any(|l| l == label);
                let id: i64 = upsert.query_row(
                    params![
                        account.0,
                        m.gmail_id,
                        m.thread_id,
                        m.from.name,
                        m.from.email,
                        m.to,
                        m.subject,
                        m.snippet,
                        m.body_text,
                        m.body_html,
                        m.received_at,
                        has("UNREAD"),
                        has("STARRED"),
                    ],
                    |r| r.get(0),
                )?;
                write_labels(&tx, account, MessageId(id), m.received_at, &m.label_ids)?;
                ids.push(MessageId(id));
            }
        }
        tx.commit()?;
        Ok(ids)
    }

    /// Replaces one message's labels (a label change from sync, or marking read locally).
    /// Returns false when the message isn't stored.
    pub fn set_labels(&mut self, account: AccountId, gmail_id: &str, label_ids: &[String]) -> Result<bool> {
        let tx = self.conn.transaction()?;
        let found: Option<(i64, i64)> = tx
            .query_row(
                "SELECT id, received_at FROM messages WHERE account_id = ?1 AND gmail_id = ?2",
                params![account.0, gmail_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let Some((id, received_at)) = found else {
            return Ok(false);
        };
        let has = |label: &str| label_ids.iter().any(|l| l == label);
        tx.execute(
            "UPDATE messages SET unread = ?2, starred = ?3 WHERE id = ?1",
            params![id, has("UNREAD"), has("STARRED")],
        )?;
        write_labels(&tx, account, MessageId(id), received_at, label_ids)?;
        tx.commit()?;
        Ok(true)
    }

    /// Removes messages by Gmail id. Their labels and search entries go with them.
    pub fn delete_messages(&mut self, account: AccountId, gmail_ids: &[String]) -> Result<usize> {
        let tx = self.conn.transaction()?;
        let mut deleted = 0;
        {
            let mut delete = tx.prepare_cached("DELETE FROM messages WHERE account_id = ?1 AND gmail_id = ?2")?;
            for gmail_id in gmail_ids {
                deleted += delete.execute(params![account.0, gmail_id])?;
            }
        }
        tx.commit()?;
        Ok(deleted)
    }

    /// One page of a mailbox, newest first. Pass the last row's [`MessageSummary::cursor`] as
    /// `after` for the next page.
    pub fn list(
        &self,
        account: AccountId,
        label_id: &str,
        after: Option<Cursor>,
        limit: usize,
    ) -> Result<Vec<MessageSummary>> {
        let base = format!(
            "SELECT {SUMMARY_COLUMNS} FROM message_labels ml JOIN messages m ON m.id = ml.message_id
             WHERE ml.account_id = ?1 AND ml.label_id = ?2"
        );
        let order = "ORDER BY ml.received_at DESC, ml.message_id DESC LIMIT ?3";
        let limit = limit as i64;
        let rows = match after {
            None => {
                let mut stmt = self.conn.prepare_cached(&format!("{base} {order}"))?;
                stmt.query_map(params![account.0, label_id, limit], summary_from_row)?.collect::<Result<Vec<_>, _>>()?
            }
            Some(c) => {
                let mut stmt = self
                    .conn
                    .prepare_cached(&format!("{base} AND (ml.received_at, ml.message_id) < (?4, ?5) {order}"))?;
                stmt.query_map(params![account.0, label_id, limit, c.received_at, c.id.0], summary_from_row)?
                    .collect::<Result<Vec<_>, _>>()?
            }
        };
        Ok(rows)
    }

    /// Which of `gmail_ids` aren't stored yet, in the order given.
    pub fn missing(&self, account: AccountId, gmail_ids: &[String]) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare_cached("SELECT 1 FROM messages WHERE account_id = ?1 AND gmail_id = ?2")?;
        let mut missing = Vec::new();
        for id in gmail_ids {
            if !stmt.exists(params![account.0, id])? {
                missing.push(id.clone());
            }
        }
        Ok(missing)
    }

    /// Messages in a mailbox.
    pub fn count(&self, account: AccountId, label_id: &str) -> Result<u64> {
        let count: i64 = self.conn.query_row(
            "SELECT count(*) FROM message_labels WHERE account_id = ?1 AND label_id = ?2",
            params![account.0, label_id],
            |r| r.get(0),
        )?;
        Ok(count.max(0) as u64)
    }

    /// Unread messages in a mailbox, for the sidebar's badge.
    pub fn unread_count(&self, account: AccountId, label_id: &str) -> Result<u64> {
        // Walk the (smaller) UNREAD label and probe the mailbox by primary key, rather than
        // walking the whole mailbox.
        let count: i64 = self.conn.query_row(
            "SELECT count(*) FROM message_labels u
             WHERE u.account_id = ?1 AND u.label_id = 'UNREAD' AND EXISTS (
                 SELECT 1 FROM message_labels ml
                 WHERE ml.account_id = u.account_id AND ml.label_id = ?2
                   AND ml.received_at = u.received_at AND ml.message_id = u.message_id)",
            params![account.0, label_id],
            |r| r.get(0),
        )?;
        Ok(count.max(0) as u64)
    }

    /// A whole message, with its body if it has been fetched.
    pub fn message(&self, id: MessageId) -> Result<Option<Message>> {
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT {SUMMARY_COLUMNS}, m.to_addrs, m.body_text, m.body_html FROM messages m WHERE m.id = ?1"
        ))?;
        let Some(mut message) = stmt
            .query_row([id.0], |r| {
                Ok(Message {
                    summary: summary_from_row(r)?,
                    to: r.get(10)?,
                    body_text: r.get(11)?,
                    body_html: r.get(12)?,
                    label_ids: Vec::new(),
                })
            })
            .optional()?
        else {
            return Ok(None);
        };
        let mut labels =
            self.conn.prepare_cached("SELECT label_id FROM message_labels WHERE message_id = ?1 ORDER BY label_id")?;
        message.label_ids = labels.query_map([id.0], |r| r.get(0))?.collect::<Result<_, _>>()?;
        Ok(Some(message))
    }

    /// Stores a body fetched after the message itself.
    pub fn set_body(&self, id: MessageId, text: Option<&str>, html: Option<&str>) -> Result<()> {
        self.conn
            .execute("UPDATE messages SET body_text = ?2, body_html = ?3 WHERE id = ?1", params![id.0, text, html])?;
        Ok(())
    }

    /// Records that the user opened a message (frecency input). `now` is Unix milliseconds.
    pub fn mark_opened(&self, id: MessageId, now: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE messages SET open_count = open_count + 1, last_opened_at = ?2 WHERE id = ?1",
            params![id.0, now],
        )?;
        Ok(())
    }
}

fn write_labels(
    tx: &Transaction<'_>,
    account: AccountId,
    id: MessageId,
    received_at: i64,
    label_ids: &[String],
) -> Result<()> {
    tx.prepare_cached("DELETE FROM message_labels WHERE message_id = ?1")?.execute([id.0])?;
    let mut insert = tx.prepare_cached(
        "INSERT OR IGNORE INTO message_labels (account_id, label_id, received_at, message_id) VALUES (?1, ?2, ?3, ?4)",
    )?;
    for label in label_ids {
        insert.execute(params![account.0, label, received_at, id.0])?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "messages_tests.rs"]
mod tests;
