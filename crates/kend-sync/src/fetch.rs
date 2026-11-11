//! Fetching in parallel on scoped threads: message metadata for sync, bodies for prefetch.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;

use kend_store::NewMessage;

use crate::{MailSource, Result};

pub(crate) struct Fetched<T> {
    /// The ids found, with what was fetched, in the order the ids were given.
    pub found: Vec<(String, T)>,
    /// Ids that no longer exist on the server.
    pub gone: Vec<String>,
}

impl Fetched<NewMessage> {
    pub fn messages(&self) -> Vec<NewMessage> {
        self.found.iter().map(|(_, m)| m.clone()).collect()
    }
}

/// Message metadata for `ids`.
pub(crate) fn all(source: &impl MailSource, ids: &[String], workers: usize) -> Result<Fetched<NewMessage>> {
    parallel(ids, workers, |id| source.message(id))
}

/// Runs `fetch` for each id with up to `workers` in flight. The first error stops the others
/// from starting new requests and is returned.
pub(crate) fn parallel<T: Send>(
    ids: &[String],
    workers: usize,
    fetch: impl Fn(&str) -> kend_gmail::Result<Option<T>> + Sync,
) -> Result<Fetched<T>> {
    let next = AtomicUsize::new(0);
    let failed = AtomicBool::new(false);
    let mut results: Vec<(usize, kend_gmail::Result<Option<T>>)> = thread::scope(|scope| {
        let handles: Vec<_> = (0..workers.clamp(1, ids.len().max(1)))
            .map(|_| {
                scope.spawn(|| {
                    let mut mine = Vec::new();
                    while !failed.load(Ordering::Relaxed) {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(id) = ids.get(i) else { break };
                        let result = fetch(id);
                        if result.is_err() {
                            failed.store(true, Ordering::Relaxed);
                        }
                        mine.push((i, result));
                    }
                    mine
                })
            })
            .collect();
        handles.into_iter().flat_map(|h| h.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic))).collect()
    });
    results.sort_by_key(|(i, _)| *i);

    let mut fetched = Fetched { found: Vec::with_capacity(ids.len()), gone: Vec::new() };
    for (i, result) in results {
        match result? {
            Some(value) => fetched.found.push((ids[i].clone(), value)),
            None => fetched.gone.push(ids[i].clone()),
        }
    }
    Ok(fetched)
}
