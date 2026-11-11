//! Fetching a page of messages in parallel on scoped threads.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;

use kend_store::NewMessage;

use crate::{MailSource, Result};

pub(crate) struct Fetched {
    /// In the order the ids were given.
    pub messages: Vec<NewMessage>,
    /// Ids that no longer exist on the server.
    pub gone: Vec<String>,
}

/// Fetches `ids` with up to `workers` requests in flight. The first error stops the others
/// from starting new requests and is returned.
pub(crate) fn all(source: &impl MailSource, ids: &[String], workers: usize) -> Result<Fetched> {
    let next = AtomicUsize::new(0);
    let failed = AtomicBool::new(false);
    let mut results: Vec<(usize, kend_gmail::Result<Option<NewMessage>>)> =
        thread::scope(|scope| {
            let handles: Vec<_> = (0..workers.clamp(1, ids.len().max(1)))
                .map(|_| {
                    scope.spawn(|| {
                        let mut mine = Vec::new();
                        while !failed.load(Ordering::Relaxed) {
                            let i = next.fetch_add(1, Ordering::Relaxed);
                            let Some(id) = ids.get(i) else { break };
                            let result = source.message(id);
                            if result.is_err() {
                                failed.store(true, Ordering::Relaxed);
                            }
                            mine.push((i, result));
                        }
                        mine
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|h| {
                    h.join()
                        .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
                })
                .collect()
        });
    results.sort_by_key(|(i, _)| *i);

    let mut fetched = Fetched {
        messages: Vec::with_capacity(ids.len()),
        gone: Vec::new(),
    };
    for (i, result) in results {
        match result? {
            Some(message) => fetched.messages.push(message),
            None => fetched.gone.push(ids[i].clone()),
        }
    }
    Ok(fetched)
}
