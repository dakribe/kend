//! App state. Slow work (Keychain, browser sign-in, network) runs on background threads and
//! lands here as an [`Event`] in `poll`, so the window never waits on it.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

use kend_gmail::{Account, CancelToken, ClientCredentials, Error};
use kend_store::Store;
use rune::{Tx, Waker};

use crate::session::Session;
use crate::sync_worker::{Spec, SyncUpdate, SyncWorker};

pub struct App {
    pub auth: Auth,
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
    SignInFinished { attempt: u64, result: Result<Account, Error> },
    Sync { session: u64, update: SyncUpdate },
}

impl App {
    pub fn new() -> Self {
        Self::with_client(ClientCredentials::built_in())
    }

    pub fn with_client(client: Option<ClientCredentials>) -> Self {
        let (sender, events) = channel();
        let auth = if client.is_some() { Auth::Loading } else { Auth::NotConfigured };
        Self { auth, client, sender, events, waker: None, next_attempt: 0 }
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
        let opened = Store::default_path().and_then(|path| {
            let worker = SyncWorker::start(Spec {
                client,
                account: account.clone(),
                db_path: path.clone(),
                session: id,
                events: self.sender.clone(),
                waker: self.waker.clone(),
            });
            Session::open(id, Store::open(&path)?, &account.email, Some(worker))
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
        if let Auth::SignedIn { session: Some(session), .. } = &mut self.auth {
            session.sync_now();
        }
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

#[cfg(test)]
mod tests {
    use kend_store::{Address, NewMessage};

    use super::*;
    use crate::session::SyncStatus;

    fn account() -> Account {
        Account::new("me@example.com".into(), "refresh".into())
    }

    fn signed_in(app: &mut App, id: u64) {
        let session = Session::open(id, Store::open_in_memory().unwrap(), "me@example.com", None).unwrap();
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
        assert_eq!((session.rows.len(), session.unread), (1, 1));
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
}
