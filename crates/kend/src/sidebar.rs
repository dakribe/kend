//! The sidebar: the accounts, Gmail's mailboxes, the user's labels with unread counts, and
//! sync.

use rune::{El, Icon, col, row};
use rune_kit::shell::{Sidebar, SidebarRow, SidebarSection};
use rune_kit::{Button, ButtonVariant, Kit, glyphs};

use crate::app::App;
use crate::app::instant;
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
    if session.mailbox == "INBOX" && session.combined() {
        return "All Inboxes";
    }
    if let Some(m) = MAILBOXES.iter().find(|m| m.id == session.mailbox) {
        return m.name;
    }
    session.labels.iter().find(|l| l.id == session.mailbox).map_or("", |l| leaf(&l.name))
}

pub fn sidebar(k: &Kit, session: &Session, adding: bool) -> El {
    let mut sidebar = Sidebar::new().label("Mailboxes").header_padding(0.0, 12.0).row_gap(2.0);
    sidebar = sidebar.section(accounts(k, session, adding));
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
                .on_toggle(|s: &mut App, tx| {
                    tx.instant();
                    s.toggle_labels()
                })
                .rows(rows),
        );
    }
    let shown = match session.single() {
        Some(account) => account.email.clone(),
        None => format!("{} accounts", session.accounts.len()),
    };
    let account = SidebarRow::new(shown).subtitle(sync_status(session)).height(45.0).select_only();
    // Sync is routine; Sign Out deletes local mail, so it's quieter and asks first (the "…").
    // It signs out of the account shown, so with every account shown it waits for a choice.
    let mut controls = row().gap(4.0).px(4.0).child(
        Button::new("Sync Now")
            .variant(ButtonVariant::GhostMuted)
            .help("Check every account for new mail now")
            .on_press_tx(instant(|s: &mut App| s.sync_now()))
            .build(k)
            .key("sync-now"),
    );
    if let Some(single) = session.single() {
        controls = controls.child(
            Button::new("Sign Out…")
                .variant(ButtonVariant::GhostMuted)
                .help(format!("Sign out of {} and remove its mail from this Mac", single.email))
                .on_press_tx(instant(|s: &mut App| s.ask_sign_out()))
                .build(k)
                .key("sign-out"),
        );
    }
    let footer = col().w_full().gap(2.0).child(account.build(k).key("account")).child(controls);
    sidebar.footer(footer).build(k)
}

/// The signed-in accounts, "All Inboxes" above them when there are several, and Add Account.
fn accounts(k: &Kit, session: &Session, adding: bool) -> SidebarSection {
    let several = session.accounts.len() > 1;
    let mut section = SidebarSection::new("Accounts").key("accounts");
    if several {
        let total: u64 = session.accounts.iter().map(|a| a.inbox_unread).sum();
        let all = SidebarRow::new("All Inboxes").icon(&icons::INBOX).height(30.0).selected(session.scope.is_none());
        section = section.row(
            with_badge(all, total)
                .on_select(|s: &mut App, tx| {
                    tx.instant();
                    s.select_account(None)
                })
                .build(k)
                .key("account-all"),
        );
    }
    for a in &session.accounts {
        let id = a.id;
        // With one account there's nothing to switch between: its row only says which it is.
        let mut r = SidebarRow::new(&a.email).icon(&icons::ACCOUNT).height(30.0);
        if several {
            r = with_badge(r, a.inbox_unread).selected(session.scope == Some(id)).on_select(move |s: &mut App, tx| {
                tx.instant();
                s.select_account(Some(id))
            });
        } else {
            r = r.select_only();
        }
        section = section.row(r.build(k).key(format!("account-{}", a.email)));
    }
    let add = if adding {
        SidebarRow::new("Signing in…").icon(&icons::ADD).height(30.0).select_only().trailing(
            Button::new("Cancel")
                .variant(ButtonVariant::GhostMuted)
                .help("Stop adding the account")
                .on_press_tx(instant(|s: &mut App| s.cancel_add_account()))
                .build(k)
                .key("add-account-cancel"),
        )
    } else {
        SidebarRow::new("Add Account…").icon(&icons::ADD).height(30.0).on_select(|s: &mut App, tx| {
            tx.instant();
            s.add_account()
        })
    };
    section.row(add.build(k).key("add-account"))
}

fn finish(row: SidebarRow, id: &str, session: &Session, k: &Kit) -> El {
    let target = id.to_string();
    row.selected(session.mailbox == id)
        .on_select(move |s: &mut App, tx| {
            tx.instant();
            s.select_mailbox(&target)
        })
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

pub fn leaf(name: &str) -> &str {
    name.rsplit('/').next().unwrap_or(name)
}

pub fn sync_status(session: &Session) -> String {
    if session.shown_error().is_some() {
        return "Sync paused".into();
    }
    match session.sync_status() {
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
