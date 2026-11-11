//! Gmail → local store sync.
//!
//! The first sync lists the newest inbox messages and fetches them in parallel, committing
//! each page as it lands so the inbox fills in while sync runs. Later syncs replay Gmail's
//! change history from the last sync's position, touching only what changed. If that history
//! has expired, the account's local mail is cleared and a full sync runs again.
//!
//! Each sync also downloads the bodies of the newest inbox messages, so they open instantly.
//!
//! Once a full sync is done, each sync also loads one batch of older inbox mail, listed from
//! the store's backfill position backwards, until Gmail has nothing older. The position is
//! saved, so an old message moved back into the inbox can't make it skip mail in between.

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

/// Bodies saved per transaction while prefetching.
const PREFETCH_BATCH: usize = 25;

#[derive(Clone, Debug)]
pub struct Options {
    /// How many of the newest inbox messages a full sync fetches.
    pub inbox_limit: usize,
    /// How many older inbox messages each sync loads in the background.
    pub backfill_batch: usize,
    /// Bodies are kept for this many of the newest inbox messages, so they open instantly.
    pub prefetch_bodies: usize,
    /// Messages listed, fetched and committed together.
    pub page_size: usize,
    /// Parallel requests while fetching a page.
    pub workers: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self { inbox_limit: 500, backfill_batch: 500, prefetch_bodies: 200, page_size: 100, workers: 8 }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// True when this was a full sync rather than a replay of changes.
    pub full: bool,
    /// Messages added or updated.
    pub fetched: usize,
    pub deleted: usize,
    /// Older messages loaded in the background.
    pub backfilled: usize,
    /// Message bodies downloaded ahead of being opened.
    pub bodies: usize,
    /// True while older mail remains to be loaded; sync again soon to continue.
    pub more_to_load: bool,
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
    let mut report = match store.history_id(account)? {
        Some(start) => match incremental(source, store, account, &start, options, progress) {
            // The change history no longer reaches back to `start`.
            Err(Error::Gmail(kend_gmail::Error::NotFound)) => {
                store.clear_messages(account)?;
                full(source, store, account, options, progress)?
            }
            other => other?,
        },
        None => full(source, store, account, options, progress)?,
    };
    // New mail first, then the bodies of the newest messages, then one batch of older mail.
    report.bodies = prefetch_bodies(source, store, account, options)?;
    let covered = report.fetched;
    let (backfilled, more) = backfill(source, store, account, options, &mut |n| progress(covered + n))?;
    report.backfilled = backfilled;
    report.more_to_load = more;
    Ok(report)
}

/// Downloads the bodies the newest `prefetch_bodies` inbox messages don't have yet, so opening
/// them needs no network and their text is searchable. Returns how many were stored.
///
/// Prefetching is optional, so Gmail pushing back or a dropped connection ends it for this
/// round without failing the sync. Bodies are saved in small batches, so little is lost; the
/// next sync picks up the rest.
fn prefetch_bodies(
    source: &impl MailSource,
    store: &mut Store,
    account: AccountId,
    options: &Options,
) -> Result<usize> {
    let missing = store.bodies_missing(account, "INBOX", options.prefetch_bodies)?;
    let mut stored = 0;
    for chunk in missing.chunks(PREFETCH_BATCH) {
        let fetched = match fetch::parallel(chunk, options.workers, |id| source.body(id)) {
            Ok(fetched) => fetched,
            Err(Error::Gmail(err)) if err.is_transient() => break,
            Err(err) => return Err(err),
        };
        let bodies: Vec<(String, String, Option<String>)> =
            fetched.found.into_iter().map(|(id, body)| (id, body.text, body.html)).collect();
        stored += store.set_bodies(account, &bodies)?;
    }
    Ok(stored)
}

/// Loads up to `backfill_batch` inbox messages older than the backfill position and moves the
/// position back past them. Returns how many were fetched and whether more may remain.
fn backfill(
    source: &impl MailSource,
    store: &mut Store,
    account: AccountId,
    options: &Options,
    progress: &mut dyn FnMut(usize),
) -> Result<(usize, bool)> {
    let (before, done) = store.backfill(account)?;
    // Databases from before backfill existed start from their oldest inbox message.
    let Some(before) = before.or(store.oldest_in(account, "INBOX")?) else { return Ok((0, false)) };
    if done {
        return Ok((0, false));
    }
    // Gmail's `before:` takes whole seconds and excludes that second. Asking for one second
    // later keeps messages from the same second; those already stored are skipped.
    let search = format!("before:{}", before.div_euclid(1000) + 1);
    let (ids, _) = source.list("INBOX", Some(&search), None, options.backfill_batch)?;
    let missing = store.missing(account, &ids)?;
    let mut fetched_count = 0;
    for chunk in missing.chunks(options.page_size) {
        let fetched = fetch::all(source, chunk, options.workers)?;
        let messages = fetched.messages();
        store.upsert_messages(account, &messages)?;
        fetched_count += messages.len();
        progress(fetched_count);
    }
    // Nothing new in this batch means Gmail has nothing older than what's stored.
    let done = missing.is_empty();
    let next = store.oldest_among(account, &ids)?.map_or(before, |oldest| oldest.min(before));
    store.set_backfill(account, Some(next), done)?;
    Ok((fetched_count, !done))
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
    let mut report = Report { full: true, ..Report::default() };
    let mut listed = 0;
    let mut page_token = None;
    while listed < options.inbox_limit {
        let want = options.page_size.min(options.inbox_limit - listed);
        let (ids, next) = source.list("INBOX", None, page_token.as_deref(), want)?;
        let fetched = fetch::all(source, &store.missing(account, &ids)?, options.workers)?;
        let messages = fetched.messages();
        store.upsert_messages(account, &messages)?;
        listed += ids.len();
        report.fetched += messages.len();
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
        let messages = fetched.messages();
        store.upsert_messages(account, &messages)?;
        deleted.extend(fetched.gone);
        report.fetched += messages.len();
        progress(report.fetched);
    }
    let deleted: Vec<String> = deleted.into_iter().collect();
    report.deleted = store.delete_messages(account, &deleted)?;
    store.set_history_id(account, &latest)?;
    Ok(report)
}

#[cfg(test)]
mod tests;
