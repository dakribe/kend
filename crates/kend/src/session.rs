//! A signed-in session: the UI's own database connection, the inbox rows it shows and the
//! sync worker feeding the database.

use crate::sync_worker::{SyncUpdate, SyncWorker};
use kend_store::{AccountId, MessageSummary, Store};

/// How many inbox rows the list shows.
const PAGE: usize = 100;

pub struct Session {
    pub id: u64,
    store: Store,
    account: AccountId,
    pub rows: Vec<MessageSummary>,
    pub total: u64,
    pub unread: u64,
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
    pub fn open(
        id: u64,
        store: Store,
        email: &str,
        worker: Option<SyncWorker>,
    ) -> kend_store::Result<Self> {
        let account = store.upsert_account(email)?;
        let mut session = Self {
            id,
            store,
            account,
            rows: Vec::new(),
            total: 0,
            unread: 0,
            sync: SyncStatus::Syncing { fetched: 0 },
            worker,
        };
        session.reload();
        Ok(session)
    }

    pub fn reload(&mut self) {
        if let Ok(rows) = self.store.list(self.account, "INBOX", None, PAGE) {
            self.rows = rows;
        }
        if let Ok(total) = self.store.count(self.account, "INBOX") {
            self.total = total;
        }
        if let Ok(unread) = self.store.unread_count(self.account, "INBOX") {
            self.unread = unread;
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
