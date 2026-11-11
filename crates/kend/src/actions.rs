//! What you can do with mail, in one bar under the toolbar. The same buttons sit in the same
//! places whether the list or a message is showing, so a hand that learned where Archive is
//! finds it there every time. Each button shows its key, so the bar teaches the keyboard.

use rune::{El, Icon, col, el, row, text};
use rune_kit::{Button, ButtonVariant, IconButton, IconButtonVariant, Keybind, KeybindVariant, Kit, glyphs};

use crate::app::{App, instant};
use crate::icons;
use crate::session::{Mark, Move, Session};

/// The bar's height: a 24-point button with room around it.
const HEIGHT: f32 = 40.0;
/// Star and Unstar (and Read and Unread) swap labels; a fixed width keeps the buttons after
/// them from moving when they do.
const STAR_WIDTH: f32 = 96.0;
const READ_WIDTH: f32 = 104.0;

/// A borderless button with an icon, a label and its key, e.g. "Archive E". Disabled when it
/// has nothing to act on, but never removed, so nothing beside it moves.
pub struct Action {
    pub icon: &'static Icon,
    pub label: &'static str,
    pub help: &'static str,
    pub keys: &'static [&'static str],
    pub key: &'static str,
    pub enabled: bool,
    pub press: fn(&mut App),
}

impl Action {
    pub fn build(self, k: &Kit) -> El {
        let label = self.label;
        let cap = Keybind::new(self.keys).variant(KeybindVariant::Ghost).build(k);
        Button::new(label)
            .variant(ButtonVariant::Ghost)
            .icon(self.icon)
            .help(self.help)
            .shortcut(self.keys.join(" or "))
            .label_with(move |font, ink| {
                row().gap(6.0).items_center().child(text(label).font(font).color(ink)).child(cap)
            })
            .disabled(!self.enabled)
            .on_press_tx(instant(self.press))
            .build(k)
            .key(self.key)
    }
}

/// A thin rule between groups of actions.
fn divider(k: &Kit) -> El {
    el().w(1.0).h(16.0).mx(6.0).fill(k.c.border_muted)
}

pub fn action_bar(k: &Kit, session: &Session) -> El {
    let reading = session.reading.as_ref();
    let has_targets = !session.targets().is_empty();
    // Triage first: it's what the bar is used for most, and it never moves.
    let archive = Action {
        icon: &icons::ARCHIVE,
        label: "Archive",
        help: "Archive",
        keys: &["E"],
        key: "archive",
        enabled: has_targets,
        press: |s| s.move_message(Move::Archive),
    };
    let delete = Action {
        icon: &icons::TRASH,
        label: "Delete",
        help: "Move to Trash",
        keys: &["#"],
        key: "delete",
        enabled: has_targets,
        press: |s| s.move_message(Move::Trash),
    };
    let star = match session.star_mark() {
        Mark::Unstar => Action {
            icon: &glyphs::STAR_FILLED,
            label: "Unstar",
            help: "Unstar",
            keys: &["S"],
            key: "toggle-star",
            enabled: has_targets,
            press: |s| s.mark(Mark::Unstar),
        },
        _ => Action {
            icon: &glyphs::STAR,
            label: "Star",
            help: "Star",
            keys: &["S"],
            key: "toggle-star",
            enabled: has_targets,
            press: |s| s.mark(Mark::Star),
        },
    };
    let read = match session.read_mark() {
        Mark::Read => Action {
            icon: &icons::READ,
            label: "Read",
            help: "Mark as read",
            keys: &["⇧I"],
            key: "toggle-read",
            enabled: has_targets,
            press: |s| s.mark(Mark::Read),
        },
        _ => Action {
            icon: &icons::UNREAD,
            label: "Unread",
            help: "Mark as unread",
            keys: &["⇧U"],
            key: "toggle-read",
            enabled: has_targets,
            press: |s| s.mark(Mark::Unread),
        },
    };
    let mut bar = row()
        .key("action-bar")
        .role("toolbar")
        .label("Message actions")
        .w_full()
        .h(HEIGHT)
        .px(12.0)
        .gap(2.0)
        .items_center()
        .child(archive.build(k))
        .child(delete.build(k))
        .child(divider(k))
        .child(star.build(k).w(STAR_WIDTH))
        .child(read.build(k).w(READ_WIDTH));
    // Replying needs the whole message, so these come with an open one, after the buttons
    // that are always there.
    if let Some(reading) = reading {
        let reply = Action {
            icon: &icons::REPLY,
            label: "Reply",
            help: "Reply",
            keys: &["R"],
            key: "reply",
            enabled: true,
            press: |s| s.reply(false),
        };
        let forward = Action {
            icon: &icons::FORWARD,
            label: "Forward",
            help: "Forward",
            keys: &["F"],
            key: "forward",
            enabled: true,
            press: |s| s.reply(true),
        };
        bar = bar.child(divider(k)).child(reply.build(k)).child(forward.build(k));
        if reading.message.body_html.is_some() {
            bar = bar.child(
                IconButton::new(&icons::EXTERNAL)
                    .variant(IconButtonVariant::Ghost)
                    .label("Open in Browser")
                    .help("Open in Browser, exactly as designed")
                    .shortcut("O")
                    .on_press_tx(instant(|s: &mut App| s.open_in_browser()))
                    .build(k)
                    .key("open-in-browser"),
            );
        }
    }
    bar.child(el().grow(1.0).min_w(8.0)).child(status(k, session))
}

/// The bar's far end: what just happened, with Undo, or how many messages are selected, with
/// Clear. Next to the buttons that caused it, where the eye already is.
fn status(k: &Kit, session: &Session) -> El {
    let mut end = row().key("action-status").gap(4.0).items_center().min_w(0.0).shrink();
    let checked = session.checked.len();
    if let Some(notice) = &session.notice {
        end = end.child(text(notice).key("notice").font(k.ty.ui).color(k.c.text_muted).truncate().shrink());
        if session.last_move.is_some() || session.discarded.is_some() {
            end = end.child(
                Action {
                    icon: &icons::UNDO,
                    label: "Undo",
                    help: "Undo",
                    keys: &["Z"],
                    key: "undo",
                    enabled: true,
                    press: |s| s.undo(),
                }
                .build(k),
            );
        }
    } else if checked > 0 && session.reading.is_none() {
        end = end
            .child(text(format!("{checked} selected")).key("checked-count").font(k.ty.ui).color(k.c.text_muted))
            .child(
                Button::new("Clear")
                    .variant(ButtonVariant::Ghost)
                    .help("Clear selection")
                    .shortcut("Esc")
                    .label_with({
                        let cap = Keybind::new(&["Esc"]).variant(KeybindVariant::Ghost).build(k);
                        move |font, ink| {
                            row().gap(6.0).items_center().child(text("Clear").font(font).color(ink)).child(cap)
                        }
                    })
                    .on_press_tx(instant(|s: &mut App| s.clear_checked()))
                    .build(k)
                    .key("clear-checked"),
            );
    }
    col().child(end)
}
