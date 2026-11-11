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
        let Some(fts_query) = fts_query(query) else {
            return Ok(Vec::new());
        };
        // Rank inside the index, then join only the winners to read their rows.
        let hidden = visible_outside_trash("m.id");
        let mut stmt = self.conn.prepare_cached(&format!(
            "SELECT {SUMMARY_COLUMNS} FROM (
                 SELECT rowid, rank FROM messages_fts
                 WHERE messages_fts MATCH ?1 AND account_id = ?2
                 ORDER BY rank LIMIT ?3
             ) f JOIN messages m ON m.id = f.rowid
             WHERE {hidden}
             ORDER BY f.rank, m.received_at DESC"
        ))?;
        let rows = stmt.query_map(params![fts_query, account.0, limit as i64], summary_from_row)?;
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
