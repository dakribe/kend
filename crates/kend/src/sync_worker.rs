//! The background sync loop: syncs at start, then every minute or on request, on its own
//! thread and its own database connection.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread;
use std::time::Duration;

use kend_gmail::{Account, ClientCredentials, Gmail};
use kend_store::Store;
use kend_sync::{Options, Report};
use rune::Waker;

use crate::app::Event;

const INTERVAL: Duration = Duration::from_secs(60);
/// After a failed sync: soon, since the next one resumes where this one stopped.
const RETRY: Duration = Duration::from_secs(15);

/// What the worker reports, tagged with its session so a stopped worker's news is ignored.
pub enum SyncUpdate {
    /// Messages fetched so far in the running sync.
    Progress(usize),
    Finished(Result<Report, String>),
    /// Google no longer accepts the saved sign-in.
    SignedOut,
}

enum Command {
    Now,
    Stop,
}

/// Owns the worker thread; dropping it stops the loop after any sync in progress.
pub struct SyncWorker {
    commands: Sender<Command>,
}

pub struct Spec {
    pub client: ClientCredentials,
    pub account: Account,
    pub db_path: PathBuf,
    pub session: u64,
    pub events: Sender<Event>,
    pub waker: Option<Waker>,
}

impl SyncWorker {
    pub fn start(spec: Spec) -> Self {
        let (commands, inbox) = channel();
        let spawned = thread::Builder::new()
            .name("sync".into())
            .spawn(move || run(spec, inbox));
        if let Err(err) = spawned {
            // Without a thread there is no sync; say so instead of spinning forever.
            let _ = commands.send(Command::Stop);
            eprintln!("kend: couldn't start sync: {err}");
        }
        Self { commands }
    }

    pub fn sync_now(&self) {
        let _ = self.commands.send(Command::Now);
    }
}

impl Drop for SyncWorker {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Stop);
    }
}

fn run(spec: Spec, commands: Receiver<Command>) {
    let send = |update: SyncUpdate| {
        if spec
            .events
            .send(Event::Sync {
                session: spec.session,
                update,
            })
            .is_ok()
            && let Some(wake) = &spec.waker
        {
            wake();
        }
    };
    let opened = Store::open(&spec.db_path)
        .and_then(|store| Ok((store.upsert_account(&spec.account.email)?, store)));
    let (account_id, mut store) = match opened {
        Ok(opened) => opened,
        Err(err) => return send(SyncUpdate::Finished(Err(err.to_string()))),
    };
    let gmail = Gmail::new(spec.client, spec.account.clone());
    let options = Options::default();

    loop {
        let result = kend_sync::sync(&gmail, &mut store, account_id, &options, &mut |n| {
            send(SyncUpdate::Progress(n))
        });
        let wait = match result {
            Err(kend_sync::Error::Gmail(kend_gmail::Error::SignedOut)) => {
                return send(SyncUpdate::SignedOut);
            }
            Ok(report) => {
                send(SyncUpdate::Finished(Ok(report)));
                INTERVAL
            }
            Err(err) => {
                send(SyncUpdate::Finished(Err(describe(&err))));
                RETRY
            }
        };
        match commands.recv_timeout(wait) {
            Ok(Command::Now) | Err(RecvTimeoutError::Timeout) => {}
            Ok(Command::Stop) | Err(RecvTimeoutError::Disconnected) => return,
        }
        // Several clicks on "Sync now" while syncing mean one more sync, not several.
        while let Ok(command) = commands.try_recv() {
            if matches!(command, Command::Stop) {
                return;
            }
        }
    }
}

/// A failed sync, as the status line says it.
fn describe(err: &kend_sync::Error) -> String {
    match err {
        kend_sync::Error::Gmail(kend_gmail::Error::RateLimited) => {
            "Gmail asked kend to slow down".into()
        }
        kend_sync::Error::Gmail(kend_gmail::Error::Network(_)) => "can't reach Gmail".into(),
        err => err.to_string(),
    }
}
