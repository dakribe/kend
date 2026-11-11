//! The sidebar: Gmail's mailboxes, the user's labels with unread counts, and the account.

use rune::{El, Icon};
use rune_kit::shell::{Sidebar, SidebarRow, SidebarSection};
use rune_kit::{Kit, glyphs};

use crate::app::App;
use crate::icons;
use crate::session::{Session, SyncStatus};

pub struct Mailbox {
    pub id: &'static str,
    pub name: &'static str,
    pub icon: &'static Icon,
    /// Show an unread badge, as Gmail does for the inbox.
    pub badge: bool,
}

pub const MAILBOXES: [Mailbox; 7] = [
    Mailbox { id: "INBOX", name: "Inbox", icon: &icons::INBOX, badge: true },
    Mailbox { id: "STARRED", name: "Starred", icon: &glyphs::STAR, badge: false },
    Mailbox { id: "IMPORTANT", name: "Important", icon: &icons::IMPORTANT, badge: false },
    Mailbox { id: "SENT", name: "Sent", icon: &icons::SENT, badge: false },
    Mailbox { id: "DRAFT", name: "Drafts", icon: &icons::DRAFT, badge: false },
    Mailbox { id: "SPAM", name: "Spam", icon: &icons::SPAM, badge: false },
    Mailbox { id: "TRASH", name: "Trash", icon: &icons::TRASH, badge: false },
];

/// What the toolbar calls the selected mailbox.
pub fn mailbox_name(session: &Session) -> &str {
    if let Some(m) = MAILBOXES.iter().find(|m| m.id == session.mailbox) {
        return m.name;
    }
    session.labels.iter().find(|l| l.id == session.mailbox).map_or("", |l| leaf(&l.name))
}

pub fn sidebar(k: &Kit, email: &str, session: &Session) -> El {
    let mut sidebar = Sidebar::new().label("Mailboxes").header_padding(0.0, 12.0).row_gap(2.0);
    for m in &MAILBOXES {
        let mut r = SidebarRow::new(m.name).icon(m.icon).height(30.0);
        if m.badge {
            r = with_badge(r, session.unread_in(m.id));
        }
        sidebar = sidebar.child(finish(r, m.id, session, k));
    }
    if !session.labels.is_empty() {
        let rows = session.labels.iter().map(|l| {
            let r = SidebarRow::new(leaf(&l.name)).icon(&icons::LABEL).height(30.0).depth(depth(&l.name));
            finish(with_badge(r, session.unread_in(&l.id)), &l.id, session, k)
        });
        sidebar = sidebar.section(
            SidebarSection::new("Labels")
                .key("labels")
                .collapsed(session.labels_collapsed)
                .on_toggle(|s: &mut App, _| s.toggle_labels())
                .rows(rows),
        );
    }
    let account = SidebarRow::new(email).subtitle(sync_status(session)).height(45.0).select_only();
    sidebar.footer(account.build(k).key("account")).build(k)
}

fn finish(row: SidebarRow, id: &str, session: &Session, k: &Kit) -> El {
    let target = id.to_string();
    row.selected(session.mailbox == id)
        .on_select(move |s: &mut App, _| s.select_mailbox(&target))
        .build(k)
        .key(format!("mailbox-{id}"))
}

fn with_badge(row: SidebarRow, unread: u64) -> SidebarRow {
    if unread == 0 { row } else { row.badge(unread.to_string()) }
}

/// Gmail nests labels by name: `Work/Clients` sits under `Work`.
fn depth(name: &str) -> u8 {
    name.matches('/').count().min(4) as u8
}

fn leaf(name: &str) -> &str {
    name.rsplit('/').next().unwrap_or(name)
}

pub fn sync_status(session: &Session) -> String {
    if session.error.is_some() {
        return "Sync paused".into();
    }
    match session.sync {
        SyncStatus::Syncing { fetched: 0 } => "Syncing…".into(),
        SyncStatus::Syncing { fetched } => format!("Syncing… {fetched} messages"),
        SyncStatus::Synced => "Up to date".into(),
        SyncStatus::LoadingOlder => "Loading older mail…".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_labels_show_their_last_part_indented() {
        assert_eq!((leaf("Work/Clients/Acme"), depth("Work/Clients/Acme")), ("Acme", 2));
        assert_eq!((leaf("Receipts"), depth("Receipts")), ("Receipts", 0));
    }
}
