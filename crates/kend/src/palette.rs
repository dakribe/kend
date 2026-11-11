//! The command palette (⌘K): every action and mailbox, found by typing a few letters of its
//! name. Only commands that apply right now are listed, and the ones used most recently come
//! first.

use rune::{El, KeyEvent, TextEdit, Tx, col, row, text};
use rune_kit::{Keybind, Kit, TextInput};

use crate::app::App;
use crate::session::{Mark, Move};
use crate::sidebar::{MAILBOXES, leaf, mailbox_name};

/// Rows shown at most; typing narrows the rest.
const SHOWN: usize = 12;
/// Commands remembered for "recently used".
const RECENT: usize = 8;

#[derive(Default)]
pub struct Palette {
    pub open: bool,
    pub query: TextEdit,
    /// The highlighted row in the filtered list.
    pub highlight: usize,
    /// Commands run from the palette, newest first.
    recent: Vec<Command>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    NewMessage,
    Reply,
    Forward,
    Archive,
    Delete,
    Undo,
    Star,
    Unstar,
    MarkRead,
    MarkUnread,
    SelectAll,
    ClearSelection,
    Search,
    SyncNow,
    ToggleSidebar,
    OpenInBrowser,
    /// Everything from the open or selected message's sender.
    ShowSender,
    LeaveSender,
    Shortcuts,
    SignOut,
    Send,
    Discard,
    /// Go to a mailbox or label, by its Gmail label id.
    Go(String),
}

pub struct Entry {
    pub command: Command,
    pub label: String,
    pub keys: &'static [&'static str],
}

impl Entry {
    fn new(command: Command, label: impl Into<String>, keys: &'static [&'static str]) -> Self {
        Self { command, label: label.into(), keys }
    }
}

/// The commands that apply in the app's current state.
pub fn entries(app: &App) -> Vec<Entry> {
    use Command::*;
    let mut out = Vec::new();
    let Some(session) = app.session() else {
        out.push(Entry::new(ToggleSidebar, "Toggle Sidebar", &["⇧⌘B"]));
        out.push(Entry::new(Shortcuts, "Keyboard Shortcuts", &["?"]));
        return out;
    };
    let reading = session.reading.as_ref();
    let composing = session.compose.is_some();
    if composing {
        out.push(Entry::new(Send, "Send", &["⌘Return"]));
        out.push(Entry::new(Discard, "Discard Draft", &[]));
    }
    out.push(Entry::new(NewMessage, "New Message", &["⌘N"]));
    if reading.is_some() && !composing {
        out.push(Entry::new(Reply, "Reply", &["R"]));
        out.push(Entry::new(Forward, "Forward", &["F"]));
    }
    if reading.is_some_and(|r| r.message.body_html.is_some()) {
        out.push(Entry::new(OpenInBrowser, "Open in Browser", &["O"]));
    }
    if !composing && !session.targets().is_empty() {
        let n = session.targets().len();
        let (archive, delete) = if n > 1 {
            (format!("Archive {n} Messages"), format!("Move {n} Messages to Trash"))
        } else {
            ("Archive".into(), "Move to Trash".into())
        };
        out.push(Entry::new(Archive, archive, &["E"]));
        out.push(Entry::new(Delete, delete, &["#"]));
        let suffix = if n > 1 { format!(" {n} Messages") } else { String::new() };
        match session.star_mark() {
            Mark::Unstar => out.push(Entry::new(Unstar, format!("Unstar{suffix}"), &["S"])),
            _ => out.push(Entry::new(Star, format!("Star{suffix}"), &["S"])),
        }
        let (read, unread) = if n > 1 {
            (format!("Mark {n} Messages as Read"), format!("Mark {n} Messages as Unread"))
        } else {
            ("Mark as Read".into(), "Mark as Unread".into())
        };
        out.push(Entry::new(MarkRead, read, &["⇧I"]));
        out.push(Entry::new(MarkUnread, unread, &["⇧U"]));
    }
    if !composing && let Some(from) = session.focused_sender() {
        out.push(Entry::new(ShowSender, format!("Show All from {}", from.display()), &["⇧F"]));
    }
    if reading.is_none() && !composing && session.sender.is_some() {
        out.push(Entry::new(LeaveSender, format!("Back to {}", mailbox_name(session)), &["Esc"]));
    }
    if session.last_move.is_some() || session.discarded.is_some() {
        out.push(Entry::new(Undo, "Undo", &["Z"]));
    }
    if reading.is_none() && !composing && !session.visible().is_empty() {
        out.push(Entry::new(SelectAll, "Select All", &["⌘A"]));
    }
    if !session.checked.is_empty() {
        out.push(Entry::new(ClearSelection, "Clear Selection", &["Esc"]));
    }
    if !composing {
        out.push(Entry::new(Search, "Search Mail", &["⌘F"]));
    }
    out.push(Entry::new(SyncNow, "Sync Now", &[]));
    out.push(Entry::new(ToggleSidebar, "Toggle Sidebar", &["⇧⌘B"]));
    out.push(Entry::new(Shortcuts, "Keyboard Shortcuts", &["?"]));
    for m in &MAILBOXES {
        out.push(Entry::new(Go(m.id.into()), format!("Go to {}", m.name), &[]));
    }
    for l in &session.labels {
        out.push(Entry::new(Go(l.id.clone()), format!("Go to {}", leaf(&l.name)), &[]));
    }
    out.push(Entry::new(SignOut, "Sign Out…", &[]));
    out
}

/// The entries matching `query`, best first. With no query, recently used commands lead.
pub fn filtered(entries: Vec<Entry>, query: &str, recent: &[Command]) -> Vec<Entry> {
    let query = query.trim().to_lowercase();
    let recency = |c: &Command| recent.iter().position(|r| r == c).map_or(0, |i| (RECENT - i) as i32);
    let mut scored: Vec<(i32, usize, Entry)> = entries
        .into_iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let score = if query.is_empty() { 0 } else { score(&query, &e.label)? };
            // Recency breaks near-ties without outranking a clearly better match.
            Some((score + recency(&e.command) * 4, i, e))
        })
        .collect();
    scored.sort_by_key(|(score, i, _)| (std::cmp::Reverse(*score), *i));
    scored.into_iter().map(|(_, _, e)| e).collect()
}

/// Case-insensitive subsequence match: `query`'s letters in order within `label`. Runs of
/// letters and letters starting a word score higher, so "ar" finds Archive before Clear Selection.
fn score(query: &str, label: &str) -> Option<i32> {
    let mut needle = query.chars().peekable();
    let mut score = 0;
    let mut run = false;
    let mut word_start = true;
    for c in label.to_lowercase().chars() {
        if needle.peek() == Some(&c) {
            needle.next();
            score += 10 + if run { 12 } else { 0 } + if word_start { 8 } else { 0 };
            run = true;
        } else {
            run = false;
            score -= 1;
        }
        word_start = !c.is_alphanumeric();
    }
    needle.peek().is_none().then_some(score)
}

impl App {
    pub fn open_palette(&mut self, tx: &mut Tx) {
        self.palette.open = true;
        self.palette.query = TextEdit::new("");
        self.palette.highlight = 0;
        tx.focus("palette-input", true);
    }

    pub fn close_palette(&mut self) {
        self.palette.open = false;
    }

    /// What the palette lists right now.
    pub fn palette_entries(&self) -> Vec<Entry> {
        filtered(entries(self), &self.palette.query.text, &self.palette.recent)
    }

    /// Keys in the palette's field: arrows move the highlight, Return runs it, Escape closes.
    fn palette_key(&mut self, event: &KeyEvent, tx: &mut Tx) -> bool {
        use rune::Key;
        tx.instant();
        let count = self.palette_entries().len().min(SHOWN);
        let ctrl = event.mods.ctrl;
        match &event.key {
            Key::Escape => self.close_palette(),
            Key::Down => self.palette.highlight = (self.palette.highlight + 1) % count.max(1),
            Key::Text(t) if ctrl && t == "n" => self.palette.highlight = (self.palette.highlight + 1) % count.max(1),
            Key::Up => self.palette.highlight = (self.palette.highlight + count.max(1) - 1) % count.max(1),
            Key::Text(t) if ctrl && t == "p" => {
                self.palette.highlight = (self.palette.highlight + count.max(1) - 1) % count.max(1)
            }
            Key::Enter => {
                let chosen = self.palette_entries().into_iter().nth(self.palette.highlight);
                if let Some(entry) = chosen {
                    self.run(entry.command, tx);
                }
            }
            _ => return false,
        }
        true
    }

    /// Closes the palette and runs `command`.
    pub fn run(&mut self, command: Command, tx: &mut Tx) {
        tx.instant();
        self.close_palette();
        let recent = &mut self.palette.recent;
        recent.retain(|c| c != &command);
        recent.insert(0, command.clone());
        recent.truncate(RECENT);
        match command {
            Command::NewMessage => self.compose_new(),
            Command::Reply => self.reply(false),
            Command::Forward => self.reply(true),
            Command::Archive => self.move_message(Move::Archive),
            Command::Delete => self.move_message(Move::Trash),
            Command::Undo => self.undo(),
            Command::Star => self.mark(Mark::Star),
            Command::Unstar => self.mark(Mark::Unstar),
            Command::MarkRead => self.mark(Mark::Read),
            Command::MarkUnread => self.mark(Mark::Unread),
            Command::SelectAll => self.check_all(),
            Command::ClearSelection => self.clear_checked(),
            Command::Search => tx.focus("search", true),
            Command::SyncNow => self.sync_now(),
            Command::ToggleSidebar => self.sidebar.toggle(),
            Command::OpenInBrowser => self.open_in_browser(),
            Command::ShowSender => self.show_sender(),
            Command::LeaveSender => self.leave_sender(),
            Command::Shortcuts => self.help = true,
            Command::SignOut => self.ask_sign_out(),
            Command::Send => self.send(),
            Command::Discard => self.discard_compose(),
            Command::Go(mailbox) => {
                self.close_message();
                self.select_mailbox(&mailbox);
            }
        }
    }
}

/// The palette, near the top of the window: a field, then the matching commands.
pub fn palette_view(k: &Kit, app: &App, layer: &mut Vec<El>) {
    if !app.palette.open {
        return;
    }
    let input = TextInput::new(&app.palette.query)
        .placeholder("Type a command or mailbox…")
        .label("Command")
        .width(None)
        .leading_icon(k.glyph.search)
        .on_edit(|s: &mut App| &mut s.palette.query)
        .on_change(|s: &mut App, tx: &mut Tx| {
            tx.instant();
            s.palette.highlight = 0;
        })
        .on_key(|s: &mut App, event: &KeyEvent, tx: &mut Tx| s.palette_key(event, tx))
        .build(k)
        .key("palette-input");
    let entries = app.palette_entries();
    let mut list = col().key("palette-list").w_full().gap(2.0);
    if entries.is_empty() {
        list = list.child(
            text("No matching commands").key("palette-empty").font(k.ty.ui).color(k.c.text_faint).px(10.0).py(8.0),
        );
    }
    for (i, entry) in entries.into_iter().take(SHOWN).enumerate() {
        let mut r = row()
            .key(format!("palette-{i}"))
            .w_full()
            .h(32.0)
            .px(10.0)
            .gap(12.0)
            .items_center()
            .radius(k.radius.md)
            .hover(k.c.hover)
            .child(text(entry.label.clone()).font(k.ty.ui).color(k.c.text_base).grow(1.0).min_w(0.0).truncate());
        if !entry.keys.is_empty() {
            r = r.child(Keybind::new(entry.keys).build(k));
        }
        if i == app.palette.highlight {
            r = r.fill(k.c.layer_03);
        }
        let command = entry.command;
        r = r.on_press_tx(move |s: &mut App, tx: &mut Tx| s.run(command.clone(), tx));
        list = list.child(r);
    }
    let panel = col()
        .key("palette")
        .role("dialog")
        .label("Command palette")
        .w_full()
        .max_w(520.0)
        .padding(8.0)
        .gap(8.0)
        .radius(8.0)
        .fill(k.c.layer_01)
        .border(1.0, k.c.border_base)
        .child(input)
        .child(list)
        .modal_tx(true, |s: &mut App, tx: &mut Tx| {
            tx.instant();
            s.close_palette();
        });
    layer.push(rune::el().key("palette-scrim").absolute(0.0, 0.0).w_full().h_full().fill(k.c.scrim).a11y_hidden());
    layer.push(
        col().key("palette-layer").absolute(0.0, 0.0).w_full().h_full().px(16.0).pt(96.0).items_center().child(panel),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(entries: &[Entry]) -> Vec<&str> {
        entries.iter().map(|e| e.label.as_str()).collect()
    }

    fn sample() -> Vec<Entry> {
        vec![
            Entry::new(Command::ClearSelection, "Clear Selection", &[]),
            Entry::new(Command::Archive, "Archive", &[]),
            Entry::new(Command::Go("INBOX".into()), "Go to Inbox", &[]),
            Entry::new(Command::Go("SENT".into()), "Go to Sent", &[]),
        ]
    }

    #[test]
    fn letters_in_order_match_and_word_starts_rank_first() {
        assert_eq!(labels(&filtered(sample(), "ar", &[]))[0], "Archive");
        assert_eq!(labels(&filtered(sample(), "gi", &[])), ["Go to Inbox"]);
        assert_eq!(labels(&filtered(sample(), "SENT", &[])), ["Go to Sent"]);
        assert!(filtered(sample(), "zz", &[]).is_empty());
    }

    #[test]
    fn recently_used_commands_come_first() {
        let recent = [Command::Go("SENT".into())];
        assert_eq!(labels(&filtered(sample(), "", &recent))[0], "Go to Sent");
        assert_eq!(labels(&filtered(sample(), "", &[]))[0], "Clear Selection", "otherwise in list order");
    }
}
