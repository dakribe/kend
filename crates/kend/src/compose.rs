//! Writing a message: a new one, a reply or a forward. The fields are plain text; replies quote
//! the original and stay in its Gmail conversation.

use jiff::Zoned;
use kend_gmail::{Outgoing, forward_subject, parse_recipients, reply_subject};
use kend_store::{Address, Message};
use rune::{El, KeyEvent, ScrollSpec, TextEdit, Tx, col, el, row, text};
use rune_kit::{Button, ButtonVariant, Kit, TextArea, TextInput};

use crate::app::App;
use crate::app::instant;
use crate::dates;

pub enum ComposeKind {
    New,
    Reply { gmail_id: String, thread_id: String },
    Forward,
}

pub enum SendState {
    Editing,
    Sending,
    Failed(String),
}

pub struct Compose {
    pub kind: ComposeKind,
    pub to: TextEdit,
    pub cc: TextEdit,
    pub subject: TextEdit,
    pub body: TextEdit,
    pub state: SendState,
}

#[derive(Clone, Copy)]
pub enum Field {
    To,
    Cc,
    Subject,
    Body,
}

impl Compose {
    pub fn new() -> Self {
        let mut to = TextEdit::new("");
        to.focused = true;
        Self::with(ComposeKind::New, to, TextEdit::new(""), TextEdit::new(""))
    }

    /// To the sender, "Re:" subject, the original quoted below an empty first line.
    pub fn reply(original: &Message, now: &Zoned) -> Self {
        let s = &original.summary;
        let quoted: String =
            original.body_text.as_deref().unwrap_or_default().lines().map(|l| format!("> {l}\n")).collect();
        let body = format!("\n\nOn {}, {} wrote:\n{quoted}", dates::full(s.received_at, now), address(&s.from));
        let mut body = TextEdit::new(body);
        body.set_caret(0);
        body.focused = true;
        Self::with(
            ComposeKind::Reply { gmail_id: s.gmail_id.clone(), thread_id: s.thread_id.clone() },
            TextEdit::new(address(&s.from)),
            TextEdit::new(reply_subject(&s.subject)),
            body,
        )
    }

    /// The original below a forwarding header, the recipient left to fill in.
    pub fn forward(original: &Message, now: &Zoned) -> Self {
        let s = &original.summary;
        let body = format!(
            "\n\n---------- Forwarded message ----------\nFrom: {}\nDate: {}\nSubject: {}\nTo: {}\n\n{}",
            address(&s.from),
            dates::full(s.received_at, now),
            s.subject,
            original.to,
            original.body_text.as_deref().unwrap_or_default()
        );
        let mut to = TextEdit::new("");
        to.focused = true;
        Self::with(ComposeKind::Forward, to, TextEdit::new(forward_subject(&s.subject)), TextEdit::new(body))
    }

    fn with(kind: ComposeKind, to: TextEdit, subject: TextEdit, body: TextEdit) -> Self {
        Self { kind, to, cc: TextEdit::new(""), subject, body, state: SendState::Editing }
    }

    pub fn field(&mut self, field: Field) -> &mut TextEdit {
        match field {
            Field::To => &mut self.to,
            Field::Cc => &mut self.cc,
            Field::Subject => &mut self.subject,
            Field::Body => &mut self.body,
        }
    }

    /// The message to send from `from`, or why it can't be sent yet.
    pub fn outgoing(&self, from: &str) -> Result<Outgoing, String> {
        let to = parse_recipients(&self.to.text)?;
        if to.is_empty() {
            return Err("Add at least one recipient.".into());
        }
        let thread_id = match &self.kind {
            ComposeKind::Reply { thread_id, .. } => Some(thread_id.clone()),
            _ => None,
        };
        Ok(Outgoing {
            from: from.to_string(),
            to,
            cc: parse_recipients(&self.cc.text)?,
            subject: self.subject.text.trim().to_string(),
            body: self.body.text.clone(),
            thread_id,
            ..Outgoing::default()
        })
    }

    /// The message being replied to, for its threading headers.
    pub fn replying_to(&self) -> Option<&str> {
        match &self.kind {
            ComposeKind::Reply { gmail_id, .. } => Some(gmail_id),
            _ => None,
        }
    }

    fn title(&self) -> &'static str {
        match self.kind {
            ComposeKind::New => "New message",
            ComposeKind::Reply { .. } => "Reply",
            ComposeKind::Forward => "Forward",
        }
    }
}

/// "Ana Lima <ana@x.com>", quoting a name with a comma or quote in it.
fn address(a: &Address) -> String {
    if a.name.is_empty() {
        return a.email.clone();
    }
    if a.name.contains([',', '"', '<', '>', ';']) {
        format!("\"{}\" <{}>", a.name.replace('"', ""), a.email)
    } else {
        format!("{} <{}>", a.name, a.email)
    }
}

/// ⌘Return sends from any field.
fn send_shortcut(s: &mut App, event: &KeyEvent, _tx: &mut Tx) -> bool {
    if matches!(event.key, rune::Key::Enter) && event.mods.cmd {
        s.send();
        return true;
    }
    false
}

pub fn compose_view(k: &Kit, compose: &Compose) -> El {
    let sending = matches!(compose.state, SendState::Sending);
    // A label beside each field, so it still says what it is once filled in.
    let input = |edit: &TextEdit, name: &str, field: Field, key: &str| {
        let field = TextInput::new(edit)
            .label(name)
            .width(None)
            .disabled(sending)
            .on_edit(move |s: &mut App| s.compose_field(field))
            .on_key(send_shortcut)
            .build(k)
            .key(key.to_string());
        row()
            .w_full()
            .gap(10.0)
            .items_center()
            .child(text(name).font(k.ty.ui).color(k.c.text_muted).w(56.0))
            .child(col().grow(1.0).min_w(0.0).child(field))
    };
    let send = Button::new(if sending { "Sending…" } else { "Send" })
        .variant(ButtonVariant::Contrast)
        .help("Send (⌘Return)")
        .disabled(sending)
        .on_press_tx(instant(|s: &mut App| s.send()))
        .build(k)
        .key("send");
    let discard = Button::new("Discard")
        .variant(ButtonVariant::Ghost)
        .disabled(sending)
        .on_press_tx(instant(|s: &mut App| s.discard_compose()))
        .build(k)
        .key("discard");
    let header = row()
        .w_full()
        .gap(8.0)
        .items_center()
        .child(text(compose.title()).font(k.ty.body_medium).color(k.c.text_base))
        .child(el().grow(1.0))
        .child(discard)
        .child(send);
    let body = TextArea::new(&compose.body)
        .placeholder("Write your message")
        .label("Message")
        .width(None)
        .auto_grow(14, 400)
        .disabled(sending)
        .on_edit(|s: &mut App| s.compose_field(Field::Body))
        .on_key(send_shortcut)
        .build(k)
        .key("compose-body");

    let mut page = col()
        .w_full()
        .max_w(720.0)
        .px(32.0)
        .pt(24.0)
        .pb(48.0)
        .gap(12.0)
        .child(header)
        .child(input(&compose.to, "To", Field::To, "compose-to"))
        .child(input(&compose.cc, "Cc", Field::Cc, "compose-cc"))
        .child(input(&compose.subject, "Subject", Field::Subject, "compose-subject"));
    if let SendState::Failed(err) = &compose.state {
        page = page.child(text(err).key("compose-error").font(k.ty.ui).color(k.c.danger.fg).wrap());
    }
    col()
        .key("compose")
        .w_full()
        .h_full()
        .min_h(0.0)
        .scroll(ScrollSpec { stick_bottom: false })
        .child(col().w_full().items_center().child(page.child(body)))
}

#[cfg(test)]
mod tests {
    use kend_store::{NewMessage, Store};

    use super::*;

    fn original() -> Message {
        let mut store = Store::open_in_memory().unwrap();
        let account = store.upsert_account("me@gmail.com").unwrap();
        let message = NewMessage {
            gmail_id: "g1".into(),
            thread_id: "t1".into(),
            from: Address::new("Lima, Ana", "ana@x.com"),
            to: "me@gmail.com".into(),
            subject: "Dinner".into(),
            body_text: Some("See you at 7\nBring wine".into()),
            ..NewMessage::default()
        };
        let [id] = store.upsert_messages(account, &[message]).unwrap()[..] else { unreachable!() };
        store.message(id).unwrap().unwrap()
    }

    fn now() -> Zoned {
        "2026-11-10T15:00:00[UTC]".parse().unwrap()
    }

    #[test]
    fn a_reply_quotes_the_original_and_threads() {
        let reply = Compose::reply(&original(), &now());
        assert_eq!(reply.to.text, "\"Lima, Ana\" <ana@x.com>");
        assert_eq!(reply.subject.text, "Re: Dinner");
        assert!(reply.body.text.starts_with("\n\nOn Thu, Jan 1, 1970"), "{}", reply.body.text);
        assert!(reply.body.text.ends_with("> See you at 7\n> Bring wine\n"));
        assert!(reply.body.focused);
        let outgoing = reply.outgoing("me@gmail.com").unwrap();
        assert_eq!(outgoing.thread_id.as_deref(), Some("t1"));
        assert_eq!(reply.replying_to(), Some("g1"));
    }

    #[test]
    fn a_forward_carries_the_original_and_needs_a_recipient() {
        let forward = Compose::forward(&original(), &now());
        assert_eq!(forward.subject.text, "Fwd: Dinner");
        assert!(forward.body.text.contains("Forwarded message"));
        assert!(forward.body.text.contains("See you at 7"));
        assert_eq!(forward.outgoing("me@gmail.com").unwrap_err(), "Add at least one recipient.");
    }

    #[test]
    fn bad_recipients_are_reported() {
        let mut compose = Compose::new();
        compose.to.text = "ana@x.com, nope".into();
        assert!(compose.outgoing("me@gmail.com").unwrap_err().contains("nope"));
        compose.to.text = "ana@x.com".into();
        compose.cc.text = "bo@y.org".into();
        let outgoing = compose.outgoing("me@gmail.com").unwrap();
        assert_eq!((outgoing.to.len(), outgoing.cc.len(), outgoing.thread_id), (1, 1, None));
    }
}
