//! Sample mail for headless snapshots and benchmarks: an in-memory database, no network.

use kend_gmail::Account;
use kend_store::{Address, Label, LabelKind, NewMessage, Store};

use crate::app::Auth;
use crate::session::{Session, SyncStatus};

const SENDERS: [&str; 8] = ["Ana Lima", "Bo Chen", "GitHub", "Priya Nair", "Linear", "Sam Ortiz", "Stripe", "Mei Tan"];
const SNIPPETS: [&str; 8] = [
    "Notes from today: we agreed to ship the sidebar first and revisit search next week",
    "Thanks for your payment. Here's a summary of what you were charged this month",
    "I left a few comments on the spacing in the header. Otherwise it looks great",
    "The build broke on the latest commit. 3 tests failed in the store crate",
    "Your trip is confirmed. Check in opens 24 hours before departure",
    "Before I sign, can you confirm the start date in section 4?",
    "Uploaded the album, the sunset ones came out really well",
    "This is a reminder that your invoice is due in 5 days",
];
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

/// An email laid out the way real ones are: nested tables, a hidden preview, a tracking pixel.
const NEWSLETTER: &str = r#"<html><head><style>td{font-family:Arial}</style></head><body>
<div style="display:none;max-height:0">New issue in kend: Search is too fast &zwnj;&zwnj;&zwnj;</div>
<table width="100%"><tr><td align="center"><table width="600"><tr><td>
<img src="https://github.example/logo.png" alt="GitHub">
<h2>New issue: Search is too fast</h2>
<p><b>@somebody</b> opened an issue in <a href="https://github.example/kend">kend</a>:</p>
<blockquote>Results show up before I finish typing. Is this a bug?</blockquote>
<ul><li>Version: 0.1.0</li><li>OS: macOS 27</li></ul>
<table><tr><th>Label</th><th>Count</th></tr><tr><td>bug</td><td>1</td></tr><tr><td>question</td><td>3</td></tr></table>
<p><a href="https://github.example/kend/issues/1">View it on GitHub</a></p>
</td></tr></table></td></tr></table>
<img src="https://github.example/track.gif" width="1" height="1">
</body></html>"#;

/// A message `i` steps older than the samples, with varied senders, subjects and read state.
fn generated(i: usize) -> NewMessage {
    let unread = i.is_multiple_of(7);
    NewMessage {
        gmail_id: format!("gen-{i:05}"),
        from: Address::new(SENDERS[i % SENDERS.len()], "someone@example.com"),
        subject: SUBJECTS[(i * 5 + i / 8) % SUBJECTS.len()].into(),
        snippet: SNIPPETS[(i * 5 + i / 8) % SNIPPETS.len()].into(),
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
    messages[0].to = "you@gmail.com".into();
    messages[0].body_text = Some(
        "Hi!\n\nWe're thinking 7pm at the usual place on Friday. Sam and Priya are in, and Bo might come \
         late after his flight lands.\n\nLet me know if that works, or if Saturday is better for you. I can \
         book a table either way.\n\nAna"
            .into(),
    );
    messages[1].body_text = Some("Somebody opened a new issue.".into());
    messages[1].body_html = Some(NEWSLETTER.into());
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
            Session::open(1, store, &account.email, None, None)
        })
        .ok()
        .map(|mut session| {
            session.sync = sync;
            Box::new(session)
        });
    Auth::SignedIn { account, session }
}
