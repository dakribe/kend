//! The background sync loop: syncs at start, then every minute or on request, on its own
//! thread and its own database connection.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread;
use std::time::{Duration, Instant};

use kend_gmail::Gmail;
use kend_store::Store;
use kend_sync::{Options, Report};
use rune::Waker;

use crate::app::Event;

const INTERVAL: Duration = Duration::from_secs(60);
/// After a failed sync: soon, since the next one resumes where this one stopped.
const RETRY: Duration = Duration::from_secs(15);
/// While older mail is still loading: the next batch right away. The Gmail client's pacing
/// keeps the overall rate under Gmail's quota.
const CONTINUE: Duration = Duration::from_millis(250);

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
    /// Shared with the session, so opening messages and sync use one token and one quota.
    pub gmail: Arc<Gmail>,
    pub db_path: PathBuf,
    pub session: u64,
    pub events: Sender<Event>,
    pub waker: Option<Waker>,
}

impl SyncWorker {
    pub fn start(spec: Spec) -> Self {
        let (commands, inbox) = channel();
        let spawned = thread::Builder::new().name("sync".into()).spawn(move || run(spec, inbox));
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
        if spec.events.send(Event::Sync { session: spec.session, update }).is_ok()
            && let Some(wake) = &spec.waker
        {
            wake();
        }
    };
    let email = spec.gmail.account().email.clone();
    let opened = Store::open(&spec.db_path).and_then(|store| Ok((store.upsert_account(&email)?, store)));
    let (account_id, mut store) = match opened {
        Ok(opened) => opened,
        Err(err) => return send(SyncUpdate::Finished(Err(err.to_string()))),
    };
    let gmail = &*spec.gmail;
    let options = Options::default();

    loop {
        let started = Instant::now();
        let result = kend_sync::sync(gmail, &mut store, account_id, &options, &mut |n| send(SyncUpdate::Progress(n)));
        log(&result, started.elapsed());
        let wait = match result {
            Err(kend_sync::Error::Gmail(kend_gmail::Error::SignedOut)) => {
                return send(SyncUpdate::SignedOut);
            }
            Ok(report) => {
                send(SyncUpdate::Finished(Ok(report)));
                if report.more_to_load { CONTINUE } else { INTERVAL }
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
        kend_sync::Error::Gmail(kend_gmail::Error::RateLimited) => "Gmail asked kend to slow down".into(),
        kend_sync::Error::Gmail(kend_gmail::Error::Network(_)) => "can't reach Gmail".into(),
        err => err.to_string(),
    }
}

/// One line per sync on stderr: what it did and how long it took, or why it failed.
fn log(result: &kend_sync::Result<Report>, took: Duration) {
    let secs = took.as_secs_f64();
    match result {
        Ok(r) => {
            let kind = if r.full { "full sync" } else { "sync" };
            let older = if r.more_to_load { ", more older mail to load" } else { "" };
            eprintln!(
                "kend: {kind}: {} new or changed, {} deleted, {} bodies prefetched, {} older loaded in {secs:.1} s{older}",
                r.fetched, r.deleted, r.bodies, r.backfilled
            );
        }
        Err(err) => eprintln!("kend: sync failed after {secs:.1} s: {err}"),
    }
}
