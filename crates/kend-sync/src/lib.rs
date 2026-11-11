//! Gmail → local store sync.
//!
//! The first sync lists the newest inbox messages and fetches them in parallel, committing
//! each page as it lands so the inbox fills in while sync runs. Later syncs replay Gmail's
//! change history from the last sync's position, touching only what changed. If that history
//! has expired, the account's local mail is cleared and a full sync runs again.

mod fetch;
mod parse;
mod source;

use std::collections::HashSet;

use kend_store::{AccountId, Store};

pub use source::{ChangePage, MailSource};

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Gmail(#[from] kend_gmail::Error),
    #[error(transparent)]
    Store(#[from] kend_store::Error),
}

#[derive(Clone, Debug)]
pub struct Options {
    /// How many of the newest inbox messages a full sync fetches.
    pub inbox_limit: usize,
    /// Messages listed, fetched and committed together.
    pub page_size: usize,
    /// Parallel requests while fetching a page.
    pub workers: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            inbox_limit: 500,
            page_size: 100,
            workers: 8,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// True when this was a full sync rather than a replay of changes.
    pub full: bool,
    /// Messages added or updated.
    pub fetched: usize,
    pub deleted: usize,
}

/// Brings `account`'s local mail up to date with `source`. `progress` is called after each
/// committed page with how many messages this sync has covered so far.
pub fn sync(
    source: &impl MailSource,
    store: &mut Store,
    account: AccountId,
    options: &Options,
    progress: &mut dyn FnMut(usize),
) -> Result<Report> {
    store.replace_labels(account, &source.labels()?)?;
    if let Some(start) = store.history_id(account)? {
        match incremental(source, store, account, &start, options, progress) {
            // The change history no longer reaches back to `start`.
            Err(Error::Gmail(kend_gmail::Error::NotFound)) => store.clear_messages(account)?,
            other => return other,
        }
    }
    full(source, store, account, options, progress)
}

fn full(
    source: &impl MailSource,
    store: &mut Store,
    account: AccountId,
    options: &Options,
    progress: &mut dyn FnMut(usize),
) -> Result<Report> {
    // The start is recorded before anything is fetched, so changes made while this sync runs
    // are replayed by the next one. An interrupted full sync keeps its original start and
    // skips what it already fetched, so it resumes instead of starting over.
    if store.full_sync_start(account)?.is_none() {
        store.begin_full_sync(account, &source.history_id()?)?;
    }
    let mut report = Report {
        full: true,
        ..Report::default()
    };
    let mut listed = 0;
    let mut page_token = None;
    while listed < options.inbox_limit {
        let want = options.page_size.min(options.inbox_limit - listed);
        let (ids, next) = source.list("INBOX", page_token.as_deref(), want)?;
        let fetched = fetch::all(source, &store.missing(account, &ids)?, options.workers)?;
        store.upsert_messages(account, &fetched.messages)?;
        listed += ids.len();
        report.fetched += fetched.messages.len();
        progress(listed);
        match next {
            Some(token) if !ids.is_empty() => page_token = Some(token),
            _ => break,
        }
    }
    store.finish_full_sync(account)?;
    Ok(report)
}

fn incremental(
    source: &impl MailSource,
    store: &mut Store,
    account: AccountId,
    start: &str,
    options: &Options,
    progress: &mut dyn FnMut(usize),
) -> Result<Report> {
    let mut changed = Vec::new();
    let mut seen = HashSet::new();
    let mut deleted = HashSet::new();
    let mut page_token = None;
    let latest = loop {
        let page = source.history(start, page_token.as_deref())?;
        for id in page.changed {
            if seen.insert(id.clone()) {
                changed.push(id);
            }
        }
        deleted.extend(page.deleted);
        match page.next_page_token {
            Some(token) => page_token = Some(token),
            None => break page.history_id,
        }
    };
    changed.retain(|id| !deleted.contains(id));

    let mut report = Report::default();
    for ids in changed.chunks(options.page_size) {
        let fetched = fetch::all(source, ids, options.workers)?;
        store.upsert_messages(account, &fetched.messages)?;
        deleted.extend(fetched.gone);
        report.fetched += fetched.messages.len();
        progress(report.fetched);
    }
    let deleted: Vec<String> = deleted.into_iter().collect();
    report.deleted = store.delete_messages(account, &deleted)?;
    store.set_history_id(account, &latest)?;
    Ok(report)
}

#[cfg(test)]
mod tests;
