//! App state. Slow work (Keychain, browser sign-in, network) runs on background threads and
//! lands here as an [`Event`] in `poll`, so the window never waits on it.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

use kend_gmail::{Account, CancelToken, ClientCredentials, Error, Gmail, MessageBody};
use kend_store::Store;
use rune::{Key, KeyEvent, TextEdit, Tx, Waker};
use rune_kit::PaneState;
use rune_kit::shell;

use crate::compose::{Compose, Field, SendState};
use crate::session::{Move, Moved, Session};
use crate::sync_worker::{Spec, SyncUpdate, SyncWorker};

pub struct App {
    pub auth: Auth,
    /// Stands in for a compose field when no message is being written, so field handlers
    /// always have something to edit.
    scratch: TextEdit,
    /// The sidebar's width and whether it's open.
    pub sidebar: PaneState,
    /// Whether the keyboard shortcuts dialog is open (?).
    pub help: bool,
    client: Option<ClientCredentials>,
    sender: Sender<Event>,
    events: Receiver<Event>,
    waker: Option<Waker>,
    next_attempt: u64,
}

/// Where sign-in stands. Each variant holds only what's valid in it.
pub enum Auth {
    /// No Google client was built in (see `.env.example`).
    NotConfigured,
    /// Reading the saved account from the Keychain at launch.
    Loading,
    SignedOut {
        error: Option<String>,
    },
    /// Waiting on the browser. `attempt` fences off answers from a cancelled sign-in.
    SigningIn {
        attempt: u64,
        cancel: CancelToken,
    },
    /// `session` opens on the next poll: the database and the sync worker.
    SignedIn {
        account: Account,
        session: Option<Box<Session>>,
    },
}

pub enum Event {
    Loaded(Result<Option<Account>, Error>),
    SignInFinished {
        attempt: u64,
        result: Result<Account, Error>,
    },
    Sync {
        session: u64,
        update: SyncUpdate,
    },
    /// A message body fetched for reading.
    Body {
        session: u64,
        gmail_id: String,
        result: Result<Option<MessageBody>, String>,
    },
    /// Gmail's answer to archiving or trashing messages: the ones it refused, to put back,
    /// and the first error.
    Moved {
        session: u64,
        failed: Vec<Moved>,
        error: Option<String>,
    },
    /// Gmail's answer to undoing a move.
    Undone {
        session: u64,
        result: Result<(), String>,
    },
    /// Gmail's answer to sending a message.
    Sent {
        session: u64,
        result: Result<(), String>,
    },
    /// Gmail's answer to marking a message read.
    MarkedRead {
        session: u64,
        gmail_id: String,
        result: Result<(), String>,
    },
}

impl App {
    pub fn new() -> Self {
        Self::with_client(ClientCredentials::built_in())
    }

    pub fn with_client(client: Option<ClientCredentials>) -> Self {
        let (sender, events) = channel();
        let auth = if client.is_some() { Auth::Loading } else { Auth::NotConfigured };
        Self {
            auth,
            scratch: TextEdit::new(""),
            sidebar: PaneState::new(232.0).range(180.0, 360.0),
            help: false,
            client,
            sender,
            events,
            waker: None,
            next_attempt: 0,
        }
    }

    /// Runs once at launch with the waker background threads use to wake `poll`.
    pub fn setup(&mut self, waker: Waker) {
        self.waker = Some(waker);
        if matches!(self.auth, Auth::Loading) {
            self.spawn("keychain", || Event::Loaded(kend_gmail::saved_account()));
        }
    }

    /// Lands finished background work. True when the view needs rebuilding.
    pub fn poll(&mut self, _now: f64, tx: &mut Tx) -> bool {
        let mut changed = false;
        while let Ok(event) = self.events.try_recv() {
            if matches!(event, Event::Loaded(_)) {
                // The launch state appears without animating in.
                tx.instant();
            }
            changed |= self.apply(event);
        }
        if let Some(session) = self.session_mut() {
            changed |= session.poll_search();
        }
        changed | self.open_session()
    }

    /// Applies one event. Answers the current state didn't ask for change nothing.
    pub fn apply(&mut self, event: Event) -> bool {
        let next = match (&mut self.auth, event) {
            (Auth::Loading, Event::Loaded(Ok(Some(account)))) => Auth::SignedIn { account, session: None },
            (Auth::Loading, Event::Loaded(Ok(None))) => Auth::SignedOut { error: None },
            (Auth::Loading, Event::Loaded(Err(err))) => Auth::SignedOut { error: Some(err.to_string()) },
            (Auth::SigningIn { attempt, .. }, Event::SignInFinished { attempt: done, result }) if *attempt == done => {
                match result {
                    Ok(account) => Auth::SignedIn { account, session: None },
                    Err(Error::Cancelled) => Auth::SignedOut { error: None },
                    Err(err) => Auth::SignedOut { error: Some(err.to_string()) },
                }
            }
            (Auth::SignedIn { session: Some(session), .. }, Event::Sync { session: id, update })
                if session.id == id =>
            {
                if !session.apply(update) {
                    return true;
                }
                Auth::SignedOut { error: Some(Error::SignedOut.to_string()) }
            }
            (Auth::SignedIn { session: Some(session), .. }, Event::Body { session: id, gmail_id, result })
                if session.id == id =>
            {
                session.body_loaded(&gmail_id, result);
                return true;
            }
            (Auth::SignedIn { session: Some(session), .. }, Event::Moved { session: id, failed, error })
                if session.id == id =>
            {
                if let Some(error) = error {
                    session.move_failed(&failed, error);
                }
                return true;
            }
            (Auth::SignedIn { session: Some(session), .. }, Event::Undone { session: id, result: Err(err) })
                if session.id == id =>
            {
                // The next sync brings back whatever Gmail actually has.
                session.error = Some(format!("Couldn't undo in Gmail: {err}"));
                return true;
            }
            (Auth::SignedIn { session: Some(session), .. }, Event::Sent { session: id, result })
                if session.id == id =>
            {
                match result {
                    Ok(()) => {
                        session.compose = None;
                        session.notice = Some("Message sent".into());
                        session.sync_now();
                    }
                    Err(err) => {
                        if let Some(compose) = &mut session.compose {
                            compose.state = SendState::Failed(format!("Couldn't send: {err}"));
                        }
                    }
                }
                return true;
            }
            (
                Auth::SignedIn { session: Some(session), .. },
                Event::MarkedRead { session: id, gmail_id, result: Err(error) },
            ) if session.id == id => {
                session.mark_read_failed(&gmail_id, error);
                return true;
            }
            _ => return false,
        };
        self.auth = next;
        true
    }

    /// Opens the session for a newly signed-in account: its stored mail first, then sync.
    fn open_session(&mut self) -> bool {
        let Some(client) = self.client else {
            return false;
        };
        let Auth::SignedIn { account, session: session @ None } = &mut self.auth else {
            return false;
        };
        self.next_attempt += 1;
        let id = self.next_attempt;
        let gmail = Arc::new(Gmail::new(client, account.clone()));
        let opened = Store::default_path().and_then(|path| {
            let worker = SyncWorker::start(Spec {
                gmail: gmail.clone(),
                db_path: path.clone(),
                session: id,
                events: self.sender.clone(),
                waker: self.waker.clone(),
            });
            let mut session = Session::open(id, Store::open(&path)?, &account.email, Some(worker), Some(gmail))?;
            if let Some(waker) = self.waker.clone() {
                session.search_in_background(Store::open(&path)?, waker);
            }
            Ok(session)
        });
        match opened {
            Ok(opened) => *session = Some(Box::new(opened)),
            Err(err) => self.auth = Auth::SignedOut { error: Some(format!("Couldn't open the mail database: {err}")) },
        }
        true
    }

    pub fn start_sign_in(&mut self) {
        let Some(client) = self.client else { return };
        if !matches!(self.auth, Auth::SignedOut { .. }) {
            return;
        }
        self.next_attempt += 1;
        let attempt = self.next_attempt;
        let cancel = CancelToken::default();
        self.auth = Auth::SigningIn { attempt, cancel: cancel.clone() };
        self.spawn("sign-in", move || Event::SignInFinished { attempt, result: kend_gmail::sign_in(&client, &cancel) });
    }

    pub fn cancel_sign_in(&mut self) {
        if let Auth::SigningIn { cancel, .. } = &self.auth {
            cancel.cancel();
            self.auth = Auth::SignedOut { error: None };
        }
    }

    pub fn sync_now(&mut self) {
        if let Some(session) = self.session_mut() {
            session.sync_now();
        }
    }

    pub fn select_mailbox(&mut self, mailbox: &str) {
        if let Some(session) = self.session_mut() {
            session.select(mailbox);
        }
    }

    pub fn toggle_labels(&mut self) {
        if let Some(session) = self.session_mut() {
            session.labels_collapsed = !session.labels_collapsed;
        }
    }

    pub fn session(&self) -> Option<&Session> {
        match &self.auth {
            Auth::SignedIn { session: Some(session), .. } => Some(session),
            _ => None,
        }
    }

    fn session_mut(&mut self) -> Option<&mut Session> {
        match &mut self.auth {
            Auth::SignedIn { session: Some(session), .. } => Some(session),
            _ => None,
        }
    }

    /// The list reached `visible_end`: load the next page if it's close. True when rows were added.
    pub fn load_more(&mut self, visible_end: usize) -> bool {
        self.session_mut().is_some_and(|session| session.load_more(visible_end))
    }

    /// Opens a message: shown at once, its body fetched and Gmail told it's read, both in the
    /// background.
    pub fn open_message(&mut self, gmail_id: &str) {
        let Some(session) = self.session_mut() else { return };
        let opened = session.open_message(gmail_id);
        let (id, gmail) = (session.id, session.gmail.clone());
        let Some(gmail) = gmail else { return };
        if opened.fetch_body {
            self.fetch_body(id, gmail.clone(), gmail_id.to_string());
        }
        if opened.mark_read {
            let gmail_id = gmail_id.to_string();
            self.spawn("mark-read", move || {
                let result = gmail.modify_labels(&gmail_id, &[], &["UNREAD"]).map_err(|e| e.to_string());
                Event::MarkedRead { session: id, gmail_id, result }
            });
        }
    }

    fn fetch_body(&self, session: u64, gmail: Arc<Gmail>, gmail_id: String) {
        self.spawn("body", move || {
            let result = gmail.body(&gmail_id).map_err(|e| e.to_string());
            Event::Body { session, gmail_id, result }
        });
    }

    pub fn compose_new(&mut self) {
        if let Some(session) = self.session_mut() {
            session.notice = None;
            session.compose = Some(Compose::new());
        }
    }

    /// Replies to (or forwards) the open message.
    pub fn reply(&mut self, forward: bool) {
        let Some(session) = self.session_mut() else { return };
        let Some(reading) = &session.reading else { return };
        let now = jiff::Zoned::now();
        let compose =
            if forward { Compose::forward(&reading.message, &now) } else { Compose::reply(&reading.message, &now) };
        session.notice = None;
        session.compose = Some(compose);
    }

    /// Archives or trashes the open, checked or selected messages: at once here, then in Gmail.
    pub fn move_message(&mut self, action: Move) {
        let Some(session) = self.session_mut() else { return };
        let moved = session.move_messages(action);
        let (id, Some(gmail)) = (session.id, session.gmail.clone()) else { return };
        if moved.is_empty() {
            return;
        }
        self.spawn("move", move || {
            let (failed, error) = each_in_gmail(moved, |m| match action {
                Move::Archive => gmail.modify_labels(&m.gmail_id, &[], &["INBOX"]),
                Move::Trash => gmail.trash(&m.gmail_id),
            });
            Event::Moved { session: id, failed, error }
        });
    }

    pub fn undo(&mut self) {
        let Some(session) = self.session_mut() else { return };
        let Some(undo) = session.undo_move() else { return };
        let (id, Some(gmail)) = (session.id, session.gmail.clone()) else { return };
        self.spawn("undo", move || {
            let (_, error) = each_in_gmail(undo.messages, |m| match undo.action {
                Move::Archive => gmail.modify_labels(&m.gmail_id, &["INBOX"], &[]),
                Move::Trash => gmail.untrash(&m.gmail_id),
            });
            Event::Undone { session: id, result: error.map_or(Ok(()), Err) }
        });
    }

    /// A row click: ⌘-click checks or unchecks it, Shift-click checks the range up to it, and a
    /// plain click opens it. The checked rows stay checked for when you come back to the list.
    pub fn click_row(&mut self, gmail_id: &str, mods: rune::Mods) {
        let Some(session) = self.session_mut() else { return };
        if mods.shift {
            session.check_range(gmail_id);
        } else if mods.cmd {
            session.toggle_checked(gmail_id);
        } else {
            self.open_message(gmail_id);
        }
    }

    /// The row's checkbox: Shift-click checks a range, any other click toggles the row.
    pub fn click_checkbox(&mut self, gmail_id: &str, mods: rune::Mods) {
        let Some(session) = self.session_mut() else { return };
        if mods.shift {
            session.check_range(gmail_id);
        } else {
            session.toggle_checked(gmail_id);
        }
    }

    pub fn clear_checked(&mut self) {
        if let Some(session) = self.session_mut() {
            session.clear_checked();
        }
    }

    pub fn toggle_help(&mut self) {
        self.help = !self.help;
    }

    pub fn discard_compose(&mut self) {
        if let Some(session) = self.session_mut() {
            session.compose = None;
        }
    }

    /// The search field, or a scratch edit before a session opens.
    pub fn search_field(&mut self) -> &mut TextEdit {
        if let Auth::SignedIn { session: Some(session), .. } = &mut self.auth {
            return &mut session.search;
        }
        &mut self.scratch
    }

    pub fn search_changed(&mut self) {
        if let Some(session) = self.session_mut() {
            session.search_changed();
        }
    }

    /// Keys in the search field: Escape clears it, Return or ↓ moves into the results.
    pub fn search_key(&mut self, event: &KeyEvent, tx: &mut Tx) -> bool {
        tx.instant();
        let Some(session) = self.session_mut() else { return false };
        match event.key {
            Key::Escape => {
                session.clear_search();
                tx.blur();
                true
            }
            Key::Enter | Key::Down if session.results.is_some() => {
                tx.blur();
                if let Some(first) = session.move_selection(1).map(str::to_string) {
                    tx.scroll_into_view(first);
                }
                true
            }
            _ => false,
        }
    }

    /// The compose field `field`, or a scratch edit when nothing is being written.
    pub fn compose_field(&mut self, field: Field) -> &mut TextEdit {
        if let Auth::SignedIn { session: Some(session), .. } = &mut self.auth
            && let Some(compose) = &mut session.compose
        {
            return compose.field(field);
        }
        &mut self.scratch
    }

    /// Sends the message being written, in the background. A reply first fetches the headers
    /// that keep it in its conversation.
    pub fn send(&mut self) {
        let Auth::SignedIn { account, session: Some(session) } = &mut self.auth else { return };
        let Some(compose) = &mut session.compose else { return };
        if matches!(compose.state, SendState::Sending) {
            return;
        }
        let mut outgoing = match compose.outgoing(&account.email) {
            Ok(outgoing) => outgoing,
            Err(err) => {
                compose.state = SendState::Failed(err);
                return;
            }
        };
        let Some(gmail) = session.gmail.clone() else {
            compose.state = SendState::Failed("Not connected to Gmail.".into());
            return;
        };
        compose.state = SendState::Sending;
        let replying_to = compose.replying_to().map(str::to_string);
        let id = session.id;
        self.spawn("send", move || {
            let result = (|| {
                if let Some(original) = replying_to {
                    (outgoing.in_reply_to, outgoing.references) = gmail.reply_headers(&original)?;
                }
                gmail.send(&outgoing)
            })();
            Event::Sent { session: id, result: result.map(|_| ()).map_err(|e| e.to_string()) }
        });
    }

    /// Opens a link from an email in the default browser. Only web and mail links: an email
    /// shouldn't be able to launch apps through custom URL schemes or open local files.
    pub fn open_link(&mut self, url: &str) {
        if let Some(url) = safe_link(url) {
            let _ = open::that_detached(url);
        }
    }

    /// Writes the open message's HTML to a temporary file and opens it in the default browser,
    /// for designed emails the reading view simplifies.
    pub fn open_in_browser(&mut self) {
        let Some(reading) = self.session().and_then(|s| s.reading.as_ref()) else { return };
        let Some(html) = &reading.message.body_html else { return };
        let dir = std::env::temp_dir().join("kend");
        // Gmail ids are hex, so they make safe file names.
        let file = dir.join(format!("{}.html", reading.message.summary.gmail_id));
        let page = format!("<!doctype html><meta charset=\"utf-8\">\n{html}");
        if std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&file, page)).is_ok() {
            let _ = open::that_detached(&file);
        }
    }

    pub fn close_message(&mut self) {
        if let Some(session) = self.session_mut() {
            session.close_message();
        }
    }

    pub fn retry_body(&mut self) {
        let Some(session) = self.session_mut() else { return };
        let Some(gmail_id) = session.retry_body() else { return };
        if let Some(gmail) = session.gmail.clone() {
            let id = session.id;
            self.fetch_body(id, gmail, gmail_id);
        }
    }

    /// Window-wide shortcuts: ⇧⌘B shows and hides the sidebar; J/K and the arrows move
    /// through messages, Return opens one and Escape goes back to the list. While a message
    /// is open, J/K open the next and previous one.
    pub fn on_key(&mut self, event: &KeyEvent, tx: &mut Tx) -> bool {
        // Keys land at once: no glide when the list follows the selection.
        tx.instant();
        let plain = !(event.mods.cmd || event.mods.ctrl || event.mods.alt);
        if self.help {
            // The shortcuts dialog is up: only the keys that close it do anything.
            let close = matches!(&event.key, Key::Escape) || matches!(&event.key, Key::Text(t) if t == "?");
            if close {
                self.help = false;
            }
            return close;
        }
        if shell::is_sidebar_toggle(event) {
            self.sidebar.toggle();
            return true;
        }
        if event.is_command('n') {
            self.compose_new();
            return true;
        }
        if event.is_command('f') {
            tx.focus("search", true);
            return true;
        }
        let Some(session) = self.session_mut() else { return false };
        if session.compose.is_some() {
            // Keys typed while writing belong to the fields, never to list shortcuts.
            return false;
        }
        let reading = session.reading.is_some();
        let shift = event.mods.shift;
        if let Key::Text(t) = &event.key
            && plain
            && t == "?"
        {
            self.help = true;
            return true;
        }
        if event.is_command('a') && !reading {
            session.check_all();
            return true;
        }
        let delta = match &event.key {
            Key::Escape if !session.checked.is_empty() && !reading => {
                session.clear_checked();
                return true;
            }
            Key::Text(t) if plain && !reading && t == "x" => {
                let Some(id) = session.selected.clone().or_else(|| session.move_selection(1).map(str::to_string))
                else {
                    return false;
                };
                session.toggle_checked(&id);
                return true;
            }
            Key::Down if !reading && shift => return extend(session, 1, tx),
            Key::Up if !reading && shift => return extend(session, -1, tx),
            Key::Text(t) if plain && !reading && t == "J" => return extend(session, 1, tx),
            Key::Text(t) if plain && !reading && t == "K" => return extend(session, -1, tx),
            Key::Escape if reading => {
                session.close_message();
                return true;
            }
            Key::Escape if session.results.is_some() => {
                session.clear_search();
                return true;
            }
            Key::Text(t) if plain && t == "/" => {
                tx.focus("search", true);
                return true;
            }
            Key::Text(t) if plain && t == "e" => {
                self.move_message(Move::Archive);
                return true;
            }
            Key::Text(t) if plain && t == "#" => {
                self.move_message(Move::Trash);
                return true;
            }
            Key::Delete | Key::Backspace if plain => {
                self.move_message(Move::Trash);
                return true;
            }
            Key::Text(t) if plain && t == "z" => {
                self.undo();
                return true;
            }
            Key::Text(t) if reading && plain && (t == "r" || t == "f") => {
                let forward = t == "f";
                self.reply(forward);
                return true;
            }
            Key::Enter if !reading => {
                let Some(id) = session.selected.clone() else { return false };
                self.open_message(&id);
                return true;
            }
            Key::Down if !reading => 1,
            Key::Up if !reading => -1,
            Key::Text(t) if plain && t == "j" => 1,
            Key::Text(t) if plain && t == "k" => -1,
            _ => return false,
        };
        let Some(selected) = session.move_selection(delta).map(str::to_string) else { return false };
        if reading {
            self.open_message(&selected);
        }
        tx.scroll_into_view(selected);
        true
    }

    /// Stops sync, then (off the UI thread) deletes the account's local mail and revokes its
    /// token. The database and Keychain keep nothing of the account.
    pub fn sign_out(&mut self) {
        let Auth::SignedIn { account, session } = std::mem::replace(&mut self.auth, Auth::SignedOut { error: None })
        else {
            return;
        };
        drop(session);
        let forget = move || {
            if let Ok(store) = Store::default_path().and_then(|path| Store::open(&path)) {
                let _ = store.upsert_account(&account.email).and_then(|id| store.delete_account(id));
            }
            // Revoking is best effort: the account is already gone locally.
            let _ = kend_gmail::sign_out(&account);
        };
        let _ = thread::Builder::new().name("sign-out".into()).spawn(forget);
    }

    fn spawn(&self, name: &str, work: impl FnOnce() -> Event + Send + 'static) {
        let sender = self.sender.clone();
        let waker = self.waker.clone();
        let spawned = thread::Builder::new().name(name.into()).spawn(move || {
            if sender.send(work()).is_ok()
                && let Some(wake) = waker
            {
                wake();
            }
        });
        if let Err(err) = spawned {
            let _ = self.sender.send(Event::Loaded(Err(Error::Io(err))));
        }
    }
}

/// Shift+J/K: checks the selected row and the next (or previous) one.
fn extend(session: &mut Session, delta: isize, tx: &mut Tx) -> bool {
    let Some(selected) = session.extend_selection(delta).map(str::to_string) else { return false };
    tx.scroll_into_view(selected);
    true
}

/// Runs `call` for each message on a few threads at once (the client paces them), returning
/// the ones that failed and the first error.
fn each_in_gmail(
    messages: Vec<Moved>,
    call: impl Fn(&Moved) -> Result<(), Error> + Sync,
) -> (Vec<Moved>, Option<String>) {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    let next = AtomicUsize::new(0);
    let failed = Mutex::new((Vec::new(), None));
    thread::scope(|scope| {
        for _ in 0..messages.len().min(4) {
            scope.spawn(|| {
                while let Some(m) = messages.get(next.fetch_add(1, Ordering::Relaxed)) {
                    if let Err(err) = call(m) {
                        let mut failed = failed.lock().unwrap_or_else(|p| p.into_inner());
                        failed.0.push(m.clone());
                        failed.1.get_or_insert(err.to_string());
                    }
                }
            });
        }
    });
    failed.into_inner().unwrap_or_else(|p| p.into_inner())
}

/// A click handler that lands its change at once, as every kend action does: no layout
/// springs, fades or colour easing (see `view::CLOCK`).
pub fn instant(f: impl Fn(&mut App) + 'static) -> impl Fn(&mut App, &mut Tx) + 'static {
    move |s, tx| {
        tx.instant();
        f(s)
    }
}

/// `url` trimmed, when it's a web or mail link.
fn safe_link(url: &str) -> Option<&str> {
    let url = url.trim();
    let lower = url.to_ascii_lowercase();
    ["https://", "http://", "mailto:"].iter().any(|scheme| lower.starts_with(scheme)).then_some(url)
}

#[cfg(test)]
mod tests {
    use kend_store::{Address, NewMessage};

    use super::*;
    use crate::session::SyncStatus;

    fn account() -> Account {
        Account::new("me@example.com".into(), "refresh".into())
    }

    fn signed_in(app: &mut App, id: u64) {
        let session = Session::open(id, Store::open_in_memory().unwrap(), "me@example.com", None, None).unwrap();
        app.auth = Auth::SignedIn { account: account(), session: Some(Box::new(session)) };
    }

    #[test]
    fn launch_lands_the_saved_account_or_signs_out() {
        let mut app = App::with_client(None);
        app.auth = Auth::Loading;
        assert!(app.apply(Event::Loaded(Ok(Some(account())))));
        assert!(matches!(&app.auth, Auth::SignedIn { account, .. } if account.email == "me@example.com"));

        app.auth = Auth::Loading;
        app.apply(Event::Loaded(Ok(None)));
        assert!(matches!(app.auth, Auth::SignedOut { error: None }));
    }

    #[test]
    fn an_answer_from_a_cancelled_sign_in_changes_nothing() {
        let mut app = App::with_client(None);
        app.auth = Auth::SigningIn { attempt: 1, cancel: CancelToken::default() };
        app.cancel_sign_in();
        assert!(matches!(app.auth, Auth::SignedOut { error: None }));

        let late = Event::SignInFinished { attempt: 1, result: Ok(account()) };
        assert!(!app.apply(late));
        assert!(matches!(app.auth, Auth::SignedOut { .. }));
    }

    #[test]
    fn only_the_current_attempt_finishes_sign_in() {
        let mut app = App::with_client(None);
        app.auth = Auth::SigningIn { attempt: 2, cancel: CancelToken::default() };
        assert!(!app.apply(Event::SignInFinished { attempt: 1, result: Ok(account()) }));
        assert!(app.apply(Event::SignInFinished { attempt: 2, result: Err(Error::TimedOut) }));
        assert!(matches!(&app.auth, Auth::SignedOut { error: Some(e) } if e.contains("timed out")));
    }

    #[test]
    fn sync_progress_reloads_the_inbox_from_the_store() {
        let mut app = App::with_client(None);
        signed_in(&mut app, 7);
        let Auth::SignedIn { session: Some(session), .. } = &mut app.auth else { unreachable!() };
        let account = session.account();
        let message = NewMessage {
            gmail_id: "m1".into(),
            from: Address::new("Ana", "ana@x.com"),
            label_ids: vec!["INBOX".into(), "UNREAD".into()],
            ..NewMessage::default()
        };
        session.store_mut().upsert_messages(account, &[message]).unwrap();

        assert!(app.apply(Event::Sync { session: 7, update: SyncUpdate::Progress(1) }));
        let Auth::SignedIn { session: Some(session), .. } = &app.auth else { unreachable!() };
        assert_eq!((session.rows.len(), session.unread_in("INBOX")), (1, 1));
        assert!(matches!(session.sync, SyncStatus::Syncing { fetched: 1 }));
    }

    #[test]
    fn a_stopped_workers_updates_are_ignored() {
        let mut app = App::with_client(None);
        signed_in(&mut app, 7);
        assert!(!app.apply(Event::Sync { session: 6, update: SyncUpdate::SignedOut }));
        assert!(matches!(app.auth, Auth::SignedIn { .. }));
    }

    #[test]
    fn a_revoked_sign_in_returns_to_the_sign_in_screen() {
        let mut app = App::with_client(None);
        signed_in(&mut app, 7);
        assert!(app.apply(Event::Sync { session: 7, update: SyncUpdate::SignedOut }));
        assert!(matches!(&app.auth, Auth::SignedOut { error: Some(e) } if e.contains("sign in again")));
    }

    fn many(count: usize) -> Vec<NewMessage> {
        (0..count)
            .map(|i| NewMessage {
                gmail_id: format!("m{i:04}"),
                received_at: -(i as i64),
                label_ids: vec!["INBOX".into()],
                ..NewMessage::default()
            })
            .collect()
    }

    fn session_with(count: usize) -> Session {
        let mut store = Store::open_in_memory().unwrap();
        let id = store.upsert_account("me@example.com").unwrap();
        store.upsert_messages(id, &many(count)).unwrap();
        Session::open(1, store, "me@example.com", None, None).unwrap()
    }

    #[test]
    fn the_list_loads_pages_as_it_nears_the_end() {
        let mut session = session_with(450);
        assert_eq!(session.rows.len(), 200);
        assert!(!session.load_more(20), "far from the end: nothing to load");
        assert!(session.load_more(150));
        assert_eq!(session.rows.len(), 400);
        assert!(session.load_more(399));
        assert_eq!(session.rows.len(), 450);
        assert!(!session.load_more(449), "the whole mailbox is loaded");
        assert_eq!(session.rows.last().map(|m| m.gmail_id.as_str()), Some("m0449"));
    }

    #[test]
    fn a_sync_reload_keeps_every_loaded_row() {
        let mut session = session_with(450);
        session.load_more(199);
        let version = session.rows_version;
        session.reload();
        assert_eq!(session.rows.len(), 400);
        assert!(session.rows_version > version);
    }

    #[test]
    fn keys_move_the_selection_and_load_more_at_the_end() {
        let mut session = session_with(450);
        assert_eq!(session.move_selection(1), Some("m0000"), "the first press selects the top row");
        assert_eq!(session.move_selection(1), Some("m0001"));
        assert_eq!(session.move_selection(-5), Some("m0000"), "stops at the top");
        session.selected = Some("m0199".into());
        assert_eq!(session.move_selection(1), Some("m0200"), "past the loaded rows");
        assert_eq!(session.rows.len(), 400);
    }

    fn signed_in_with(app: &mut App, count: usize, unread: bool) {
        let mut store = Store::open_in_memory().unwrap();
        let id = store.upsert_account("me@example.com").unwrap();
        let mut messages = many(count);
        for m in &mut messages {
            if unread {
                m.label_ids.push("UNREAD".into());
            }
        }
        messages[1].body_text = Some("already here".into());
        store.upsert_messages(id, &messages).unwrap();
        let session = Session::open(1, store, "me@example.com", None, None).unwrap();
        app.auth = Auth::SignedIn { account: account(), session: Some(Box::new(session)) };
    }

    fn reading(app: &App) -> Option<&str> {
        app.session()?.reading.as_ref().map(|r| r.message.summary.gmail_id.as_str())
    }

    #[test]
    fn opening_marks_read_at_once_and_asks_for_a_missing_body() {
        let mut session = session_with(3);
        let account = session.account();
        session.store_mut().set_unread(account, "m0000", true).unwrap();
        session.reload();
        let opened = session.open_message("m0000");
        assert_eq!(opened, crate::session::Opened { fetch_body: true, mark_read: true });
        assert!(!session.rows[0].unread, "the list shows it read straight away");
        assert!(matches!(session.reading.as_ref().unwrap().body, crate::session::BodyState::Loading));

        // Opening it again doesn't mark it read twice.
        assert!(!session.open_message("m0000").mark_read);
    }

    #[test]
    fn a_fetched_body_is_shown_and_kept_for_next_time() {
        let mut session = session_with(3);
        session.open_message("m0002");
        let body = kend_gmail::MessageBody { text: "hello".into(), html: None };
        session.body_loaded("m0002", Ok(Some(body)));
        let reading = session.reading.as_ref().unwrap();
        assert!(matches!(reading.body, crate::session::BodyState::Ready));
        assert_eq!(reading.message.body_text.as_deref(), Some("hello"));

        session.close_message();
        assert!(!session.open_message("m0002").fetch_body, "stored: no second download");
    }

    #[test]
    fn a_body_for_a_message_no_longer_open_is_still_stored() {
        let mut session = session_with(3);
        session.open_message("m0000");
        session.open_message("m0001");
        session.body_loaded("m0000", Ok(Some(kend_gmail::MessageBody { text: "late".into(), html: None })));
        assert_eq!(session.reading.as_ref().unwrap().message.summary.gmail_id, "m0001");
        assert!(!session.open_message("m0000").fetch_body);
    }

    #[test]
    fn a_refused_read_mark_shows_the_message_unread_again() {
        let mut session = session_with(3);
        let account = session.account();
        session.store_mut().set_unread(account, "m0000", true).unwrap();
        session.open_message("m0000");
        session.mark_read_failed("m0000", "offline".into());
        session.reload();
        assert!(session.rows[0].unread);
        assert!(session.error.as_deref().is_some_and(|e| e.contains("offline")));
    }

    fn press(app: &mut App, key: Key) {
        let mut tx = Tx::default();
        app.on_key(&KeyEvent { key, mods: Default::default() }, &mut tx);
    }

    #[test]
    fn return_opens_escape_closes_and_j_moves_to_the_next_message() {
        let mut app = App::with_client(None);
        signed_in_with(&mut app, 5, false);
        press(&mut app, Key::Text("j".into()));
        assert_eq!(app.session().unwrap().selected.as_deref(), Some("m0000"));
        press(&mut app, Key::Enter);
        assert_eq!(reading(&app), Some("m0000"));
        press(&mut app, Key::Text("j".into()));
        assert_eq!(reading(&app), Some("m0001"), "j opens the next message while reading");
        press(&mut app, Key::Escape);
        assert_eq!(reading(&app), None);
        assert_eq!(app.session().unwrap().selected.as_deref(), Some("m0001"), "back on the list, at the same message");
    }

    #[test]
    fn only_web_and_mail_links_open() {
        assert_eq!(safe_link(" https://example.com/a "), Some("https://example.com/a"));
        assert_eq!(safe_link("MAILTO:ana@example.com"), Some("MAILTO:ana@example.com"));
        assert_eq!(safe_link("file:///etc/passwd"), None);
        assert_eq!(safe_link("javascript:alert(1)"), None);
        assert_eq!(safe_link("zoommtg://join?x"), None);
    }

    #[test]
    fn background_searches_land_only_for_the_current_query() {
        let dir = std::env::temp_dir().join(format!("kend-search-{}", std::process::id()));
        let path = dir.join("mail.sqlite");
        let mut store = Store::open(&path).unwrap();
        let id = store.upsert_account("me@example.com").unwrap();
        let mut messages = many(3);
        messages[1].subject = "Quarterly report".into();
        store.upsert_messages(id, &messages).unwrap();
        let mut session = Session::open(1, store, "me@example.com", None, None).unwrap();
        session.search_in_background(Store::open(&path).unwrap(), Arc::new(|| {}));

        session.search.text = "quart".into();
        session.search_changed();
        let started = std::time::Instant::now();
        while !session.poll_search() {
            assert!(started.elapsed().as_secs() < 5, "search never landed");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let ids: Vec<_> = session.visible().iter().map(|m| m.gmail_id.as_str()).collect();
        assert_eq!(ids, ["m0001"]);

        session.clear_search();
        assert_eq!(session.visible().len(), 3, "the mailbox is back");
        std::fs::remove_dir_all(dir).ok();
    }

    fn ids(moved: &[Moved]) -> Vec<&str> {
        moved.iter().map(|m| m.gmail_id.as_str()).collect()
    }

    fn visible_ids(session: &Session) -> Vec<&str> {
        session.visible().iter().map(|m| m.gmail_id.as_str()).collect()
    }

    #[test]
    fn archiving_takes_a_message_out_of_the_inbox_and_selects_the_next() {
        let mut session = session_with(4);
        session.selected = Some("m0001".into());
        assert_eq!(ids(&session.move_messages(Move::Archive)), ["m0001"]);
        assert_eq!(visible_ids(&session), ["m0000", "m0002", "m0003"]);
        assert_eq!(session.selected.as_deref(), Some("m0002"), "the next message takes its place");
        assert_eq!(session.notice.as_deref(), Some("Archived"));
        // Archived again: it's no longer in the inbox, so nothing happens.
        session.selected = Some("m0001".into());
        assert!(session.move_messages(Move::Archive).is_empty());
    }

    #[test]
    fn undo_puts_a_trashed_message_back_with_its_labels() {
        let mut session = session_with(3);
        session.selected = Some("m0002".into());
        session.move_messages(Move::Trash);
        assert_eq!(visible_ids(&session), ["m0000", "m0001"]);
        session.select("TRASH");
        assert_eq!(visible_ids(&session), ["m0002"]);
        session.select("INBOX");

        let undo = session.undo_move().unwrap();
        assert_eq!((ids(&undo.messages), undo.action), (vec!["m0002"], Move::Trash));
        assert_eq!(visible_ids(&session), ["m0000", "m0001", "m0002"]);
        assert_eq!(session.selected.as_deref(), Some("m0002"));
        assert!(session.undo_move().is_none(), "one undo per move");
    }

    #[test]
    fn a_refused_move_brings_the_message_back() {
        let mut session = session_with(2);
        session.selected = Some("m0000".into());
        let moved = session.move_messages(Move::Trash);
        session.move_failed(&moved, "offline".into());
        assert_eq!(visible_ids(&session), ["m0000", "m0001"]);
        assert!(session.error.as_deref().is_some_and(|e| e.contains("offline")));
        assert!(session.last_move.is_none());
    }

    #[test]
    fn the_open_message_is_the_one_moved() {
        let mut session = session_with(3);
        session.open_message("m0001");
        session.move_messages(Move::Archive);
        assert!(session.reading.is_none(), "back to the list");
        assert_eq!(visible_ids(&session), ["m0000", "m0002"]);
    }

    #[test]
    fn checked_messages_are_archived_together_and_undone_together() {
        let mut session = session_with(5);
        session.toggle_checked("m0001");
        session.toggle_checked("m0003");
        session.selected = Some("m0004".into());
        assert_eq!(
            ids(&session.move_messages(Move::Archive)),
            ["m0001", "m0003"],
            "checked rows win over the selection"
        );
        assert_eq!(visible_ids(&session), ["m0000", "m0002", "m0004"]);
        assert!(session.checked.is_empty());
        assert_eq!(session.notice.as_deref(), Some("Archived 2 messages"));

        session.undo_move();
        assert_eq!(visible_ids(&session).len(), 5);
        assert_eq!(session.checked.len(), 2, "the undone messages are selected again");
    }

    #[test]
    fn shift_click_checks_a_range_and_cmd_click_toggles() {
        let mut app = App::with_client(None);
        signed_in_with(&mut app, 6, false);
        let cmd = rune::Mods { cmd: true, ..Default::default() };
        let shift = rune::Mods { shift: true, ..Default::default() };
        app.click_row("m0001", cmd);
        app.click_row("m0004", shift);
        let checked = |app: &App| {
            let mut c: Vec<_> = app.session().unwrap().checked.iter().cloned().collect();
            c.sort();
            c
        };
        assert_eq!(checked(&app), ["m0001", "m0002", "m0003", "m0004"]);
        app.click_row("m0002", cmd);
        assert_eq!(checked(&app), ["m0001", "m0003", "m0004"]);
        assert_eq!(reading(&app), None, "modified clicks don't open");
        app.click_row("m0005", rune::Mods::default());
        assert_eq!(reading(&app), Some("m0005"));
        press(&mut app, Key::Escape);
        assert_eq!(checked(&app), ["m0001", "m0003", "m0004"], "still checked after reading a message");
    }

    #[test]
    fn x_shift_j_and_cmd_a_check_rows_and_escape_clears() {
        let mut app = App::with_client(None);
        signed_in_with(&mut app, 5, false);
        let checked = |app: &App| app.session().unwrap().checked.len();
        press(&mut app, Key::Text("x".into()));
        assert_eq!(checked(&app), 1, "x checks the top row when nothing is selected");
        press(&mut app, Key::Text("x".into()));
        assert_eq!(checked(&app), 0);
        press(&mut app, Key::Text("J".into()));
        press(&mut app, Key::Text("J".into()));
        assert_eq!(checked(&app), 3);
        assert_eq!(app.session().unwrap().selected.as_deref(), Some("m0002"));
        press(&mut app, Key::Escape);
        assert_eq!(checked(&app), 0);
        let mut tx = Tx::default();
        app.on_key(&KeyEvent { key: Key::Command('a'), mods: rune::Mods { cmd: true, ..Default::default() } }, &mut tx);
        assert_eq!(checked(&app), 5);
        press(&mut app, Key::Text("#".into()));
        assert!(app.session().unwrap().visible().is_empty(), "all five went to Trash");
    }

    #[test]
    fn moving_the_open_message_keeps_the_others_checked() {
        let mut session = session_with(4);
        session.toggle_checked("m0000");
        session.toggle_checked("m0002");
        session.open_message("m0002");
        assert_eq!(ids(&session.move_messages(Move::Archive)), ["m0002"], "only the open message moves");
        let checked: Vec<_> = session.checked.iter().map(String::as_str).collect();
        assert_eq!(checked, ["m0000"]);
    }

    #[test]
    fn question_mark_opens_help_and_escape_closes_it() {
        let mut app = App::with_client(None);
        signed_in_with(&mut app, 3, false);
        press(&mut app, Key::Text("?".into()));
        assert!(app.help);
        press(&mut app, Key::Text("e".into()));
        assert_eq!(visible_ids(app.session().unwrap()).len(), 3, "list keys wait while help is open");
        press(&mut app, Key::Escape);
        assert!(!app.help);
    }

    #[test]
    fn bulk_gmail_calls_report_only_the_failures() {
        let messages: Vec<Moved> =
            (0..10).map(|i| Moved { gmail_id: format!("m{i}"), previous: vec!["INBOX".into()] }).collect();
        let (failed, error) = each_in_gmail(messages, |m| {
            if m.gmail_id == "m3" || m.gmail_id == "m7" { Err(Error::TimedOut) } else { Ok(()) }
        });
        let mut failed = ids(&failed);
        failed.sort();
        assert_eq!(failed, ["m3", "m7"]);
        assert!(error.is_some());
    }
}
