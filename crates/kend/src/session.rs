//! A signed-in session: the UI's own database connection, the mailbox it shows and the sync
//! worker feeding the database.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use kend_gmail::{Gmail, MessageBody};
use kend_store::{AccountId, Label, LabelKind, Message, MessageSummary, Store};
use rune::{Latest, TextEdit, Waker};

use crate::compose::Compose;
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
    /// The search field.
    pub search: TextEdit,
    /// Matches for the search field's text, shown instead of the mailbox while it has any.
    pub results: Option<Vec<MessageSummary>>,
    /// Searches run on a worker with their own connection, the newest query winning.
    searches: Option<Searches>,
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
    /// The open message, when one is open.
    pub reading: Option<Reading>,
    /// A message being written. Shown over the list or the open message until sent or discarded.
    pub compose: Option<Compose>,
    /// A passing confirmation ("Message sent"), cleared by the next thing the user does.
    pub notice: Option<String>,
    /// The last archive or delete, for Undo.
    pub last_move: Option<Undo>,
    worker: Option<SyncWorker>,
    /// The Gmail client sync uses too, for fetching bodies and marking messages read.
    pub gmail: Option<Arc<Gmail>>,
}

/// Taking a message out of the inbox, or moving it to Trash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Archive,
    Trash,
}

/// What Undo needs: the message and the labels it had before.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Undo {
    pub gmail_id: String,
    pub previous: Vec<String>,
    pub action: Move,
}

struct Searches {
    db: Arc<Mutex<Store>>,
    latest: Latest<(String, Vec<MessageSummary>)>,
}

/// Search results shown at most.
const RESULTS: usize = 200;

pub struct Reading {
    pub message: Message,
    pub body: BodyState,
    /// The HTML body as Markdown, converted once when the message opens.
    pub markdown: Option<String>,
}

impl Reading {
    fn new(message: Message, body: BodyState) -> Self {
        let markdown = message.body_html.as_deref().map(crate::html::to_markdown).filter(|md| !md.trim().is_empty());
        Self { message, body, markdown }
    }
}

pub enum BodyState {
    /// In `message.body_text`.
    Ready,
    Loading,
    Failed(String),
}

/// What opening a message needs done off the UI thread.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Opened {
    /// The body isn't stored yet: fetch it.
    pub fetch_body: bool,
    /// It was unread: tell Gmail it's read now.
    pub mark_read: bool,
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
    pub fn open(
        id: u64,
        store: Store,
        email: &str,
        worker: Option<SyncWorker>,
        gmail: Option<Arc<Gmail>>,
    ) -> kend_store::Result<Self> {
        let account = store.upsert_account(email)?;
        let mut session = Self {
            id,
            store,
            account,
            mailbox: "INBOX".into(),
            rows: Vec::new(),
            search: TextEdit::new(""),
            results: None,
            searches: None,
            rows_version: 0,
            selected: None,
            total: 0,
            unread: HashMap::new(),
            labels: Vec::new(),
            labels_collapsed: false,
            sync: SyncStatus::Syncing { fetched: 0 },
            error: None,
            reading: None,
            compose: None,
            notice: None,
            last_move: None,
            worker,
            gmail,
        };
        session.reload();
        Ok(session)
    }

    pub fn unread_in(&self, label: &str) -> u64 {
        self.unread.get(label).copied().unwrap_or(0)
    }

    pub fn select(&mut self, mailbox: &str) {
        self.notice = None;
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

    /// What the list shows: search results while searching, else the mailbox.
    pub fn visible(&self) -> &[MessageSummary] {
        self.results.as_deref().unwrap_or(&self.rows)
    }

    pub fn query(&self) -> &str {
        self.search.text.trim()
    }

    /// Searches run off the UI thread on their own connection, waking the window with results.
    pub fn search_in_background(&mut self, db: Store, waker: Waker) {
        self.searches = Some(Searches { db: Arc::new(Mutex::new(db)), latest: Latest::new(waker) });
    }

    /// The search field changed: search for its text, or show the mailbox again when empty.
    pub fn search_changed(&mut self) {
        self.notice = None;
        self.reading = None;
        let query = self.query().to_string();
        if query.is_empty() {
            if let Some(searches) = &self.searches {
                searches.latest.cancel();
            }
            self.show_results(None);
            return;
        }
        let account = self.account;
        match &self.searches {
            Some(searches) => {
                let db = searches.db.clone();
                searches.latest.request(query, move |query| {
                    let store = db.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                    let results = store.search(account, &query, RESULTS).unwrap_or_default();
                    (query, results)
                });
            }
            // Without a worker (tests, snapshots), search right here.
            None => {
                let results = self.store.search(account, &query, RESULTS).unwrap_or_default();
                self.show_results(Some(results));
            }
        }
    }

    /// Lands a finished search if it's still for what the field says. True when it landed.
    pub fn poll_search(&mut self) -> bool {
        let Some((query, results)) = self.searches.as_ref().and_then(|s| s.latest.take()) else { return false };
        if query != self.query() {
            return false;
        }
        self.show_results(Some(results));
        true
    }

    pub fn clear_search(&mut self) {
        self.search = TextEdit::new("");
        self.search_changed();
    }

    fn show_results(&mut self, results: Option<Vec<MessageSummary>>) {
        self.results = results;
        self.selected = None;
        self.rows_version += 1;
    }

    /// Loads the next page when the list shows rows within half a page of the end. True when
    /// rows were added.
    pub fn load_more(&mut self, visible_end: usize) -> bool {
        let loaded = self.rows.len();
        if self.results.is_some() || visible_end + PAGE / 2 < loaded || loaded as u64 >= self.total {
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
        let current = self.selected.as_ref().and_then(|id| self.visible().iter().position(|m| &m.gmail_id == id));
        let next = match current {
            None => 0,
            Some(i) => i.saturating_add_signed(delta),
        };
        self.load_more(next);
        let next = next.min(self.visible().len().checked_sub(1)?);
        self.selected = Some(self.visible()[next].gmail_id.clone());
        self.selected.as_deref()
    }

    /// Opens a message: shown at once from the store, marked read locally straight away.
    pub fn open_message(&mut self, gmail_id: &str) -> Opened {
        self.notice = None;
        self.last_move = None;
        let Ok(Some(id)) = self.store.find(self.account, gmail_id) else { return Opened::default() };
        let Ok(Some(message)) = self.store.message(id) else { return Opened::default() };
        let mark_read = message.summary.unread && self.store.set_unread(self.account, gmail_id, false).unwrap_or(false);
        let fetch_body = message.body_text.is_none();
        let body = if fetch_body { BodyState::Loading } else { BodyState::Ready };
        self.selected = Some(gmail_id.to_string());
        self.reading = Some(Reading::new(message, body));
        if mark_read {
            self.reload();
        }
        Opened { fetch_body, mark_read }
    }

    pub fn close_message(&mut self) {
        self.reading = None;
    }

    /// Marks the open message's body as loading again, for a retry.
    pub fn retry_body(&mut self) -> Option<String> {
        let reading = self.reading.as_mut()?;
        reading.body = BodyState::Loading;
        Some(reading.message.summary.gmail_id.clone())
    }

    /// A fetched body: stored (so it opens instantly next time and is searchable) and shown if
    /// its message is still open.
    pub fn body_loaded(&mut self, gmail_id: &str, result: Result<Option<MessageBody>, String>) {
        let outcome = match result {
            Ok(Some(body)) => self
                .store
                .find(self.account, gmail_id)
                .and_then(|id| match id {
                    Some(id) => self.store.set_body(id, Some(&body.text), body.html.as_deref()).map(|_| Some(id)),
                    None => Ok(None),
                })
                .map_err(|e| e.to_string()),
            Ok(None) => Err("This message no longer exists in Gmail.".to_string()),
            Err(err) => Err(err),
        };
        let Some(reading) = self.reading.as_mut().filter(|r| r.message.summary.gmail_id == gmail_id) else { return };
        match outcome {
            Ok(Some(id)) => {
                if let Ok(Some(message)) = self.store.message(id) {
                    *reading = Reading::new(message, BodyState::Ready);
                } else {
                    reading.body = BodyState::Ready;
                }
            }
            Ok(None) => reading.body = BodyState::Failed("This message is no longer stored.".into()),
            Err(err) => reading.body = BodyState::Failed(err),
        }
    }

    /// The message an action applies to: the open one, else the selected row.
    pub fn target(&self) -> Option<&str> {
        self.reading.as_ref().map(|r| r.message.summary.gmail_id.as_str()).or(self.selected.as_deref())
    }

    /// Archives or trashes the target locally, at once. The selection moves to the row that
    /// takes its place, so the next key acts on the next message. Returns the message's Gmail
    /// id when something changed; the caller tells Gmail.
    pub fn move_message(&mut self, action: Move) -> Option<String> {
        let gmail_id = self.target()?.to_string();
        let id = self.store.find(self.account, &gmail_id).ok()??;
        let previous = self.store.message(id).ok()??.label_ids;
        let mut labels = previous.clone();
        match action {
            Move::Archive => labels.retain(|l| l != "INBOX"),
            Move::Trash if !labels.iter().any(|l| l == "TRASH") => labels.push("TRASH".into()),
            Move::Trash => {}
        }
        if labels == previous {
            return None;
        }
        let index = self.visible().iter().position(|m| m.gmail_id == gmail_id);
        self.store.set_labels(self.account, &gmail_id, &labels).ok()?;
        if action == Move::Trash
            && let Some(results) = &mut self.results
        {
            // Search covers all mail, so only a trashed message leaves the results.
            results.retain(|m| m.gmail_id != gmail_id);
        }
        self.reading = None;
        self.reload();
        let visible = self.visible();
        self.selected =
            index.and_then(|i| visible.get(i.min(visible.len().checked_sub(1)?))).map(|m| m.gmail_id.clone());
        self.notice = Some(match action {
            Move::Archive => "Archived".into(),
            Move::Trash => "Moved to Trash".into(),
        });
        self.last_move = Some(Undo { gmail_id: gmail_id.clone(), previous, action });
        Some(gmail_id)
    }

    /// Puts the last moved message back as it was, locally. Returns what to undo in Gmail.
    pub fn undo_move(&mut self) -> Option<Undo> {
        let undo = self.last_move.take()?;
        self.store.set_labels(self.account, &undo.gmail_id, &undo.previous).ok()?;
        self.notice = None;
        if self.results.is_some() {
            self.search_changed();
        }
        self.reload();
        self.selected = Some(undo.gmail_id.clone());
        Some(undo)
    }

    /// Gmail refused a move: put the message back and say why.
    pub fn move_failed(&mut self, gmail_id: &str, previous: &[String], error: String) {
        let _ = self.store.set_labels(self.account, gmail_id, previous);
        if self.last_move.as_ref().is_some_and(|u| u.gmail_id == gmail_id) {
            self.last_move = None;
            self.notice = None;
        }
        self.error = Some(format!("Couldn't move the message: {error}"));
        self.reload();
    }

    /// Gmail didn't take the read mark: show the message as unread again, and why.
    pub fn mark_read_failed(&mut self, gmail_id: &str, error: String) {
        let _ = self.store.set_unread(self.account, gmail_id, true);
        self.error = Some(format!("Couldn't mark the message read: {error}"));
        self.reload();
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
