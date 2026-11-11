use kend_gmail::{Account, CancelToken};
use rune::{Align, Cx, El, col, el, row, text};
use rune_kit::shell::{SidebarToggle, SplitView, Toolbar};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit, Spinner};

use crate::app::{App, Auth};
use crate::compose::compose_view;
use crate::message_list::message_list;
use crate::reading::reading_view;
use crate::samples::{self, sample_inbox};
use crate::session::{Session, SyncStatus};
use crate::sidebar::{mailbox_name, sidebar, sync_status};

type Scene = (&'static str, fn(&mut App));

/// Named states for headless snapshots (`--snap out.png --scene inbox`).
pub const SCENES: &[Scene] = &[
    ("not-configured", |s| s.auth = Auth::NotConfigured),
    ("signed-out", |s| s.auth = Auth::SignedOut { error: None }),
    ("sign-in-failed", |s| s.auth = Auth::SignedOut { error: Some("sign-in timed out".into()) }),
    ("signing-in", |s| s.auth = Auth::SigningIn { attempt: 1, cancel: CancelToken::default() }),
    ("inbox", |s| s.auth = sample_inbox(SyncStatus::Synced)),
    ("syncing", |s| s.auth = sample_inbox(SyncStatus::Syncing { fetched: 200 })),
    ("sync-error", |s| {
        s.auth = sample_inbox(SyncStatus::Synced);
        if let Auth::SignedIn { session: Some(session), .. } = &mut s.auth {
            session.error = Some("Gmail asked kend to slow down".into());
        }
    }),
    ("large", |s| s.auth = samples::large_inbox(5_000)),
    ("reading", |s| {
        s.auth = sample_inbox(SyncStatus::Synced);
        s.open_message("sample-0");
    }),
    ("reading-html", |s| {
        s.auth = sample_inbox(SyncStatus::Synced);
        s.open_message("sample-1");
    }),
    ("compose", |s| {
        s.auth = sample_inbox(SyncStatus::Synced);
        s.compose_new();
    }),
    ("reply", |s| {
        s.auth = sample_inbox(SyncStatus::Synced);
        s.open_message("sample-0");
        s.reply(false);
    }),
    ("reading-loading", |s| {
        s.auth = sample_inbox(SyncStatus::Synced);
        s.open_message("sample-2");
    }),
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
        Auth::SignedIn { account, session: Some(session) } => {
            let inset = cx.titlebar_inset();
            return mail(s, &k, cx.system_dark(), inset, cx.is_reduced_motion(), account, session);
        }
    };
    col().w_full().h_full().center().fill(k.c.bg_base).child(content)
}

/// The signed-in window: the sidebar beside the selected mailbox's messages.
fn mail(
    s: &App,
    k: &Kit,
    dark: bool,
    titlebar_inset: f32,
    reduced_motion: bool,
    account: &Account,
    session: &Session,
) -> El {
    let title = text(mailbox_name(session)).key("mailbox-title").font(k.ty.ui_medium).color(k.c.text_base);
    let counts = text(counts(session.total, session.unread_in(&session.mailbox)))
        .key("counts")
        .font(k.ty.ui)
        .color(k.c.text_muted);
    let composing = session.compose.is_some();
    let reading = session.reading.as_ref().filter(|_| !composing);
    let ghost = |label: &str, help: &str, key: &str, press: fn(&mut App)| {
        Button::new(label).variant(ButtonVariant::Ghost).help(help).on_press(press).build(k).key(key.to_string())
    };
    let mut toolbar = Toolbar::new();
    if reading.is_some() {
        toolbar = toolbar.leading(ghost("Back", "Back to the list (Esc)", "back", |s| s.close_message()));
    }
    toolbar = toolbar.leading(row().gap(10.0).items_center().child(title).child(counts));
    if let Some(notice) = &session.notice {
        toolbar = toolbar.trailing(text(notice).key("notice").font(k.ty.ui).color(k.c.text_muted));
    }
    if let Some(reading) = reading {
        toolbar = toolbar.trailing(ghost("Reply", "Reply (R)", "reply", |s| s.reply(false))).trailing(ghost(
            "Forward",
            "Forward (F)",
            "forward",
            |s| s.reply(true),
        ));
        if reading.message.body_html.is_some() {
            toolbar = toolbar.trailing(ghost(
                "Open in Browser",
                "See this email exactly as designed",
                "open-in-browser",
                |s| s.open_in_browser(),
            ));
        }
    }
    if !composing {
        toolbar = toolbar.trailing(ghost("New Message", "New message (⌘N)", "new-message", |s| s.compose_new()));
    }
    let toolbar = toolbar
        .trailing(ghost("Sync now", &sync_status(session), "sync-now", |s| s.sync_now()))
        .trailing(ghost("Sign out", &format!("Signed in as {}", account.email), "sign-out", |s| s.sign_out()));
    let mut main = col().w_full().h_full().min_h(0.0);
    if let Some(error) = &session.error {
        main = main.child(sync_error(k, error));
    }
    let content = match (&session.compose, &session.reading) {
        (Some(compose), _) => compose_view(k, compose),
        (None, Some(reading)) => reading_view(k, dark, reading),
        (None, None) => message_list(k, dark, session),
    };
    let main = main.child(col().w_full().grow(1.0).min_h(0.0).child(content));
    SplitView::new(main)
        .sidebar(&s.sidebar, |s: &mut App| &mut s.sidebar, sidebar(k, &account.email, session))
        .toolbar(toolbar)
        .titlebar_inset(titlebar_inset)
        .frame_inset(8.0)
        .toolbar_in_panel()
        .sidebar_padding(12.0, 8.0)
        .sidebar_toggle(SidebarToggle::new().size(ControlSize::Large).past_lights(16.0).edge(18.0).gap(4.0).in_header())
        .reduced_motion(reduced_motion)
        .build(k)
}

/// A failed sync, in full, until a sync succeeds. kend retries on its own; the button is for
/// someone who fixed the cause (back online, say) and doesn't want to wait.
fn sync_error(k: &Kit, error: &str) -> El {
    let message = text(format!("Sync paused: {error}. kend will retry shortly."))
        .key("sync-error")
        .font(k.ty.ui)
        .color(k.c.danger.fg)
        .wrap()
        .grow(1.0)
        .min_w(0.0);
    let retry = Button::new("Try again")
        .variant(ButtonVariant::Ghost)
        .on_press(|s: &mut App| s.sync_now())
        .build(k)
        .key("sync-retry");
    row()
        .w_full()
        .px(16.0)
        .py(10.0)
        .gap(12.0)
        .items_center()
        .fill(k.c.danger.bg)
        .radius(k.radius.md)
        .child(message)
        .child(retry)
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

pub fn message(k: &Kit, s: &str) -> El {
    text(s).font(k.ty.ui).color(k.c.text_muted).wrap().align(Align::Center).w_full()
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
        for key in
            ["sample-0", "sample-1", "sample-2", "sync-now", "counts", "mailbox-INBOX", "mailbox-Label_3", "account"]
        {
            assert!(rt.find(key).is_some(), "{key} missing");
        }
    }

    #[test]
    fn choosing_a_label_shows_its_messages() {
        let mut rt = render("inbox");
        let (x, y) = rt.find("mailbox-Label_2").unwrap();
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(1.0 / 60.0);
        let Auth::SignedIn { session: Some(session), .. } = &rt.state.auth else { panic!("signed out") };
        assert_eq!(session.mailbox, "Label_2");
        assert_eq!(session.rows.len(), 2, "the two unread samples carry Work");
        assert!(rt.find("sample-2").is_none());
    }

    #[test]
    fn a_sync_error_stays_visible_with_a_retry() {
        let rt = render("sync-error");
        assert!(rt.find("sync-error").is_some());
        assert!(rt.find("sync-retry").is_some());
    }

    #[test]
    fn an_open_message_shows_its_body_and_a_way_back() {
        let rt = render("reading");
        for key in ["reading-subject", "reading-from", "reading-body", "back"] {
            assert!(rt.find(key).is_some(), "{key} missing");
        }
        assert!(rt.find("sample-1").is_none(), "the list is replaced while reading");
    }

    #[test]
    fn an_html_message_renders_with_a_browser_option() {
        let rt = render("reading-html");
        assert!(rt.find("md-sample-1").is_some(), "the Markdown body, keyed by its memo");
        assert!(rt.find("open-in-browser").is_some());
        assert!(render("reading").find("open-in-browser").is_none(), "plain text mail has no HTML to open");
    }

    #[test]
    fn a_message_without_a_stored_body_shows_it_loading() {
        let rt = render("reading-loading");
        assert!(rt.find("reading-loading").is_some());
    }

    #[test]
    fn compose_replaces_the_list_and_reply_prefills() {
        let rt = render("compose");
        for key in ["compose-to", "compose-cc", "compose-subject", "compose-body", "send", "discard"] {
            assert!(rt.find(key).is_some(), "{key} missing");
        }
        assert!(rt.find("new-message").is_none());
        let rt = render("reply");
        let Auth::SignedIn { session: Some(session), .. } = &rt.state.auth else { panic!() };
        let compose = session.compose.as_ref().unwrap();
        assert_eq!(compose.subject.text, "Re: Dinner on Friday?");
        assert_eq!(compose.to.text, "Ana Lima <ana@example.com>");
    }

    #[test]
    fn sending_with_no_recipient_explains_why() {
        let mut rt = render("compose");
        let (x, y) = rt.find("send").unwrap();
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(1.0 / 60.0);
        assert!(rt.find("compose-error").is_some());
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
