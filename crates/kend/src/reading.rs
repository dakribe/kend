//! An open message: who sent it, when, to whom, and what it says.

use jiff::Zoned;
use rune::{El, ScrollSpec, TextStyle, col, el, memo, row, text};
use rune_kit::{Button, ButtonVariant, Kit, Markdown};

use crate::app::App;
use crate::app::instant;
use crate::dates;
use crate::session::{BodyState, Move, Reading};
use crate::view::ghost;

const SUBJECT: TextStyle = TextStyle::new(20.0, 600.0).lh(27.0);
/// A little larger than the UI text and generously spaced: this is for reading.
const BODY: TextStyle = TextStyle::new(14.0, 400.0).lh(22.0);
/// Lines longer than this are hard to read; wide windows centre the column.
const MEASURE: f32 = 720.0;

/// What you can do with the open message.
fn actions(k: &Kit, has_html: bool) -> El {
    let mut bar = row()
        .key("reading-actions")
        .w_full()
        .gap(4.0)
        .items_center()
        .child(ghost(k, "Back", "Back to the list (Esc)", "back", |s| s.close_message()))
        .child(el().grow(1.0))
        .child(ghost(k, "Reply", "Reply (R)", "reply", |s| s.reply(false)))
        .child(ghost(k, "Forward", "Forward (F)", "forward", |s| s.reply(true)))
        .child(ghost(k, "Archive", "Archive (E)", "archive", |s| s.move_message(Move::Archive)))
        .child(ghost(k, "Delete", "Move to Trash (#)", "delete", |s| s.move_message(Move::Trash)));
    if has_html {
        bar = bar.child(ghost(k, "Open in Browser", "See this email exactly as designed", "open-in-browser", |s| {
            s.open_in_browser()
        }));
    }
    bar
}

pub fn reading_view(k: &Kit, dark: bool, reading: &Reading) -> El {
    let m = &reading.message;
    let s = &m.summary;
    let now = Zoned::now();
    let subject = if s.subject.is_empty() { "(no subject)" } else { &s.subject };

    let initial = s.from.display().chars().next().unwrap_or('?').to_uppercase().to_string();
    let avatar = row()
        .size(36.0, 36.0)
        .radius(18.0)
        .center()
        .fill(k.c.layer_03)
        .child(text(initial).font(k.ty.ui_medium).color(k.c.text_base));
    let mut who = col()
        .gap(2.0)
        .grow(1.0)
        .min_w(0.0)
        .child(text(s.from.display()).key("reading-from").font(k.ty.ui_medium).color(k.c.text_base).truncate());
    if !s.from.name.is_empty() {
        who = who.child(text(&s.from.email).font(k.ty.small).color(k.c.text_muted).selectable(true));
    }
    let date = text(dates::full(s.received_at, &now)).font(k.ty.small).color(k.c.text_muted);
    let sender = row().w_full().gap(12.0).items_center().child(avatar).child(who).child(date);

    let mut header = col()
        .w_full()
        .gap(14.0)
        .child(text(subject).key("reading-subject").font(SUBJECT).color(k.c.text_base).wrap().selectable(true))
        .child(sender);
    if !m.to.is_empty() {
        header = header.child(text(format!("To: {}", m.to)).font(k.ty.small).color(k.c.text_muted).wrap());
    }

    let body = match &reading.body {
        BodyState::Ready => match (&reading.markdown, m.body_text.as_deref().filter(|t| !t.trim().is_empty())) {
            (Some(markdown), _) => {
                let kit = k.clone();
                // Parsed and laid out only when the message or the theme changes. Keyed by the
                // memo id.
                memo(format!("md-{}", s.gmail_id), (&s.gmail_id, markdown.len(), dark), move || {
                    Markdown::new(markdown.as_str())
                        .selectable(true)
                        .on_link(|s: &mut App, url: &str| s.open_link(url))
                        .build(&kit)
                })
            }
            (None, Some(body)) => {
                text(body).key("reading-body").font(BODY).color(k.c.text_base).wrap().selectable(true)
            }
            (None, None) => text("This message has no text.").font(k.ty.ui).color(k.c.text_muted),
        },
        BodyState::Loading => row()
            .key("reading-loading")
            .gap(10.0)
            .items_center()
            .child(crate::view::spinner(k))
            .child(text("Loading message…").font(k.ty.ui).color(k.c.text_muted)),
        BodyState::Failed(err) => col()
            .key("reading-error")
            .gap(10.0)
            .child(text(format!("Couldn't load this message: {err}")).font(k.ty.ui).color(k.c.danger.fg).wrap())
            .child(
                Button::new("Try again")
                    .variant(ButtonVariant::Ghost)
                    .on_press_tx(instant(|s: &mut App| s.retry_body()))
                    .build(k)
                    .key("reading-retry"),
            ),
    };

    let page = col()
        .w_full()
        .max_w(MEASURE)
        .px(32.0)
        .pt(12.0)
        .pb(48.0)
        .gap(20.0)
        .child(actions(k, m.body_html.is_some()))
        .child(header)
        .child(el().w_full().h(1.0).fill(k.c.border_muted))
        .child(body);
    // Keyed by message, so moving to the next message starts at its top.
    col()
        .key(format!("reading-{}", s.gmail_id))
        .w_full()
        .h_full()
        .min_h(0.0)
        .scroll(ScrollSpec { stick_bottom: false })
        .child(col().w_full().items_center().child(page))
}
