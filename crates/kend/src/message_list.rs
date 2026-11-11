//! The message list. Only rows near the viewport are built (`virtual_rows`), each row is
//! memoised on what it shows, and pages load from the database as the list nears its end, so
//! scrolling costs the same in a mailbox of 100 or 100,000 messages.

use std::borrow::Cow;

use jiff::Zoned;
use kend_store::MessageSummary;
use rune::{Align, El, GrowthSpec, ScrollSpec, col, el, memo, row, text};
use rune_kit::Kit;

use crate::app::App;
use crate::app::instant;
use crate::dates;
use crate::session::{Session, SyncStatus};
use crate::view::message;

/// Every row is this tall, so the estimate is exact and nothing shifts as rows mount.
const ROW_HEIGHT: f32 = 40.0;

pub fn message_list(k: &Kit, dark: bool, session: &Session) -> El {
    if session.visible().is_empty() {
        let note = match (&session.results, &session.sync) {
            (Some(_), _) => format!("No messages match “{}”.", session.query()),
            (None, SyncStatus::Syncing { .. }) => "Loading your mail…".into(),
            (None, _) => "Nothing here.".into(),
        };
        return col().key("messages").w_full().h_full().center().child(message(k, &note).key("empty"));
    }
    let kit = k.clone();
    let now = Zoned::now();
    let rows = col()
        .key("message-rows")
        .w_full()
        .growth(GrowthSpec::default())
        .virtual_rows(
            session.visible().len(),
            ROW_HEIGHT,
            |s: &App, i| {
                s.session().and_then(|x| x.visible().get(i)).map_or(Cow::Borrowed(""), |m| Cow::Borrowed(&m.gmail_id))
            },
            move |s: &App, i| {
                let Some((session, m)) = s.session().and_then(|x| Some((x, x.visible().get(i)?))) else { return el() };
                let selected = session.selected.as_deref() == Some(m.gmail_id.as_str());
                let date = dates::label(m.received_at, &now);
                // Rebuilt only when what the row shows changes.
                memo(m.gmail_id.as_str(), (m, selected, &date, dark), || message_row(&kit, m, selected, &date))
            },
        )
        .virtual_rows_version(session.rows_version)
        .on_visible_rows(|s: &mut App, visible| s.load_more(visible.end));
    col().key("messages").w_full().h_full().min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).child(rows)
}

fn message_row(k: &Kit, m: &MessageSummary, selected: bool, date: &str) -> El {
    let (weight, ink) = if m.unread { (k.ty.ui_medium, k.c.text_base) } else { (k.ty.ui, k.c.text_muted) };
    let mut dot = el().size(8.0, 8.0).radius(4.0);
    if m.unread {
        dot = dot.fill(k.c.text_accent);
    }
    let from = text(m.from.display()).font(weight).color(ink).truncate().w(180.0);
    let subject = if m.subject.is_empty() { "(no subject)" } else { &m.subject };
    let line = row()
        .grow(1.0)
        .min_w(0.0)
        .gap(8.0)
        .child(text(subject).font(weight).color(ink).truncate().shrink())
        .child(text(&m.snippet).font(k.ty.ui).color(k.c.text_faint).truncate().grow(1.0).min_w(0.0));
    let date = text(date)
        .font(k.ty.small)
        .color(if m.unread { k.c.text_base } else { k.c.text_faint })
        .align(Align::End)
        .w(64.0);
    let id = m.gmail_id.clone();
    let mut r = row()
        .w_full()
        .h(ROW_HEIGHT)
        .px(16.0)
        .gap(12.0)
        .items_center()
        .radius(k.radius.md)
        .hover(k.c.hover)
        .on_press_tx(instant(move |s: &mut App| s.open_message(&id)))
        .child(dot)
        .child(from)
        .child(line)
        .child(date);
    if selected {
        r = r.fill(k.c.layer_02);
    }
    r
}
