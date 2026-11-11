//! A signed-in session: the UI's own database connection, the mailbox it shows and the sync
//! worker feeding the database.

use std::collections::HashMap;

use kend_store::{AccountId, Label, LabelKind, MessageSummary, Store};

use crate::sync_worker::{SyncUpdate, SyncWorker};

/// How many rows the message list shows.
const PAGE: usize = 100;

pub struct Session {
    pub id: u64,
    store: Store,
    account: AccountId,
    /// The selected mailbox's Gmail label id.
    pub mailbox: String,
    pub rows: Vec<MessageSummary>,
    /// Messages in the selected mailbox.
    pub total: u64,
    /// Unread messages per label id.
    pub unread: HashMap<String, u64>,
    /// The user's own labels, sorted so nested ones follow their parent.
    pub labels: Vec<Label>,
    pub labels_collapsed: bool,
    pub sync: SyncStatus,
    worker: Option<SyncWorker>,
}

pub enum SyncStatus {
    Syncing { fetched: usize },
    Synced,
    Failed(String),
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
            total: 0,
            unread: HashMap::new(),
            labels: Vec::new(),
            labels_collapsed: false,
            sync: SyncStatus::Syncing { fetched: 0 },
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
            self.reload();
        }
    }

    /// Re-reads the selected mailbox, the unread counts and the labels. All local, ~1 ms.
    pub fn reload(&mut self) {
        if let Ok(rows) = self.store.list(self.account, &self.mailbox, None, PAGE) {
            self.rows = rows;
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

    /// Applies a worker update. Returns true when Google signed the account out.
    pub fn apply(&mut self, update: SyncUpdate) -> bool {
        match update {
            SyncUpdate::Progress(fetched) => self.sync = SyncStatus::Syncing { fetched },
            SyncUpdate::Finished(Ok(_)) => self.sync = SyncStatus::Synced,
            SyncUpdate::Finished(Err(err)) => self.sync = SyncStatus::Failed(err),
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
