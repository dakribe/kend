//! Sample mail for headless snapshots and benchmarks: an in-memory database, no network.

use kend_gmail::Account;
use kend_store::{Address, Label, LabelKind, NewMessage, Store};

use crate::app::Auth;
use crate::session::{Session, SyncStatus};

const SENDERS: [&str; 8] = ["Ana Lima", "Bo Chen", "GitHub", "Priya Nair", "Linear", "Sam Ortiz", "Stripe", "Mei Tan"];
const SUBJECTS: [&str; 8] = [
    "Weekly sync notes",
    "Your receipt from Stripe",
    "Re: Design review for the settings page",
    "[kend] CI failed on main",
    "Flight confirmation: SFO to JFK",
    "Quick question about the contract",
    "Photos from the weekend",
    "Invoice #4821 is due",
];

fn now_ms() -> i64 {
    jiff::Timestamp::now().as_millisecond()
}

/// A message `i` steps older than the samples, with varied senders, subjects and read state.
fn generated(i: usize) -> NewMessage {
    let unread = i.is_multiple_of(7);
    NewMessage {
        gmail_id: format!("gen-{i:05}"),
        from: Address::new(SENDERS[i % SENDERS.len()], "someone@example.com"),
        subject: format!("{} ({})", SUBJECTS[(i / 3) % SUBJECTS.len()], i),
        snippet: "Here is a longer preview line so the row has to truncate text like real mail does".into(),
        // Hours apart, reaching back months, so every date format shows up.
        received_at: now_ms() - (i as i64 + 4) * 97 * 60_000,
        label_ids: if unread { vec!["INBOX".into(), "UNREAD".into()] } else { vec!["INBOX".into()] },
        ..NewMessage::default()
    }
}

/// A signed-in session over an in-memory database with a few messages, for snapshots.
pub fn sample_inbox(sync: SyncStatus) -> Auth {
    inbox_with(sync, 0)
}

/// The sample inbox plus `extra` generated messages, for scrolling benchmarks.
pub fn large_inbox(extra: usize) -> Auth {
    inbox_with(SyncStatus::Synced, extra)
}

fn inbox_with(sync: SyncStatus, extra: usize) -> Auth {
    let samples = [
        (
            "Ana Lima",
            "ana@example.com",
            "Dinner on Friday?",
            "We're thinking 7pm at the usual place, let me know",
            true,
        ),
        (
            "GitHub",
            "noreply@github.com",
            "[kend] New issue: Search is too fast",
            "Somebody opened a new issue in your repository",
            true,
        ),
        (
            "Bo Chen",
            "bo@example.com",
            "Re: Quarterly report",
            "Thanks, the numbers look right to me. One question about",
            false,
        ),
    ];
    let mut messages: Vec<NewMessage> = samples
        .iter()
        .enumerate()
        .map(|(i, (name, email, subject, snippet, unread))| NewMessage {
            gmail_id: format!("sample-{i}"),
            from: Address::new(*name, *email),
            subject: (*subject).into(),
            snippet: (*snippet).into(),
            received_at: now_ms() - i as i64 * 40 * 60_000,
            label_ids: if *unread {
                vec!["INBOX".into(), "UNREAD".into(), "Label_2".into()]
            } else {
                vec!["INBOX".into()]
            },
            ..NewMessage::default()
        })
        .collect();
    messages.extend((0..extra).map(generated));
    let account = Account::new("you@gmail.com".into(), String::new());
    let session = Store::open_in_memory()
        .and_then(|mut store| {
            let id = store.upsert_account(&account.email)?;
            let label = |id: &str, name: &str| Label { id: id.into(), name: name.into(), kind: LabelKind::User };
            store.replace_labels(
                id,
                &[label("Label_1", "Receipts"), label("Label_2", "Work"), label("Label_3", "Work/Clients")],
            )?;
            store.upsert_messages(id, &messages)?;
            Session::open(1, store, &account.email, None)
        })
        .ok()
        .map(|mut session| {
            session.sync = sync;
            Box::new(session)
        });
    Auth::SignedIn { account, session }
}
