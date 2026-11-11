//! App state. Slow work (Keychain, browser sign-in, network) runs on background threads and
//! lands here as an [`Event`] in `poll`, so the window never waits on it.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

use kend_gmail::{Account, CancelToken, ClientCredentials, Error};
use rune::{Tx, Waker};

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
    SignedIn {
        account: Account,
    },
}

pub enum Event {
    Loaded(Result<Option<Account>, Error>),
    SignInFinished {
        attempt: u64,
        result: Result<Account, Error>,
    },
}

impl App {
    pub fn new() -> Self {
        Self::with_client(ClientCredentials::built_in())
    }

    pub fn with_client(client: Option<ClientCredentials>) -> Self {
        let (sender, events) = channel();
        let auth = if client.is_some() {
            Auth::Loading
        } else {
            Auth::NotConfigured
        };
        Self {
            auth,
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
        changed
    }

    /// Applies one event. Answers the current state didn't ask for change nothing.
    pub fn apply(&mut self, event: Event) -> bool {
        let next = match (&self.auth, event) {
            (Auth::Loading, Event::Loaded(Ok(Some(account)))) => Auth::SignedIn { account },
            (Auth::Loading, Event::Loaded(Ok(None))) => Auth::SignedOut { error: None },
            (Auth::Loading, Event::Loaded(Err(err))) => Auth::SignedOut {
                error: Some(err.to_string()),
            },
            (
                Auth::SigningIn { attempt, .. },
                Event::SignInFinished {
                    attempt: done,
                    result,
                },
            ) if *attempt == done => match result {
                Ok(account) => Auth::SignedIn { account },
                Err(Error::Cancelled) => Auth::SignedOut { error: None },
                Err(err) => Auth::SignedOut {
                    error: Some(err.to_string()),
                },
            },
            _ => return false,
        };
        self.auth = next;
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
        self.auth = Auth::SigningIn {
            attempt,
            cancel: cancel.clone(),
        };
        self.spawn("sign-in", move || Event::SignInFinished {
            attempt,
            result: kend_gmail::sign_in(&client, &cancel),
        });
    }

    pub fn cancel_sign_in(&mut self) {
        if let Auth::SigningIn { cancel, .. } = &self.auth {
            cancel.cancel();
            self.auth = Auth::SignedOut { error: None };
        }
    }

    pub fn sign_out(&mut self) {
        if let Auth::SignedIn { account } =
            std::mem::replace(&mut self.auth, Auth::SignedOut { error: None })
        {
            // Revoking is best effort: the account is already gone from the Keychain and the UI.
            let _ = thread::Builder::new()
                .name("sign-out".into())
                .spawn(move || kend_gmail::sign_out(&account));
        }
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
    use super::*;

    fn account() -> Account {
        Account::new("me@example.com".into(), "refresh".into())
    }

    #[test]
    fn launch_lands_the_saved_account_or_signs_out() {
        let mut app = App::with_client(None);
        app.auth = Auth::Loading;
        assert!(app.apply(Event::Loaded(Ok(Some(account())))));
        assert!(
            matches!(&app.auth, Auth::SignedIn { account } if account.email == "me@example.com")
        );

        app.auth = Auth::Loading;
        app.apply(Event::Loaded(Ok(None)));
        assert!(matches!(app.auth, Auth::SignedOut { error: None }));
    }

    #[test]
    fn an_answer_from_a_cancelled_sign_in_changes_nothing() {
        let mut app = App::with_client(None);
        app.auth = Auth::SigningIn {
            attempt: 1,
            cancel: CancelToken::default(),
        };
        app.cancel_sign_in();
        assert!(matches!(app.auth, Auth::SignedOut { error: None }));

        let late = Event::SignInFinished {
            attempt: 1,
            result: Ok(account()),
        };
        assert!(!app.apply(late));
        assert!(matches!(app.auth, Auth::SignedOut { .. }));
    }

    #[test]
    fn only_the_current_attempt_finishes_sign_in() {
        let mut app = App::with_client(None);
        app.auth = Auth::SigningIn {
            attempt: 2,
            cancel: CancelToken::default(),
        };
        assert!(!app.apply(Event::SignInFinished {
            attempt: 1,
            result: Ok(account())
        }));
        assert!(app.apply(Event::SignInFinished {
            attempt: 2,
            result: Err(Error::TimedOut)
        }));
        assert!(matches!(&app.auth, Auth::SignedOut { error: Some(e) } if e.contains("timed out")));
    }
}
