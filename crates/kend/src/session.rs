//! A signed-in session: the UI's own database connection, the mailbox it shows and the sync
//! worker feeding the database.

use std::collections::HashMap;

use kend_store::{AccountId, Label, LabelKind, MessageSummary, Store};

use crate::sync_worker::{SyncUpdate, SyncWorker};

/// Rows read from the database at a time. The list loads the next page as you near the end.
const PAGE: usize = 200;

pub struct Session {
    pub id: u64,
    store: Store,
    account: AccountId,
    /// The selected mailbox's Gmail label id.
    pub mailbox: String,
    /// The mailbox's newest rows, as far down as the list has scrolled.
    pub rows: Vec<MessageSummary>,
    /// Bumped whenever `rows` is replaced rather than extended, so the list re-reads its keys.
    pub rows_version: u64,
    /// The selected message's Gmail id.
    pub selected: Option<String>,
    /// Messages in the selected mailbox.
    pub total: u64,
    /// Unread messages per label id.
    pub unread: HashMap<String, u64>,
    /// The user's own labels, sorted so nested ones follow their parent.
    pub labels: Vec<Label>,
    pub labels_collapsed: bool,
    pub sync: SyncStatus,
    /// Why the last sync failed. Stays until a sync succeeds, so it can be read.
    pub error: Option<String>,
    worker: Option<SyncWorker>,
}

pub enum SyncStatus {
    Syncing {
        fetched: usize,
    },
    Synced,
    /// Up to date with new mail, still loading older mail in the background.
    LoadingOlder,
}

impl Session {
    /// Reads whatever mail is already stored, so a returning user sees their inbox at once.
    pub fn open(id: u64, store: Store, email: &str, worker: Option<SyncWorker>) -> kend_store::Result<Self> {
        let account = store.upsert_account(email)?;
        let mut session = Self {
            id,
            store,
            account,
            mailbox: "INBOX".into(),
            rows: Vec::new(),
            rows_version: 0,
            selected: None,
            total: 0,
            unread: HashMap::new(),
            labels: Vec::new(),
            labels_collapsed: false,
            sync: SyncStatus::Syncing { fetched: 0 },
            error: None,
            worker,
        };
        session.reload();
        Ok(session)
    }

    pub fn unread_in(&self, label: &str) -> u64 {
        self.unread.get(label).copied().unwrap_or(0)
    }

    pub fn select(&mut self, mailbox: &str) {
        if self.mailbox != mailbox {
            self.mailbox = mailbox.to_string();
            self.rows.clear();
            self.selected = None;
            self.reload();
        }
    }

    /// Re-reads the selected mailbox (as many rows as are loaded), the unread counts and the
    /// labels. All local: about a millisecond for a few hundred rows.
    pub fn reload(&mut self) {
        let loaded = self.rows.len().max(PAGE);
        if let Ok(rows) = self.store.list(self.account, &self.mailbox, None, loaded) {
            self.rows = rows;
            self.rows_version += 1;
        }
        if let Ok(total) = self.store.count(self.account, &self.mailbox) {
            self.total = total;
        }
        if let Ok(unread) = self.store.unread_counts(self.account) {
            self.unread = unread;
        }
        if let Ok(labels) = self.store.labels(self.account) {
            let mut labels: Vec<Label> = labels.into_iter().filter(|l| l.kind == LabelKind::User).collect();
            labels.sort_by_key(|l| l.name.to_lowercase());
            self.labels = labels;
        }
    }

    /// Loads the next page when the list shows rows within half a page of the end. True when
    /// rows were added.
    pub fn load_more(&mut self, visible_end: usize) -> bool {
        let loaded = self.rows.len();
        if visible_end + PAGE / 2 < loaded || loaded as u64 >= self.total {
            return false;
        }
        let after = self.rows.last().map(MessageSummary::cursor);
        match self.store.list(self.account, &self.mailbox, after, PAGE) {
            Ok(more) if !more.is_empty() => {
                self.rows.extend(more);
                true
            }
            _ => false,
        }
    }

    /// Moves the selection `delta` rows (down when positive), starting at the top when nothing
    /// is selected. Returns the newly selected message's id.
    pub fn move_selection(&mut self, delta: isize) -> Option<&str> {
        let current = self.selected.as_ref().and_then(|id| self.rows.iter().position(|m| &m.gmail_id == id));
        let next = match current {
            None => 0,
            Some(i) => i.saturating_add_signed(delta),
        };
        self.load_more(next);
        let next = next.min(self.rows.len().checked_sub(1)?);
        self.selected = Some(self.rows[next].gmail_id.clone());
        self.selected.as_deref()
    }

    /// Applies a worker update. Returns true when Google signed the account out.
    pub fn apply(&mut self, update: SyncUpdate) -> bool {
        match update {
            // While older mail loads, each round's progress keeps saying so.
            SyncUpdate::Progress(_) if matches!(self.sync, SyncStatus::LoadingOlder) => {}
            SyncUpdate::Progress(fetched) => self.sync = SyncStatus::Syncing { fetched },
            SyncUpdate::Finished(Ok(report)) => {
                self.error = None;
                self.sync = if report.more_to_load { SyncStatus::LoadingOlder } else { SyncStatus::Synced }
            }
            SyncUpdate::Finished(Err(err)) => self.error = Some(err),
            SyncUpdate::SignedOut => return true,
        }
        self.reload();
        false
    }

    pub fn sync_now(&mut self) {
        if let Some(worker) = &self.worker {
            self.sync = SyncStatus::Syncing { fetched: 0 };
            worker.sync_now();
        }
    }

    #[cfg(test)]
    pub fn store_mut(&mut self) -> &mut Store {
        &mut self.store
    }

    #[cfg(test)]
    pub fn account(&self) -> AccountId {
        self.account
    }
}
