//! Several accounts' mail read as one: "All Inboxes". Each account is read with its own
//! indexed query and the pages are merged, so a combined page costs one page per account.

use std::collections::HashMap;

use crate::{AccountId, Cursor, MessageSummary, Result, Store};

impl Store {
    /// One page of a mailbox across `accounts`, newest first. Pages continue from the last
    /// row's [`MessageSummary::cursor`], as with [`Store::list`].
    pub fn list_in(
        &self,
        accounts: &[AccountId],
        label_id: &str,
        after: Option<Cursor>,
        limit: usize,
    ) -> Result<Vec<MessageSummary>> {
        let mut rows = Vec::new();
        for &account in accounts {
            rows.extend(self.list(account, label_id, after, limit)?);
        }
        Ok(newest(rows, limit))
    }

    /// One page of everything from `email` across `accounts`, as [`Store::list_from`].
    pub fn list_from_in(
        &self,
        accounts: &[AccountId],
        email: &str,
        after: Option<Cursor>,
        limit: usize,
    ) -> Result<Vec<MessageSummary>> {
        let mut rows = Vec::new();
        for &account in accounts {
            rows.extend(self.list_from(account, email, after, limit)?);
        }
        Ok(newest(rows, limit))
    }

    /// Messages in a mailbox across `accounts`.
    pub fn count_in(&self, accounts: &[AccountId], label_id: &str) -> Result<u64> {
        accounts.iter().map(|&a| self.count(a, label_id)).sum()
    }

    /// Messages from `email` across `accounts`, and how many are unread.
    pub fn count_from_in(&self, accounts: &[AccountId], email: &str) -> Result<(u64, u64)> {
        let mut sum = (0, 0);
        for &account in accounts {
            let (total, unread) = self.count_from(account, email)?;
            sum = (sum.0 + total, sum.1 + unread);
        }
        Ok(sum)
    }

    /// Unread messages per label id across `accounts`. Only Gmail's own label ids (INBOX,
    /// STARRED, …) mean the same thing in every account.
    pub fn unread_counts_in(&self, accounts: &[AccountId]) -> Result<HashMap<String, u64>> {
        let mut sum = HashMap::new();
        for &account in accounts {
            for (label, n) in self.unread_counts(account)? {
                *sum.entry(label).or_default() += n;
            }
        }
        Ok(sum)
    }
}

/// The `limit` newest rows, in the order a single mailbox page has them.
fn newest(mut rows: Vec<MessageSummary>, limit: usize) -> Vec<MessageSummary> {
    rows.sort_unstable_by_key(|m| std::cmp::Reverse((m.received_at, m.id.0)));
    rows.truncate(limit);
    rows
}

#[cfg(test)]
mod tests {
    use crate::messages::tests::msg;
    use crate::{MessageSummary, Store};

    fn ids(rows: &[MessageSummary]) -> Vec<&str> {
        rows.iter().map(|m| m.gmail_id.as_str()).collect()
    }

    #[test]
    fn inboxes_merge_newest_first_and_page_across_accounts() {
        let mut store = Store::open_in_memory().unwrap();
        let a = store.upsert_account("me@example.com").unwrap();
        let b = store.upsert_account("work@example.com").unwrap();
        store.upsert_messages(a, &[msg("a1", 1, &["INBOX"]), msg("a3", 3, &["INBOX", "UNREAD"])]).unwrap();
        store
            .upsert_messages(b, &[msg("b2", 2, &["INBOX", "UNREAD"]), msg("b4", 4, &["INBOX"]), msg("b5", 5, &[])])
            .unwrap();

        let first = store.list_in(&[a, b], "INBOX", None, 3).unwrap();
        assert_eq!(ids(&first), ["b4", "a3", "b2"]);
        assert_eq!(first[0].account, b);
        assert_eq!(first[1].account, a);
        let rest = store.list_in(&[a, b], "INBOX", first.last().map(|m| m.cursor()), 3).unwrap();
        assert_eq!(ids(&rest), ["a1"]);

        assert_eq!(store.count_in(&[a, b], "INBOX").unwrap(), 4);
        assert_eq!(store.unread_counts_in(&[a, b]).unwrap()["INBOX"], 2);
        assert_eq!(store.count_from_in(&[a, b], "ana@example.com").unwrap(), (5, 2));
        assert_eq!(ids(&store.list_from_in(&[a, b], "ana@example.com", None, 2).unwrap()), ["b5", "b4"]);
        assert_eq!(ids(&store.list_in(&[b], "INBOX", None, 10).unwrap()), ["b4", "b2"], "one account alone");
    }

    #[test]
    fn search_covers_the_accounts_asked_for() {
        let mut store = Store::open_in_memory().unwrap();
        let a = store.upsert_account("me@example.com").unwrap();
        let b = store.upsert_account("work@example.com").unwrap();
        let c = store.upsert_account("old@example.com").unwrap();
        store.upsert_messages(a, &[msg("a1", 1, &["INBOX"])]).unwrap();
        store.upsert_messages(b, &[msg("b1", 2, &["INBOX"])]).unwrap();
        store.upsert_messages(c, &[msg("c1", 3, &["INBOX"])]).unwrap();
        let results = store.search_in(&[a, b], "subject", 10).unwrap();
        let mut found = ids(&results);
        found.sort();
        assert_eq!(found, ["a1", "b1"]);
        assert!(store.search_in(&[], "subject", 10).unwrap().is_empty());
    }
}
