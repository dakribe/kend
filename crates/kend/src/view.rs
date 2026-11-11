use kend_gmail::{Account, CancelToken};
use rune::{Align, Cx, El, col, el, row, text};
use rune_kit::shell::{SidebarToggle, SplitView, Toolbar};
use rune_kit::{
    Button, ButtonVariant, ControlSize, Dialog, IconButton, IconButtonVariant, Keybind, KeybindVariant, Kit, Spinner,
    TextInput, glyphs,
};

use crate::actions::action_bar;
use crate::app::instant;
use crate::app::{App, Auth};
use crate::compose::compose_view;
use crate::icons;
use crate::message_list::message_list;
use crate::reading::reading_view;
use crate::samples::{self, sample_inbox};
use crate::session::{Session, SyncStatus};
use crate::sidebar::{mailbox_name, sidebar};

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
    ("search", |s| {
        s.auth = samples::large_inbox(400);
        if let Auth::SignedIn { session: Some(session), .. } = &mut s.auth {
            session.search = rune::TextEdit::new("invoice");
            session.search_changed();
        }
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
    ("selecting", |s| {
        s.auth = sample_inbox(SyncStatus::Synced);
        if let Auth::SignedIn { session: Some(session), .. } = &mut s.auth {
            session.toggle_checked("sample-0");
            session.check_range("sample-2");
        }
    }),
    ("palette", |s| {
        s.auth = sample_inbox(SyncStatus::Synced);
        s.palette.open = true;
    }),
    ("help", |s| {
        s.auth = sample_inbox(SyncStatus::Synced);
        s.help = true;
    }),
    ("sender", |s| {
        s.auth = samples::large_inbox(40);
        s.open_message("sample-1");
        s.show_sender();
    }),
    ("sign-out", |s| {
        s.auth = sample_inbox(SyncStatus::Synced);
        s.confirm_sign_out = true;
    }),
    ("discarded", |s| {
        s.auth = sample_inbox(SyncStatus::Synced);
        s.compose_new();
        s.discard_compose();
    }),
    ("reading-loading", |s| {
        s.auth = sample_inbox(SyncStatus::Synced);
        s.open_message("sample-2");
    }),
];

/// kend's motion rule: every action lands at once; only loading indicators move. The root
/// lands layout, entrances and exits instantly (`instant_changes`), and the clock runs at
/// rune's fastest so colour changes (hover, selection) finish within a frame. Spinners run in
/// a subtree slowed back to normal speed (`spinner`).
pub const CLOCK: f32 = 8.0;

pub fn view(s: &App, cx: &mut Cx<App>) -> El {
    cx.time_scale(CLOCK);
    let k = kit(cx.system_dark());
    let mut layer = Vec::new();
    shortcuts(&k, s.help, &mut layer);
    sign_out_dialog(&k, s, &mut layer);
    crate::palette::palette_view(&k, s, &mut layer);
    instant_root(rune_kit::root(build(s, cx), layer))
}

/// A shortcut's keys (any of them works) and what it does.
type Shortcut = (&'static [&'static str], &'static str);

/// Every shortcut, grouped as the help dialog lists them.
const SHORTCUTS: &[(&str, &[Shortcut])] = &[
    (
        "Moving around",
        &[
            (&["J", "↓"], "Next message"),
            (&["K", "↑"], "Previous message"),
            (&["Return"], "Open message"),
            (&["Esc"], "Back to the list, or clear the selection"),
            (&["⌘F", "/"], "Search"),
            (&["⇧⌘B"], "Show or hide the sidebar"),
        ],
    ),
    (
        "Selecting",
        &[
            (&["X"], "Select or deselect message"),
            (&["⇧J", "⇧↓"], "Extend selection down"),
            (&["⇧K", "⇧↑"], "Extend selection up"),
            (&["⌘A"], "Select all"),
            (&["⌘ click"], "Select or deselect message"),
            (&["⇧ click"], "Select a range"),
        ],
    ),
    (
        "Actions",
        &[
            (&["E"], "Archive"),
            (&["#", "Delete"], "Move to Trash"),
            (&["S"], "Star or unstar"),
            (&["⇧U"], "Mark as unread"),
            (&["⇧I"], "Mark as read"),
            (&["Z"], "Undo, or bring back a discarded draft"),
            (&["O"], "Open in Browser"),
            (&["R"], "Reply"),
            (&["F"], "Forward"),
            (&["⇧F"], "All mail from this sender"),
            (&["⌘N"], "New message"),
            (&["⌘Return"], "Send"),
            (&["⌘K"], "Command palette"),
            (&["?"], "Show these shortcuts"),
        ],
    ),
];

/// The keyboard shortcuts dialog (?).
fn shortcuts(k: &Kit, open: bool, layer: &mut Vec<El>) {
    let mut body = col().key("shortcuts").w_full().px(16.0).pb(16.0).gap(16.0);
    for (group, keys) in SHORTCUTS {
        let mut section =
            col().key(*group).w_full().gap(6.0).child(text(*group).font(k.ty.ui_medium).color(k.c.text_base));
        for (keys, what) in *keys {
            let mut caps = row().gap(6.0).items_center().w(120.0);
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    caps = caps.child(text("or").font(k.ty.small).color(k.c.text_faint));
                }
                caps = caps.child(Keybind::new(&[*key]).build(k));
            }
            section = section.child(
                row()
                    .w_full()
                    .gap(12.0)
                    .items_center()
                    .child(caps)
                    .child(text(*what).font(k.ty.ui).color(k.c.text_muted)),
            );
        }
        body = body.child(section);
    }
    Dialog::new("help", "Keyboard shortcuts")
        .open(open)
        .width(440.0)
        .fit()
        .on_close(|s: &mut App| s.help = false)
        .child(body)
        .build(k, layer);
}

fn instant_root(root: El) -> El {
    root.instant_changes()
}

/// A loading spinner at normal speed under the fast clock.
pub fn spinner(k: &Kit) -> El {
    col().key("spinner").time_scale(1.0 / CLOCK).child(Spinner::new().build(k))
}

/// The neutral kit with its motion springs made instant, so rune-kit's own controls (sidebar
/// selection, disclosure, panes) follow the same rule.
pub fn kit(dark: bool) -> Kit {
    let mut k = Kit::neutral(dark);
    let instant = rune::Spring::new(0.001, 0.0);
    k.motion.control = instant;
    k.motion.overlay = instant;
    k.motion.disclosure = instant;
    k
}

fn build(s: &App, cx: &mut Cx<App>) -> El {
    let k = kit(cx.system_dark());
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
                    .on_press_tx(instant(|s: &mut App| s.start_sign_in()))
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
                .child(spinner(&k))
                .child(
                    Button::new("Cancel")
                        .variant(ButtonVariant::Ghost)
                        .on_press_tx(instant(|s: &mut App| s.cancel_sign_in()))
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
    let searching = session.results.as_ref();
    let title = match (searching, &session.sender) {
        (Some(_), _) => "Search",
        (None, Some(sender)) => sender.address.display(),
        (None, None) => mailbox_name(session),
    };
    let composing = session.compose.is_some();
    let reading = session.reading.is_some();
    let counts = text(match (searching, &session.sender) {
        (Some(results), _) => results_count(results.len()),
        (None, Some(sender)) => counts(session.total, sender.unread),
        (None, None) => counts(session.total, session.unread_in(&session.mailbox)),
    })
    .key("counts")
    .font(k.ty.ui)
    .color(k.c.text_muted);
    // Where you are. From an open message, or from one sender's mail, the place you came from
    // is a button that takes you back (Esc), in the spot the title held.
    let mut leading = row().gap(10.0).items_center().min_w(0.0);
    if reading {
        leading = leading.child(back(k, title, "back", |s| s.close_message()));
    } else if session.sender.is_some() && searching.is_none() && !composing {
        leading = leading
            .child(back(k, mailbox_name(session), "leave-sender", |s| s.leave_sender()))
            .child(title_text(k, title));
    } else {
        leading = leading.child(title_text(k, title));
    }
    let leading = leading.child(counts);
    let mut toolbar = Toolbar::new().leading(leading);
    if !composing {
        let search = TextInput::new(&session.search)
            .placeholder("Search mail  ⌘F")
            .label("Search mail")
            .leading_icon(k.glyph.search)
            .clear_button()
            .width(Some(260.0))
            .on_edit(|s: &mut App| s.search_field())
            .on_change(|s: &mut App, tx: &mut rune::Tx| {
                tx.instant();
                s.search_changed()
            })
            .on_key(|s: &mut App, event: &rune::KeyEvent, tx: &mut rune::Tx| s.search_key(event, tx))
            .build(k)
            .key("search");
        let new_message = crate::actions::Action {
            icon: &icons::COMPOSE,
            label: "New Message",
            help: "New message",
            keys: &["⌘N"],
            key: "new-message",
            enabled: true,
            press: |s| s.compose_new(),
        };
        let help = IconButton::new(&glyphs::KEYBOARD)
            .variant(IconButtonVariant::Ghost)
            .label("Keyboard Shortcuts")
            .help("Keyboard shortcuts")
            .shortcut("?")
            .on_press_tx(instant(|s: &mut App| s.toggle_help()))
            .build(k)
            .key("help");
        toolbar = toolbar.center(search).trailing(new_message.build(k)).trailing(help);
    }
    let mut main = col().w_full().h_full().min_h(0.0);
    if let Some(error) = &session.error {
        main = main.child(sync_error(k, error));
    }
    // The action bar comes first, so its buttons sit at the same height on every screen.
    if !composing {
        main = main.child(action_bar(k, session)).child(el().w_full().h(1.0).fill(k.c.border_muted));
    }
    if let Some(sender) = &session.sender
        && !reading
        && !composing
        && searching.is_none()
    {
        main = main.child(sender_strip(k, &sender.address.email));
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

/// Above a sender's mail: the address, since a newsletter's name rarely says where it comes
/// from. The way back is in the toolbar.
fn sender_strip(k: &Kit, email: &str) -> El {
    row().key("sender-strip").w_full().px(16.0).py(6.0).items_center().child(
        text(format!("All mail from {email}"))
            .key("sender-email")
            .font(k.ty.ui)
            .color(k.c.text_muted)
            .truncate()
            .grow(1.0)
            .min_w(0.0),
    )
}

/// The toolbar's title.
fn title_text(k: &Kit, title: &str) -> El {
    text(title).key("mailbox-title").font(k.ty.ui_medium).color(k.c.text_base).truncate().max_w(240.0)
}

/// "‹ Inbox  Esc": the way back to where you were.
fn back(k: &Kit, to: &str, key: &str, press: fn(&mut App)) -> El {
    let label = to.to_string();
    let cap = Keybind::new(&["Esc"]).variant(KeybindVariant::Ghost).build(k);
    Button::new(format!("Back to {to}"))
        .variant(ButtonVariant::Ghost)
        .icon(&icons::BACK)
        .help(format!("Back to {to}"))
        .shortcut("Esc")
        .label_with(move |font, ink| {
            row().gap(6.0).items_center().max_w(220.0).child(text(label).font(font).color(ink).truncate()).child(cap)
        })
        .on_press_tx(instant(press))
        .build(k)
        .key(key.to_string())
}

/// Sign Out deletes this account's mail from this Mac, so it asks first and says so.
fn sign_out_dialog(k: &Kit, s: &App, layer: &mut Vec<El>) {
    let email = match &s.auth {
        Auth::SignedIn { account, .. } => account.email.clone(),
        _ => String::new(),
    };
    let body = col()
        .key("sign-out-confirm")
        .w_full()
        .px(16.0)
        .pb(16.0)
        .gap(16.0)
        .child(
            text(format!(
                "kend will remove {email}'s mail from this Mac and forget its sign-in. Your mail stays in Gmail; \
                 signing in again downloads it again."
            ))
            .font(k.ty.ui)
            .color(k.c.text_muted)
            .wrap(),
        )
        .child(
            row()
                .w_full()
                .gap(8.0)
                .child(el().grow(1.0))
                .child(
                    Button::new("Cancel")
                        .variant(ButtonVariant::Ghost)
                        .shortcut("Esc")
                        .on_press_tx(instant(|s: &mut App| s.confirm_sign_out = false))
                        .build(k)
                        .key("sign-out-cancel"),
                )
                .child(
                    Button::new("Sign Out and Remove Mail")
                        .variant(ButtonVariant::Danger)
                        .on_press_tx(instant(|s: &mut App| s.sign_out()))
                        .build(k)
                        .key("sign-out-confirmed"),
                ),
        );
    Dialog::new("sign-out", "Sign out of kend?")
        .open(s.confirm_sign_out)
        .width(400.0)
        .fit()
        .on_close(|s: &mut App| s.confirm_sign_out = false)
        .child(body)
        .build(k, layer);
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
        .on_press_tx(instant(|s: &mut App| s.sync_now()))
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

/// "1 result", "200+ results": the search shows at most 200.
fn results_count(n: usize) -> String {
    match n {
        1 => "1 result".into(),
        n if n >= 200 => "200+ results".into(),
        n => format!("{n} results"),
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
        // As `main` wires it, so keys reach kend's shortcuts.
        rt.on_key = Some(App::on_key);
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

    /// Where everything visible sits and how it looks, by key (or path for unkeyed elements).
    /// Off-screen rows are left out (the virtual list mounts those a few per frame on purpose),
    /// and so are spinners, the one thing allowed to move.
    fn pose(rt: &Runtime<App>) -> Vec<String> {
        let mut pose: Vec<String> = rt
            .elements()
            .into_iter()
            .filter(|e| e.visible[2] > 0.0 && e.visible[3] > 0.0 && !e.path.contains("spinner"))
            .map(|e| {
                let name = e.key.clone().unwrap_or(e.path.clone());
                let r = e.rect.map(|v| v.round());
                format!("{name} {r:?} {:?} {:?} {:.2}", e.fill, e.text_color, e.opacity)
            })
            .collect();
        pose.sort();
        pose
    }

    /// Runs `act`, draws one frame, and checks ten more frames change nothing.
    fn lands_in_one_frame(rt: &mut Runtime<App>, what: &str, act: impl FnOnce(&mut Runtime<App>)) {
        act(rt);
        rt.frame(1.0 / 60.0);
        let first = pose(rt);
        for _ in 0..10 {
            rt.frame(1.0 / 60.0);
        }
        let later = pose(rt);
        let moved: Vec<_> = later.iter().filter(|p| !first.contains(p)).collect();
        assert!(moved.is_empty(), "{what} was still animating after one frame: {moved:#?}");
    }

    fn key(rt: &mut Runtime<App>, key: rune::Key, mods: rune::Mods) {
        rt.key(rune::KeyEvent { key, mods });
    }

    fn click(rt: &mut Runtime<App>, target: &str) {
        let (x, y) = rt.find(target).unwrap_or_else(|| panic!("{target} not found"));
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
    }

    #[test]
    fn every_action_lands_at_once() {
        let mut rt = render("large");
        for _ in 0..30 {
            rt.frame(1.0 / 60.0);
        }
        let plain = rune::Mods::default();
        for _ in 0..20 {
            lands_in_one_frame(&mut rt, "J", |rt| key(rt, rune::Key::Text("j".into()), plain));
        }
        lands_in_one_frame(&mut rt, "K", |rt| key(rt, rune::Key::Text("k".into()), plain));
        lands_in_one_frame(&mut rt, "opening a message", |rt| key(rt, rune::Key::Enter, plain));
        lands_in_one_frame(&mut rt, "Escape", |rt| key(rt, rune::Key::Escape, plain));
        lands_in_one_frame(&mut rt, "opening a row by click", |rt| click(rt, "sample-0"));
        lands_in_one_frame(&mut rt, "Back", |rt| click(rt, "back"));
        lands_in_one_frame(&mut rt, "switching mailbox", |rt| click(rt, "mailbox-Label_2"));
        let toggle = rune::Mods { shift: true, cmd: true, ..rune::Mods::default() };
        lands_in_one_frame(&mut rt, "hiding the sidebar", |rt| key(rt, rune::Key::Command('b'), toggle));
        lands_in_one_frame(&mut rt, "showing the sidebar", |rt| key(rt, rune::Key::Command('b'), toggle));
        lands_in_one_frame(&mut rt, "back to the inbox", |rt| click(rt, "mailbox-INBOX"));
        lands_in_one_frame(&mut rt, "New Message", |rt| click(rt, "new-message"));
        lands_in_one_frame(&mut rt, "Discard", |rt| click(rt, "discard"));
        lands_in_one_frame(&mut rt, "opening to reply", |rt| click(rt, "sample-0"));
        lands_in_one_frame(&mut rt, "Reply", |rt| click(rt, "reply"));
        lands_in_one_frame(&mut rt, "discarding the reply", |rt| click(rt, "discard"));
        lands_in_one_frame(&mut rt, "selecting", |rt| key(rt, rune::Key::Text("j".into()), plain));
        lands_in_one_frame(&mut rt, "archiving", |rt| key(rt, rune::Key::Text("e".into()), plain));
        lands_in_one_frame(&mut rt, "deleting", |rt| key(rt, rune::Key::Text("#".into()), plain));
        lands_in_one_frame(&mut rt, "undoing", |rt| key(rt, rune::Key::Text("z".into()), plain));
        lands_in_one_frame(&mut rt, "starring", |rt| key(rt, rune::Key::Text("s".into()), plain));
        lands_in_one_frame(&mut rt, "clicking a star", |rt| click(rt, "star"));
        lands_in_one_frame(&mut rt, "marking unread", |rt| key(rt, rune::Key::Text("U".into()), plain));
        lands_in_one_frame(&mut rt, "marking read", |rt| key(rt, rune::Key::Text("I".into()), plain));
        lands_in_one_frame(&mut rt, "Delete button", |rt| click(rt, "delete"));
        lands_in_one_frame(&mut rt, "checking a row", |rt| key(rt, rune::Key::Text("x".into()), plain));
        lands_in_one_frame(&mut rt, "extending the selection", |rt| key(rt, rune::Key::Text("J".into()), plain));
        lands_in_one_frame(&mut rt, "clicking a checkbox", |rt| click(rt, "check"));
        lands_in_one_frame(&mut rt, "clearing the selection", |rt| click(rt, "clear-checked"));
        lands_in_one_frame(&mut rt, "opening help", |rt| key(rt, rune::Key::Text("?".into()), plain));
        lands_in_one_frame(&mut rt, "closing help", |rt| key(rt, rune::Key::Escape, plain));
        let cmd = rune::Mods { cmd: true, ..rune::Mods::default() };
        lands_in_one_frame(&mut rt, "opening the palette", |rt| key(rt, rune::Key::Command('k'), cmd));
        lands_in_one_frame(&mut rt, "filtering the palette", |rt| key(rt, rune::Key::Text("s".into()), plain));
        lands_in_one_frame(&mut rt, "moving in the palette", |rt| key(rt, rune::Key::Down, plain));
        lands_in_one_frame(&mut rt, "closing the palette", |rt| key(rt, rune::Key::Escape, plain));
        lands_in_one_frame(&mut rt, "showing a sender's mail", |rt| key(rt, rune::Key::Text("F".into()), plain));
        lands_in_one_frame(&mut rt, "leaving the sender", |rt| key(rt, rune::Key::Escape, plain));
        lands_in_one_frame(&mut rt, "opening to see the sender", |rt| click(rt, "sample-0"));
        lands_in_one_frame(&mut rt, "clicking the sender", |rt| click(rt, "show-sender"));
        lands_in_one_frame(&mut rt, "Back to Inbox", |rt| click(rt, "leave-sender"));
        lands_in_one_frame(&mut rt, "focusing search", |rt| click(rt, "search"));
        lands_in_one_frame(&mut rt, "typing a search", |rt| key(rt, rune::Key::Text("w".into()), plain));
        lands_in_one_frame(&mut rt, "clearing the search", |rt| key(rt, rune::Key::Escape, plain));
        lands_in_one_frame(&mut rt, "selecting again", |rt| key(rt, rune::Key::Text("j".into()), plain));
        lands_in_one_frame(&mut rt, "Star button", |rt| click(rt, "toggle-star"));
        lands_in_one_frame(&mut rt, "Unread button", |rt| click(rt, "toggle-read"));
        lands_in_one_frame(&mut rt, "Archive button", |rt| click(rt, "archive"));
        lands_in_one_frame(&mut rt, "Undo button", |rt| click(rt, "undo"));
        lands_in_one_frame(&mut rt, "asking to sign out", |rt| click(rt, "sign-out"));
        lands_in_one_frame(&mut rt, "cancelling sign out", |rt| click(rt, "sign-out-cancel"));
    }

    /// Archive and Delete are in the same place in the list and in an open message.
    #[test]
    fn triage_buttons_stay_put_when_a_message_opens() {
        let list = render("selecting");
        let reading = render("reading");
        let sender = render("sender");
        for target in ["archive", "delete", "toggle-star", "toggle-read"] {
            assert_eq!(list.find(target), reading.find(target), "{target} moved");
            assert_eq!(list.find(target), sender.find(target), "{target} moved");
        }
    }

    #[test]
    fn a_discarded_draft_comes_back_with_undo() {
        let mut rt = render("compose");
        click(&mut rt, "discard");
        rt.frame(0.0);
        assert!(rt.find("compose").is_none());
        assert!(rt.find("undo").is_some());
        key(&mut rt, rune::Key::Text("z".into()), rune::Mods::default());
        rt.frame(0.0);
        assert!(rt.find("compose").is_some());
    }

    #[test]
    fn sign_out_asks_first() {
        let mut rt = render("inbox");
        click(&mut rt, "sign-out");
        rt.frame(0.0);
        assert!(rt.find("sign-out-confirm").is_some());
        assert!(rt.find("mailbox-INBOX").is_some(), "still signed in until confirmed");
        key(&mut rt, rune::Key::Escape, rune::Mods::default());
        rt.frame(0.0);
        assert!(rt.find("sign-out-confirm").is_none());
    }

    #[test]
    fn loading_still_spins_at_normal_speed() {
        let mut rt = render("reading-loading");
        let mut poses = Vec::new();
        // A quarter second, sampled every 1/60 s.
        for _ in 0..15 {
            rt.frame(1.0 / 60.0);
            let spinner: Vec<String> = rt
                .elements()
                .into_iter()
                .filter(|e| e.path.contains("spinner"))
                .map(|e| format!("{:?} {:.2}", e.rect, e.opacity))
                .collect();
            assert!(!spinner.is_empty(), "the spinner shows");
            poses.push(spinner);
        }
        let changes = poses.windows(2).filter(|w| w[0] != w[1]).count();
        assert!(changes > 0, "the spinner moves while a message loads");
        // At 8× speed it would cycle several times in this window; at 1× the frames keep
        // changing smoothly instead of repeating a short loop.
        let distinct: std::collections::HashSet<_> = poses.iter().collect();
        assert!(distinct.len() > 4, "only {} distinct frames", distinct.len());
    }

    fn type_text(rt: &mut Runtime<App>, text: &str) {
        for c in text.chars() {
            key(rt, rune::Key::Text(c.to_string()), rune::Mods::default());
            rt.frame(1.0 / 60.0);
        }
    }

    #[test]
    fn searching_shows_matches_and_escape_clears() {
        let mut rt = render("inbox");
        key(&mut rt, rune::Key::Command('f'), rune::Mods { cmd: true, ..rune::Mods::default() });
        rt.frame(1.0 / 60.0);
        type_text(&mut rt, "quarterly");
        assert!(rt.find("sample-2").is_some(), "Bo's quarterly report matches");
        assert!(rt.find("sample-0").is_none(), "dinner doesn't");
        let Auth::SignedIn { session: Some(session), .. } = &rt.state.auth else { panic!() };
        assert_eq!(session.results.as_ref().map(Vec::len), Some(1));

        key(&mut rt, rune::Key::Escape, rune::Mods::default());
        rt.frame(1.0 / 60.0);
        assert!(rt.find("sample-0").is_some(), "the inbox is back");
        let Auth::SignedIn { session: Some(session), .. } = &rt.state.auth else { panic!() };
        assert!(session.results.is_none() && session.search.text.is_empty());
    }

    #[test]
    fn a_search_with_no_matches_says_so() {
        let mut rt = render("inbox");
        key(&mut rt, rune::Key::Text("/".into()), rune::Mods::default());
        rt.frame(1.0 / 60.0);
        type_text(&mut rt, "zebra");
        let empty = rt.elements().into_iter().find(|e| e.key.as_deref() == Some("empty")).and_then(|e| e.text);
        assert_eq!(empty.as_deref(), Some("No messages match “zebra”."));
    }

    #[test]
    fn e_archives_the_selected_message_and_z_brings_it_back() {
        let mut rt = render("inbox");
        let plain = rune::Mods::default();
        key(&mut rt, rune::Key::Text("j".into()), plain);
        rt.frame(1.0 / 60.0);
        key(&mut rt, rune::Key::Text("e".into()), plain);
        rt.frame(1.0 / 60.0);
        assert!(rt.find("sample-0").is_none(), "archived out of the inbox");
        assert!(rt.find("undo").is_some());
        key(&mut rt, rune::Key::Text("z".into()), plain);
        rt.frame(1.0 / 60.0);
        assert!(rt.find("sample-0").is_some(), "back after undo");
    }

    #[test]
    fn the_help_dialog_lists_shortcuts() {
        let mut rt = render("inbox");
        assert!(rt.find("help-dialog").is_none());
        click(&mut rt, "help");
        rt.frame(1.0 / 60.0);
        assert!(rt.find("help-dialog").is_some());
        assert!(rt.find("Selecting").is_some());
        key(&mut rt, rune::Key::Escape, rune::Mods::default());
        rt.frame(1.0 / 60.0);
        assert!(rt.find("help-dialog").is_none());
    }

    fn palette_open(rt: &Runtime<App>) -> bool {
        rt.state.palette.open
    }

    #[test]
    fn the_palette_finds_a_mailbox_and_goes_there() {
        let mut rt = render("inbox");
        key(&mut rt, rune::Key::Command('k'), rune::Mods { cmd: true, ..rune::Mods::default() });
        rt.frame(1.0 / 60.0);
        assert!(palette_open(&rt) && rt.find("palette-input").is_some());
        type_text(&mut rt, "gosent");
        let first = rt.elements().into_iter().find(|e| e.key.as_deref() == Some("palette-0"));
        assert!(first.is_some(), "a match is listed");
        key(&mut rt, rune::Key::Enter, rune::Mods::default());
        rt.frame(1.0 / 60.0);
        assert!(!palette_open(&rt));
        let Auth::SignedIn { session: Some(session), .. } = &rt.state.auth else { panic!() };
        assert_eq!(session.mailbox, "SENT");
    }

    #[test]
    fn palette_keys_dont_reach_the_list() {
        let mut rt = render("inbox");
        key(&mut rt, rune::Key::Command('k'), rune::Mods { cmd: true, ..rune::Mods::default() });
        rt.frame(1.0 / 60.0);
        type_text(&mut rt, "e");
        let Auth::SignedIn { session: Some(session), .. } = &rt.state.auth else { panic!() };
        assert_eq!(session.rows.len(), 3, "typing e in the palette doesn't archive");
        assert_eq!(rt.state.palette.query.text, "e");
        key(&mut rt, rune::Key::Escape, rune::Mods::default());
        rt.frame(1.0 / 60.0);
        assert!(!palette_open(&rt));
    }

    #[test]
    fn clicking_a_palette_row_runs_it() {
        let mut rt = render("inbox");
        key(&mut rt, rune::Key::Command('k'), rune::Mods { cmd: true, ..rune::Mods::default() });
        rt.frame(1.0 / 60.0);
        type_text(&mut rt, "new mes");
        click(&mut rt, "palette-0");
        rt.frame(1.0 / 60.0);
        let Auth::SignedIn { session: Some(session), .. } = &rt.state.auth else { panic!() };
        assert!(session.compose.is_some());
    }

    #[test]
    fn clicking_a_rows_star_stars_only_that_message() {
        let mut rt = render("inbox");
        click(&mut rt, "star");
        rt.frame(1.0 / 60.0);
        let Auth::SignedIn { session: Some(session), .. } = &rt.state.auth else { panic!() };
        assert!(session.reading.is_none(), "the star doesn't open the message");
        assert_eq!(session.visible().iter().filter(|m| m.starred).count(), 1);
    }

    #[test]
    fn checking_rows_shows_the_bulk_actions() {
        let mut rt = render("inbox");
        click(&mut rt, "check");
        rt.frame(1.0 / 60.0);
        let count = rt.elements().into_iter().find(|e| e.key.as_deref() == Some("checked-count")).and_then(|e| e.text);
        assert_eq!(count.as_deref(), Some("1 selected"));
        assert!(rt.find("archive").is_some() && rt.find("delete").is_some());
        let Auth::SignedIn { session: Some(session), .. } = &rt.state.auth else { panic!() };
        assert!(session.reading.is_none(), "the checkbox doesn't open the message");
    }

    #[test]
    fn the_sender_view_names_the_sender_and_lists_only_their_mail() {
        let rt = render("sender");
        let text = |key: &str| rt.elements().into_iter().find(|e| e.key.as_deref() == Some(key)).and_then(|e| e.text);
        assert_eq!(text("mailbox-title").as_deref(), Some("GitHub"));
        assert_eq!(text("counts").as_deref(), Some("1 message"));
        assert_eq!(text("sender-email").as_deref(), Some("All mail from noreply@github.com"));
        assert!(rt.find("sample-1").is_some() && rt.find("sample-0").is_none());
        assert!(rt.find("leave-sender").is_some());
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
