//! Full-text search with FTS5. User input becomes a safe FTS query: each word is quoted
//! (so `"`, `-`, `OR` and `NEAR` are just text) and prefix-matched, so results appear while
//! the last word is still being typed.

use rusqlite::params;

use crate::messages::{SUMMARY_COLUMNS, summary_from_row, visible_outside_trash};
use crate::{AccountId, MessageSummary, Result, Store};

impl Store {
    /// Messages matching every word of `query`, best match first: subject and sender
    /// count more than body text (see the `rank` weights in the schema).
    pub fn search(&self, account: AccountId, query: &str, limit: usize) -> Result<Vec<MessageSummary>> {
        self.search_in(&[account], query, limit)
    }

    /// [`Store::search`] across several accounts at once, ranked together.
    pub fn search_in(&self, accounts: &[AccountId], query: &str, limit: usize) -> Result<Vec<MessageSummary>> {
        let Some(fts_query) = fts_query(query) else {
            return Ok(Vec::new());
        };
        if accounts.is_empty() {
            return Ok(Vec::new());
        }
        // Account ids are integers, so listing them in the SQL is safe; the statement cache
        // keeps one statement per set of accounts.
        let accounts = accounts.iter().map(|a| a.0.to_string()).collect::<Vec<_>>().join(", ");
        // Rank inside the index, then join only the winners to read their rows.
        let hidden = visible_outside_trash("m.id");
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT {SUMMARY_COLUMNS} FROM (
                 SELECT rowid, rank FROM messages_fts
                 WHERE messages_fts MATCH ?1 AND account_id IN ({accounts})
                 ORDER BY rank LIMIT ?2
             ) f JOIN messages m ON m.id = f.rowid
             WHERE {hidden}
             ORDER BY f.rank, m.received_at DESC"
        ))?;
        let rows = stmt.query_map(params![fts_query, limit as i64], summary_from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

/// `ana invoice` → `"ana"* "invoice"*`. None when there's nothing to search for.
fn fts_query(input: &str) -> Option<String> {
    let terms: Vec<String> = input
        .split_whitespace()
        .map(|word| word.replace('"', ""))
        .filter(|word| !word.is_empty())
        .map(|word| format!("\"{word}\"*"))
        .collect();
    (!terms.is_empty()).then(|| terms.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_are_quoted_and_prefix_matched() {
        assert_eq!(fts_query("ana invoice").as_deref(), Some("\"ana\"* \"invoice\"*"));
        assert_eq!(fts_query(r#"say "hi" OR -no"#).as_deref(), Some("\"say\"* \"hi\"* \"OR\"* \"-no\"*"));
        assert_eq!(fts_query("  \"\" "), None);
    }
}
