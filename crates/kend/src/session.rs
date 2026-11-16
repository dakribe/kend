//! A signed-in session: the UI's own database connection, the mailbox it shows, and the
//! signed-in accounts with the sync workers feeding the database. The list shows one account,
//! or every account's mail together.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use kend_gmail::{Gmail, MessageBody};
use kend_store::{AccountId, Address, Cursor, Label, LabelKind, Message, MessageSummary, Store};
use rune::{Latest, TextEdit, Waker};

use crate::compose::Compose;
use crate::sync_worker::{SyncUpdate, SyncWorker};

/// Rows read from the database at a time. The list loads the next page as you near the end.
const PAGE: usize = 200;

pub struct Session {
    pub id: u64,
    store: Store,
    /// The signed-in accounts, in the order they were added.
    pub accounts: Vec<Linked>,
    /// The account whose mail the list shows, or `None` for every account's together.
    pub scope: Option<AccountId>,
    /// The selected mailbox's Gmail label id.
    pub mailbox: String,
    /// Everything from one sender, shown instead of the mailbox until left.
    pub sender: Option<Sender>,
    /// The mailbox's (or sender's) newest rows, as far down as the list has scrolled.
    pub rows: Vec<MessageSummary>,
    /// The search field.
    pub search: TextEdit,
    /// Matches for the search field's text, shown instead of the mailbox while it has any.
    pub results: Option<Vec<MessageSummary>>,
    /// Searches run on a worker with their own connection, the newest query winning.
    searches: Option<Searches>,
    /// Bumped whenever `rows` is replaced rather than extended, so the list re-reads its keys.
    pub rows_version: u64,
    /// The selected message's Gmail id: the row the keyboard is on.
    pub selected: Option<String>,
    /// Messages checked for a bulk action (X, ⌘-click, Shift-click). Archive and delete act on
    /// these when there are any.
    pub checked: HashSet<String>,
    /// Where a Shift-click range starts: the last row checked or unchecked on its own.
    anchor: Option<String>,
    /// Messages in the selected mailbox, or from the sender being shown.
    pub total: u64,
    /// Unread messages per label id.
    pub unread: HashMap<String, u64>,
    /// The user's own labels, sorted so nested ones follow their parent. Empty while several
    /// accounts show together, since each account numbers its labels its own way.
    pub labels: Vec<Label>,
    pub labels_collapsed: bool,
    /// Why the last change to Gmail failed. Stays until a sync succeeds, so it can be read.
    pub error: Option<String>,
    /// The open message, when one is open.
    pub reading: Option<Reading>,
    /// A message being written. Shown over the list or the open message until sent or discarded.
    pub compose: Option<Compose>,
    /// A passing confirmation ("Message sent"), cleared by the next thing the user does.
    pub notice: Option<String>,
    /// The last archive or delete, for Undo.
    pub last_move: Option<Undo>,
    /// The draft last discarded, for Undo. Cleared by the next archive or delete, so Undo
    /// always takes back the latest thing.
    pub discarded: Option<Compose>,
}

/// A signed-in account: its sync, and the Gmail client actions on its mail go through.
pub struct Linked {
    pub id: AccountId,
    pub email: String,
    pub sync: SyncStatus,
    /// Why this account's last sync failed. Stays until a sync succeeds, so it can be read.
    pub error: Option<String>,
    /// Unread messages in this account's inbox, for the account switcher.
    pub inbox_unread: u64,
    /// The Gmail client sync uses too, for fetching bodies and marking messages read.
    pub gmail: Option<Arc<Gmail>>,
    /// Dropping it stops this account's sync.
    worker: Option<SyncWorker>,
}

/// Taking a message out of the inbox, or moving it to Trash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Archive,
    Trash,
}

/// Starring and read state, changed on the targets the way archive is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Star,
    Unstar,
    Read,
    Unread,
}

impl Mark {
    /// The Gmail label this adds or removes.
    pub fn label(self) -> &'static str {
        match self {
            Mark::Star | Mark::Unstar => "STARRED",
            Mark::Read | Mark::Unread => "UNREAD",
        }
    }

    pub fn adds(self) -> bool {
        matches!(self, Mark::Star | Mark::Unread)
    }
}

/// A moved message and the labels it had before, to put it back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Moved {
    pub account: AccountId,
    pub gmail_id: String,
    pub previous: Vec<String>,
}

/// What Undo needs: the messages one archive or delete moved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Undo {
    pub messages: Vec<Moved>,
    pub action: Move,
}

/// The sender whose mail the list shows, across every mailbox but Trash and Spam.
pub struct Sender {
    pub address: Address,
    pub unread: u64,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
        let mut session = Self {
            id,
            store,
            accounts: Vec::new(),
            scope: None,
            mailbox: "INBOX".into(),
            sender: None,
            rows: Vec::new(),
            search: TextEdit::new(""),
            results: None,
            searches: None,
            rows_version: 0,
            selected: None,
            checked: HashSet::new(),
            anchor: None,
            total: 0,
            unread: HashMap::new(),
            labels: Vec::new(),
            labels_collapsed: false,
            error: None,
            reading: None,
            compose: None,
            notice: None,
            last_move: None,
            discarded: None,
        };
        session.add_account(email, worker, gmail)?;
        Ok(session)
    }

    /// Adds a signed-in account, its stored mail showing at once. Adding one already here
    /// replaces its sync and client.
    pub fn add_account(
        &mut self,
        email: &str,
        worker: Option<SyncWorker>,
        gmail: Option<Arc<Gmail>>,
    ) -> kend_store::Result<AccountId> {
        let id = self.store.upsert_account(email)?;
        let linked = Linked {
            id,
            email: email.to_string(),
            sync: SyncStatus::Syncing { fetched: 0 },
            error: None,
            inbox_unread: 0,
            gmail,
            worker,
        };
        match self.accounts.iter_mut().find(|a| a.id == id) {
            Some(existing) => *existing = linked,
            None => self.accounts.push(linked),
        }
        self.reload();
        Ok(id)
    }

    /// Stops an account's sync and takes its mail out of the list. Deleting its stored mail is
    /// up to the caller. Returns its id, if it was here.
    pub fn remove_account(&mut self, email: &str) -> Option<AccountId> {
        let i = self.accounts.iter().position(|a| a.email == email)?;
        let id = self.accounts.remove(i).id;
        if self.scope == Some(id) {
            self.scope = None;
        }
        if self.reading.as_ref().is_some_and(|r| r.message.summary.account == id) {
            self.reading = None;
        }
        if self.compose.as_ref().is_some_and(|c| c.account == Some(id)) {
            self.compose = None;
        }
        if self.results.is_some() {
            self.search_changed();
        }
        self.last_move = None;
        self.switch_list(None);
        Some(id)
    }

    /// Shows one account's mail, or every account's together (`None`). Labels belong to one
    /// account, so a label that isn't the new account's falls back to the inbox.
    pub fn select_account(&mut self, scope: Option<AccountId>) {
        self.notice = None;
        if self.scope == scope {
            return;
        }
        self.scope = scope;
        self.reading = None;
        if self.mailbox.starts_with("Label_") {
            self.mailbox = "INBOX".into();
        }
        if self.results.is_some() {
            self.search_changed();
        }
        self.switch_list(None);
    }

    /// The accounts whose mail the list shows.
    pub fn scope_ids(&self) -> Vec<AccountId> {
        match self.scope {
            Some(id) => vec![id],
            None => self.accounts.iter().map(|a| a.id).collect(),
        }
    }

    /// The one account the list shows, when it shows only one: the selected account, or the
    /// only one signed in.
    pub fn single(&self) -> Option<&Linked> {
        match self.scope {
            Some(id) => self.linked(id),
            None if self.accounts.len() == 1 => self.accounts.first(),
            None => None,
        }
    }

    /// Whether several accounts' mail shows together.
    pub fn combined(&self) -> bool {
        self.single().is_none()
    }

    pub fn linked(&self, account: AccountId) -> Option<&Linked> {
        self.accounts.iter().find(|a| a.id == account)
    }

    /// The Gmail client for each account that has one.
    pub fn clients(&self) -> HashMap<AccountId, Arc<Gmail>> {
        self.accounts.iter().filter_map(|a| Some((a.id, a.gmail.clone()?))).collect()
    }

    /// Where sync stands for the accounts shown: syncing while any of them is.
    pub fn sync_status(&self) -> SyncStatus {
        let shown = self.scope_ids();
        let mut status = SyncStatus::Synced;
        for a in self.accounts.iter().filter(|a| shown.contains(&a.id)) {
            status = match (status, a.sync) {
                (SyncStatus::Syncing { fetched: x }, SyncStatus::Syncing { fetched: y }) => {
                    SyncStatus::Syncing { fetched: x + y }
                }
                (SyncStatus::Syncing { .. }, _) => status,
                (_, SyncStatus::Syncing { .. }) => a.sync,
                (SyncStatus::LoadingOlder, _) | (_, SyncStatus::LoadingOlder) => SyncStatus::LoadingOlder,
                _ => SyncStatus::Synced,
            };
        }
        status
    }

    /// What went wrong, if anything: a refused change, else a failed sync of an account shown.
    /// With several accounts, a sync error says which account it's about.
    pub fn shown_error(&self) -> Option<String> {
        if let Some(error) = &self.error {
            return Some(error.clone());
        }
        let shown = self.scope_ids();
        let failed = self.accounts.iter().find(|a| shown.contains(&a.id) && a.error.is_some())?;
        let error = failed.error.as_deref().unwrap_or_default();
        Some(if self.accounts.len() > 1 { format!("{}: {error}", failed.email) } else { error.to_string() })
    }

    /// The account a listed, open or stored message belongs to.
    pub fn account_of(&self, gmail_id: &str) -> Option<AccountId> {
        if let Some(reading) = self.reading.as_ref().filter(|r| r.message.summary.gmail_id == gmail_id) {
            return Some(reading.message.summary.account);
        }
        let listed = self.visible().iter().chain(&self.rows).find(|m| m.gmail_id == gmail_id);
        if let Some(m) = listed {
            return Some(m.account);
        }
        self.accounts.iter().map(|a| a.id).find(|&a| self.store.find(a, gmail_id).ok().flatten().is_some())
    }

    pub fn unread_in(&self, label: &str) -> u64 {
        self.unread.get(label).copied().unwrap_or(0)
    }

    pub fn select(&mut self, mailbox: &str) {
        self.notice = None;
        if self.mailbox != mailbox || self.sender.is_some() {
            self.mailbox = mailbox.to_string();
            self.sender = None;
            self.rows.clear();
            self.selected = None;
            self.clear_checked();
            self.reload();
        }
    }

    /// Re-reads the selected mailbox or sender (as many rows as are loaded), the unread counts
    /// and the labels. All local: about a millisecond for a few hundred rows.
    pub fn reload(&mut self) {
        let loaded = self.rows.len().max(PAGE);
        if let Ok(rows) = self.page(None, loaded) {
            self.rows = rows;
            self.rows_version += 1;
        }
        let shown = self.scope_ids();
        match &mut self.sender {
            Some(sender) => {
                if let Ok((total, unread)) = self.store.count_from_in(&shown, &sender.address.email) {
                    self.total = total;
                    sender.unread = unread;
                }
            }
            None => {
                if let Ok(total) = self.store.count_in(&shown, &self.mailbox) {
                    self.total = total;
                }
            }
        }
        // Every account's counts, for the switcher's badges; the shown ones add up for the rest.
        let mut unread: HashMap<String, u64> = HashMap::new();
        for account in &mut self.accounts {
            let Ok(counts) = self.store.unread_counts(account.id) else { continue };
            account.inbox_unread = counts.get("INBOX").copied().unwrap_or(0);
            if shown.contains(&account.id) {
                for (label, n) in counts {
                    *unread.entry(label).or_default() += n;
                }
            }
        }
        self.unread = unread;
        self.labels = match self.single().map(|a| a.id) {
            Some(account) => {
                let mut labels: Vec<Label> = self
                    .store
                    .labels(account)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|l| l.kind == LabelKind::User)
                    .collect();
                labels.sort_by_key(|l| l.name.to_lowercase());
                labels
            }
            None => Vec::new(),
        };
    }

    /// One page of what the list shows when not searching: the sender's mail, else the mailbox.
    fn page(&self, after: Option<Cursor>, limit: usize) -> kend_store::Result<Vec<MessageSummary>> {
        let shown = self.scope_ids();
        match &self.sender {
            Some(sender) => self.store.list_from_in(&shown, &sender.address.email, after, limit),
            None => self.store.list_in(&shown, &self.mailbox, after, limit),
        }
    }

    /// The sender of the open message, else of the selected row.
    pub fn focused_sender(&self) -> Option<&Address> {
        if let Some(reading) = &self.reading {
            return Some(&reading.message.summary.from);
        }
        let selected = self.selected.as_ref()?;
        self.visible().iter().find(|m| &m.gmail_id == selected).map(|m| &m.from)
    }

    /// Shows everything from `address` in place of the mailbox. The message you came from stays
    /// selected, so you can see where it sits among the rest.
    pub fn show_sender(&mut self, address: Address) {
        if address.email.is_empty() {
            return;
        }
        let came_from = self.reading.as_ref().map(|r| r.message.summary.gmail_id.clone()).or(self.selected.clone());
        if !self.search.text.is_empty() {
            self.clear_search();
        }
        self.notice = None;
        self.reading = None;
        self.sender = Some(Sender { address, unread: 0 });
        self.switch_list(came_from);
    }

    /// Back from a sender's mail to the mailbox.
    pub fn leave_sender(&mut self) {
        if self.sender.take().is_some() {
            self.notice = None;
            let came_from = self.selected.clone();
            self.switch_list(came_from);
        }
    }

    /// Re-reads the list for a new source, keeping `selected` when it's still listed.
    fn switch_list(&mut self, selected: Option<String>) {
        self.rows.clear();
        self.clear_checked();
        self.reload();
        self.selected = selected.filter(|id| self.rows.iter().any(|m| &m.gmail_id == id));
    }

    /// What the list shows: search results while searching, else the mailbox or sender.
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
        let accounts = self.scope_ids();
        match &self.searches {
            Some(searches) => {
                let db = searches.db.clone();
                searches.latest.request(query, move |query| {
                    let store = db.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                    let results = store.search_in(&accounts, &query, RESULTS).unwrap_or_default();
                    (query, results)
                });
            }
            // Without a worker (tests, snapshots), search right here.
            None => {
                let results = self.store.search_in(&accounts, &query, RESULTS).unwrap_or_default();
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
        self.clear_checked();
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
        match self.page(after, PAGE) {
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

    /// Selects the first row, or the last one (loading every remaining page) when `last`.
    /// Returns the newly selected message's id.
    pub fn select_edge(&mut self, last: bool) -> Option<&str> {
        if last {
            while self.load_more(self.rows.len()) {}
        }
        let visible = self.visible();
        let edge = if last { visible.last() } else { visible.first() }?;
        self.selected = Some(edge.gmail_id.clone());
        self.selected.as_deref()
    }

    /// Checks or unchecks one message, and starts the next Shift-click range there.
    pub fn toggle_checked(&mut self, gmail_id: &str) {
        self.notice = None;
        if !self.checked.remove(gmail_id) {
            self.checked.insert(gmail_id.to_string());
        }
        self.selected = Some(gmail_id.to_string());
        self.anchor = Some(gmail_id.to_string());
    }

    /// Checks every row from the anchor (or the selected row) to `gmail_id`, as Shift-click does.
    pub fn check_range(&mut self, gmail_id: &str) {
        self.notice = None;
        let position =
            |id: &Option<String>| id.as_ref().and_then(|id| self.visible().iter().position(|m| &m.gmail_id == id));
        let Some(end) = self.visible().iter().position(|m| m.gmail_id == gmail_id) else { return };
        let start = position(&self.anchor).or(position(&self.selected)).unwrap_or(end);
        let ids: Vec<String> =
            self.visible()[start.min(end)..=start.max(end)].iter().map(|m| m.gmail_id.clone()).collect();
        self.checked.extend(ids);
        if self.anchor.is_none() {
            self.anchor = Some(self.visible()[start].gmail_id.clone());
        }
        self.selected = Some(gmail_id.to_string());
    }

    /// Checks the selected row and the one `delta` away, then selects that one: Shift+J/K.
    pub fn extend_selection(&mut self, delta: isize) -> Option<&str> {
        self.notice = None;
        let Some(from) = self.selected.clone() else { return self.move_selection(delta) };
        self.checked.insert(from.clone());
        self.anchor.get_or_insert(from);
        let to = self.move_selection(delta)?.to_string();
        self.checked.insert(to);
        self.selected.as_deref()
    }

    /// Checks every loaded row in the list.
    pub fn check_all(&mut self) {
        self.notice = None;
        let ids: Vec<String> = self.visible().iter().map(|m| m.gmail_id.clone()).collect();
        self.checked.extend(ids);
    }

    pub fn clear_checked(&mut self) {
        self.checked.clear();
        self.anchor = None;
    }

    /// Opens a message: shown at once from the store, marked read locally straight away.
    pub fn open_message(&mut self, gmail_id: &str) -> Opened {
        self.notice = None;
        self.last_move = None;
        let Some(account) = self.account_of(gmail_id) else { return Opened::default() };
        let Ok(Some(id)) = self.store.find(account, gmail_id) else { return Opened::default() };
        let Ok(Some(message)) = self.store.message(id) else { return Opened::default() };
        let mark_read = message.summary.unread && self.store.set_unread(account, gmail_id, false).unwrap_or(false);
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
    pub fn body_loaded(&mut self, account: AccountId, gmail_id: &str, result: Result<Option<MessageBody>, String>) {
        let outcome = match result {
            Ok(Some(body)) => self
                .store
                .find(account, gmail_id)
                .and_then(|id| match id {
                    Some(id) => self.store.set_body(id, Some(&body.text), body.html.as_deref()).map(|_| Some(id)),
                    None => Ok(None),
                })
                .map_err(|e| e.to_string()),
            Ok(None) => Err("This message no longer exists in Gmail.".to_string()),
            Err(err) => Err(err),
        };
        let open = |r: &&mut Reading| r.message.summary.gmail_id == gmail_id && r.message.summary.account == account;
        let Some(reading) = self.reading.as_mut().filter(open) else { return };
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

    /// The messages an action applies to: the open one, else the checked rows (in list
    /// order), else the selected row.
    pub fn targets(&self) -> Vec<String> {
        if let Some(reading) = &self.reading {
            return vec![reading.message.summary.gmail_id.clone()];
        }
        if !self.checked.is_empty() {
            return self
                .visible()
                .iter()
                .filter(|m| self.checked.contains(&m.gmail_id))
                .map(|m| m.gmail_id.clone())
                .collect();
        }
        self.selected.iter().cloned().collect()
    }

    /// Archives or trashes the targets locally, at once. The selection moves to the row that
    /// takes the first one's place, so the next key acts on the next message. Returns what
    /// changed; the caller tells Gmail.
    pub fn move_messages(&mut self, action: Move) -> Vec<Moved> {
        let targets = self.targets();
        let first = targets.first().and_then(|id| self.visible().iter().position(|m| &m.gmail_id == id));
        let mut moved = Vec::new();
        for gmail_id in targets {
            let Some((account, previous)) = self.labels_of(&gmail_id) else { continue };
            let mut labels = previous.clone();
            match action {
                Move::Archive => labels.retain(|l| l != "INBOX"),
                Move::Trash if !labels.iter().any(|l| l == "TRASH") => labels.push("TRASH".into()),
                Move::Trash => {}
            }
            if labels != previous && self.store.set_labels(account, &gmail_id, &labels).unwrap_or(false) {
                moved.push(Moved { account, gmail_id, previous });
            }
        }
        if moved.is_empty() {
            return moved;
        }
        if action == Move::Trash
            && let Some(results) = &mut self.results
        {
            // Search covers all mail, so only trashed messages leave the results.
            results.retain(|m| !moved.iter().any(|x| x.gmail_id == m.gmail_id));
        }
        self.reading = None;
        // Moved messages leave the selection; any others stay checked.
        self.checked.retain(|id| !moved.iter().any(|m| &m.gmail_id == id));
        if self.anchor.as_ref().is_some_and(|a| !self.checked.contains(a)) {
            self.anchor = None;
        }
        self.reload();
        let visible = self.visible();
        self.selected =
            first.and_then(|i| visible.get(i.min(visible.len().checked_sub(1)?))).map(|m| m.gmail_id.clone());
        self.notice = Some(match (action, moved.len()) {
            (Move::Archive, 1) => "Archived".into(),
            (Move::Trash, 1) => "Moved to Trash".into(),
            (Move::Archive, n) => format!("Archived {n} messages"),
            (Move::Trash, n) => format!("Moved {n} messages to Trash"),
        });
        self.last_move = Some(Undo { messages: moved.clone(), action });
        self.discarded = None;
        moved
    }

    /// Stars, unstars, or marks read or unread `ids`, locally and at once. Marking the open
    /// message unread goes back to the list, as Gmail does. Returns what changed; the caller
    /// tells Gmail.
    pub fn mark(&mut self, ids: &[String], mark: Mark) -> Vec<Moved> {
        let label = mark.label();
        let mut changed = Vec::new();
        for gmail_id in ids {
            let Some((account, previous)) = self.labels_of(gmail_id) else { continue };
            let has = previous.iter().any(|l| l == label);
            if has == mark.adds() {
                continue;
            }
            let mut labels = previous.clone();
            if mark.adds() {
                labels.push(label.into());
            } else {
                labels.retain(|l| l != label);
            }
            if self.store.set_labels(account, gmail_id, &labels).unwrap_or(false) {
                changed.push(Moved { account, gmail_id: gmail_id.clone(), previous });
            }
        }
        if changed.is_empty() {
            return changed;
        }
        self.notice = None;
        if let Some(reading) = &mut self.reading
            && changed.iter().any(|m| m.gmail_id == reading.message.summary.gmail_id)
        {
            match mark {
                Mark::Unread => self.reading = None,
                _ => reading.message.summary.starred = mark == Mark::Star,
            }
        }
        if let Some(results) = &mut self.results {
            for m in results.iter_mut().filter(|m| changed.iter().any(|c| c.gmail_id == m.gmail_id)) {
                match mark {
                    Mark::Star | Mark::Unstar => m.starred = mark == Mark::Star,
                    Mark::Read | Mark::Unread => m.unread = mark == Mark::Unread,
                }
            }
            self.rows_version += 1;
        }
        self.reload();
        changed
    }

    /// S: stars the targets, or unstars them when every one is already starred.
    /// ⇧U or ⇧I, whichever changes something: Mark Read when every target is unread.
    pub fn read_mark(&self) -> Mark {
        let targets = self.targets();
        let all_unread = !targets.is_empty()
            && targets.iter().all(|id| self.labels_of(id).is_some_and(|(_, l)| l.iter().any(|l| l == "UNREAD")));
        if all_unread { Mark::Read } else { Mark::Unread }
    }

    pub fn star_mark(&self) -> Mark {
        let targets = self.targets();
        let all_starred = !targets.is_empty()
            && targets.iter().all(|id| self.labels_of(id).is_some_and(|(_, l)| l.iter().any(|l| l == "STARRED")));
        if all_starred { Mark::Unstar } else { Mark::Star }
    }

    /// Gmail refused some label changes: put those messages back and say why.
    pub fn mark_failed(&mut self, failed: &[Moved], error: String) {
        for m in failed {
            let _ = self.store.set_labels(m.account, &m.gmail_id, &m.previous);
        }
        let what = if failed.len() == 1 { "the message".to_string() } else { format!("{} messages", failed.len()) };
        self.error = Some(format!("Couldn't update {what}: {error}"));
        self.reload();
    }

    /// A message's account and its labels now.
    fn labels_of(&self, gmail_id: &str) -> Option<(AccountId, Vec<String>)> {
        let account = self.account_of(gmail_id)?;
        let id = self.store.find(account, gmail_id).ok()??;
        Some((account, self.store.message(id).ok()??.label_ids))
    }

    /// Puts the last moved messages back as they were, locally. Returns what to undo in Gmail.
    pub fn undo_move(&mut self) -> Option<Undo> {
        let undo = self.last_move.take()?;
        for m in &undo.messages {
            let _ = self.store.set_labels(m.account, &m.gmail_id, &m.previous);
        }
        self.notice = None;
        if self.results.is_some() {
            self.search_changed();
        }
        self.reload();
        self.selected = undo.messages.first().map(|m| m.gmail_id.clone());
        if undo.messages.len() > 1 {
            self.checked = undo.messages.iter().map(|m| m.gmail_id.clone()).collect();
        }
        Some(undo)
    }

    /// Gmail refused some moves: put those messages back and say why.
    pub fn move_failed(&mut self, failed: &[Moved], error: String) {
        for m in failed {
            let _ = self.store.set_labels(m.account, &m.gmail_id, &m.previous);
        }
        if self.last_move.as_ref().is_some_and(|u| u.messages.iter().any(|m| failed.contains(m))) {
            self.last_move = None;
            self.notice = None;
        }
        let what = if failed.len() == 1 { "the message".to_string() } else { format!("{} messages", failed.len()) };
        self.error = Some(format!("Couldn't move {what}: {error}"));
        self.reload();
    }

    /// Gmail didn't take the read mark: show the message as unread again, and why.
    pub fn mark_read_failed(&mut self, account: AccountId, gmail_id: &str, error: String) {
        let _ = self.store.set_unread(account, gmail_id, true);
        self.error = Some(format!("Couldn't mark the message read: {error}"));
        self.reload();
    }

    /// Applies a worker update for the account `email`. Returns true when Google signed that
    /// account out.
    pub fn apply(&mut self, email: &str, update: SyncUpdate) -> bool {
        let Some(account) = self.accounts.iter_mut().find(|a| a.email == email) else { return false };
        match update {
            // While older mail loads, each round's progress keeps saying so.
            SyncUpdate::Progress(_) if matches!(account.sync, SyncStatus::LoadingOlder) => {}
            SyncUpdate::Progress(fetched) => account.sync = SyncStatus::Syncing { fetched },
            SyncUpdate::Finished(Ok(report)) => {
                self.error = None;
                account.error = None;
                account.sync = if report.more_to_load { SyncStatus::LoadingOlder } else { SyncStatus::Synced }
            }
            SyncUpdate::Finished(Err(err)) => account.error = Some(err),
            SyncUpdate::SignedOut => return true,
        }
        self.reload();
        false
    }

    /// Syncs every account now.
    pub fn sync_now(&mut self) {
        for account in &mut self.accounts {
            if let Some(worker) = &account.worker {
                account.sync = SyncStatus::Syncing { fetched: 0 };
                worker.sync_now();
            }
        }
    }

    /// Writes sample mail into the database, for snapshots.
    pub fn store_for_samples(&mut self) -> &mut Store {
        &mut self.store
    }

    #[cfg(test)]
    pub fn store_mut(&mut self) -> &mut Store {
        &mut self.store
    }

    #[cfg(test)]
    pub fn account(&self) -> AccountId {
        self.accounts[0].id
    }
}
