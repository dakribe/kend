use kend_gmail::{Account, CancelToken};
use kend_store::{Address, MessageSummary, NewMessage, Store};
use rune::{Align, Cx, El, ScrollSpec, col, el, row, text};
use rune_kit::{Button, ButtonVariant, Kit, Spinner};

use crate::app::{App, Auth};
use crate::session::{Session, SyncStatus};

type Scene = (&'static str, fn(&mut App));

/// Named states for headless snapshots (`--snap out.png --scene inbox`).
pub const SCENES: &[Scene] = &[
    ("not-configured", |s| s.auth = Auth::NotConfigured),
    ("signed-out", |s| s.auth = Auth::SignedOut { error: None }),
    ("sign-in-failed", |s| s.auth = Auth::SignedOut { error: Some("sign-in timed out".into()) }),
    ("signing-in", |s| s.auth = Auth::SigningIn { attempt: 1, cancel: CancelToken::default() }),
    ("inbox", |s| s.auth = sample_inbox(SyncStatus::Synced)),
    ("syncing", |s| s.auth = sample_inbox(SyncStatus::Syncing { fetched: 200 })),
];

pub fn view(s: &App, cx: &mut Cx<App>) -> El {
    let k = Kit::neutral(cx.system_dark());
    let content = match &s.auth {
        Auth::Loading | Auth::SignedIn { session: None, .. } => el(),
        Auth::NotConfigured => panel(
            &k,
            "Gmail isn't set up",
            "Add your Google client ID and secret to .env (see .env.example), then rebuild.",
        ),
        Auth::SignedOut { error } => {
            let mut p = panel(&k, "Welcome to kend", "Sign in with your Google account to load your mail.").child(
                Button::new("Sign in with Google")
                    .variant(ButtonVariant::Contrast)
                    .on_press(|s: &mut App| s.start_sign_in())
                    .build(&k)
                    .key("sign-in"),
            );
            if let Some(error) = error {
                p = p.child(message(&k, error).color(k.c.danger.fg).key("error"));
            }
            p
        }
        Auth::SigningIn { .. } => {
            panel(&k, "Finish signing in in your browser", "kend will continue once you allow access.")
                .child(Spinner::new().build(&k))
                .child(
                    Button::new("Cancel")
                        .variant(ButtonVariant::Ghost)
                        .on_press(|s: &mut App| s.cancel_sign_in())
                        .build(&k)
                        .key("cancel"),
                )
        }
        Auth::SignedIn { account, session: Some(session) } => return inbox(&k, account, session),
    };
    col().w_full().h_full().center().fill(k.c.bg_base).child(content)
}

fn inbox(k: &Kit, account: &Account, session: &Session) -> El {
    let title = text("Inbox").font(k.ty.ui_medium).color(k.c.text_base);
    let unread = text(counts(session.total, session.unread)).key("unread").font(k.ty.ui).color(k.c.text_muted);
    let status = text(sync_status(&session.sync)).key("sync-status").font(k.ty.ui).color(match session.sync {
        SyncStatus::Failed(_) => k.c.danger.fg,
        _ => k.c.text_faint,
    });
    let header = row()
        .w_full()
        .h(52.0)
        .px(20.0)
        .gap(12.0)
        .items_center()
        .child(title)
        .child(unread)
        .child(el().grow(1.0))
        .child(status)
        .child(
            Button::new("Sync now")
                .variant(ButtonVariant::Ghost)
                .on_press(|s: &mut App| s.sync_now())
                .build(k)
                .key("sync-now"),
        )
        .child(
            Button::new("Sign out")
                .variant(ButtonVariant::Ghost)
                .help(format!("Signed in as {}", account.email))
                .on_press(|s: &mut App| s.sign_out())
                .build(k)
                .key("sign-out"),
        );
    let divider = el().w_full().h(1.0).fill(k.c.border_muted);

    let body = if session.rows.is_empty() {
        let note = if matches!(session.sync, SyncStatus::Syncing { .. }) {
            "Loading your inbox…"
        } else {
            "Your inbox is empty."
        };
        col().w_full().grow(1.0).center().child(message(k, note).key("empty"))
    } else {
        let rows = col().w_full().children(session.rows.iter().map(|m| message_row(k, m)));
        col().key("messages").w_full().grow(1.0).min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).child(rows)
    };
    col().w_full().h_full().fill(k.c.bg_base).child(header).child(divider).child(body)
}

fn message_row(k: &Kit, m: &MessageSummary) -> El {
    let (weight, ink) = if m.unread { (k.ty.ui_medium, k.c.text_base) } else { (k.ty.ui, k.c.text_muted) };
    let dot = el().size(8.0, 8.0).radius(4.0).fill(if m.unread { k.c.text_accent } else { k.c.bg_base });
    let from = text(m.from.display()).font(weight).color(ink).truncate().w(200.0);
    let subject = if m.subject.is_empty() { "(no subject)" } else { &m.subject };
    let line = row()
        .grow(1.0)
        .min_w(0.0)
        .gap(8.0)
        .child(text(subject).font(weight).color(ink).truncate().shrink())
        .child(text(&m.snippet).font(k.ty.ui).color(k.c.text_faint).truncate().grow(1.0).min_w(0.0));
    row().key(m.gmail_id.clone()).w_full().h(40.0).px(20.0).gap(12.0).items_center().child(dot).child(from).child(line)
}

fn sync_status(status: &SyncStatus) -> String {
    match status {
        SyncStatus::Syncing { fetched: 0 } => "Syncing…".into(),
        SyncStatus::Syncing { fetched } => format!("Syncing… {fetched} messages"),
        SyncStatus::Synced => "Up to date".into(),
        SyncStatus::Failed(err) => format!("Sync paused: {err}. Retrying shortly"),
    }
}

/// "1,234 messages · 213 unread".
fn counts(total: u64, unread: u64) -> String {
    let noun = if total == 1 { "message" } else { "messages" };
    match unread {
        0 => format!("{} {noun}", thousands(total)),
        n => format!("{} {noun} · {} unread", thousands(total), thousands(n)),
    }
}

fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn panel(k: &Kit, title: &str, body: &str) -> El {
    let title = text(title).key("title").font(k.ty.ui_medium).color(k.c.text_base);
    col().w(360.0).center().gap(12.0).child(title).child(message(k, body).key("body"))
}

fn message(k: &Kit, s: &str) -> El {
    text(s).font(k.ty.ui).color(k.c.text_muted).wrap().align(Align::Center).w_full()
}

/// A signed-in session over an in-memory database with a few messages, for snapshots.
fn sample_inbox(sync: SyncStatus) -> Auth {
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
    let messages: Vec<NewMessage> = samples
        .iter()
        .enumerate()
        .map(|(i, (name, email, subject, snippet, unread))| NewMessage {
            gmail_id: format!("sample-{i}"),
            from: Address::new(*name, *email),
            subject: (*subject).into(),
            snippet: (*snippet).into(),
            received_at: 1_000 - i as i64,
            label_ids: if *unread { vec!["INBOX".into(), "UNREAD".into()] } else { vec!["INBOX".into()] },
            ..NewMessage::default()
        })
        .collect();
    let account = Account::new("you@gmail.com".into(), String::new());
    let session = Store::open_in_memory()
        .and_then(|mut store| {
            let id = store.upsert_account(&account.email)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use rune::{Env, Runtime};

    fn render(scene: &str) -> Runtime<App> {
        let mut app = App::with_client(None);
        let (_, set) = SCENES.iter().find(|(name, _)| *name == scene).unwrap();
        set(&mut app);
        let mut rt = Runtime::new(app, view, (1100.0, 720.0), Env { system_dark: true, scale: 2.0 });
        rt.frame(0.0);
        rt
    }

    #[test]
    fn each_sign_in_state_shows_its_controls() {
        assert!(render("not-configured").find("sign-in").is_none());
        assert!(render("signed-out").find("sign-in").is_some());
        assert!(render("sign-in-failed").find("error").is_some());
        assert!(render("signing-in").find("cancel").is_some());
        assert!(render("inbox").find("sign-out").is_some());
    }

    #[test]
    fn the_inbox_lists_stored_messages() {
        let rt = render("inbox");
        for key in ["sample-0", "sample-1", "sample-2", "sync-now", "unread"] {
            assert!(rt.find(key).is_some(), "{key} missing");
        }
    }

    #[test]
    fn counts_read_naturally() {
        assert_eq!(counts(1, 0), "1 message");
        assert_eq!(counts(500, 213), "500 messages · 213 unread");
        assert_eq!(counts(1_234_567, 1_000), "1,234,567 messages · 1,000 unread");
    }

    #[test]
    fn cancel_returns_to_the_sign_in_button() {
        let mut rt = render("signing-in");
        let (x, y) = rt.find("cancel").unwrap();
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(1.0 / 60.0);
        assert!(matches!(rt.state.auth, Auth::SignedOut { error: None }));
    }
}
