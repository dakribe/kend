//! Where sync reads mail from: Gmail in the app, a fake in tests.

use kend_gmail::{Gmail, GmailMessage, MessageBody};
use kend_store::{Label, LabelKind, NewMessage};

use crate::parse::{decode_entities, parse_address};

/// The Gmail calls sync makes, in store types. `Sync` because pages are fetched in parallel.
pub trait MailSource: Sync {
    /// The mailbox's current history id.
    fn history_id(&self) -> kend_gmail::Result<String>;
    fn labels(&self) -> kend_gmail::Result<Vec<Label>>;
    /// Up to `max` message ids in `label`, newest first, and the next page's token.
    /// Narrowed by a Gmail `search` (`before:1700000000`) when given.
    fn list(
        &self,
        label: &str,
        search: Option<&str>,
        page_token: Option<&str>,
        max: usize,
    ) -> kend_gmail::Result<(Vec<String>, Option<String>)>;
    /// `None` when the message no longer exists.
    fn message(&self, id: &str) -> kend_gmail::Result<Option<NewMessage>>;
    /// The message's body; `None` when the message no longer exists.
    fn body(&self, id: &str) -> kend_gmail::Result<Option<MessageBody>>;
    /// One page of changes since `start`; `Err(NotFound)` when `start` has expired.
    fn history(&self, start: &str, page_token: Option<&str>) -> kend_gmail::Result<ChangePage>;
}

#[derive(Clone, Debug, Default)]
pub struct ChangePage {
    /// Messages added or relabelled.
    pub changed: Vec<String>,
    pub deleted: Vec<String>,
    /// Where these changes leave the mailbox.
    pub history_id: String,
    pub next_page_token: Option<String>,
}

impl MailSource for Gmail {
    fn history_id(&self) -> kend_gmail::Result<String> {
        Gmail::history_id(self)
    }

    fn labels(&self) -> kend_gmail::Result<Vec<Label>> {
        Ok(Gmail::labels(self)?
            .into_iter()
            .map(|l| Label {
                kind: if l.kind == "user" { LabelKind::User } else { LabelKind::System },
                id: l.id,
                name: l.name,
            })
            .collect())
    }

    fn list(
        &self,
        label: &str,
        search: Option<&str>,
        page_token: Option<&str>,
        max: usize,
    ) -> kend_gmail::Result<(Vec<String>, Option<String>)> {
        let page = self.list_messages(label, search, page_token, max.min(500) as u32)?;
        Ok((page.messages.into_iter().map(|m| m.id).collect(), page.next_page_token))
    }

    fn message(&self, id: &str) -> kend_gmail::Result<Option<NewMessage>> {
        Ok(Gmail::message(self, id)?.map(to_new_message))
    }

    fn body(&self, id: &str) -> kend_gmail::Result<Option<MessageBody>> {
        Gmail::body(self, id)
    }

    fn history(&self, start: &str, page_token: Option<&str>) -> kend_gmail::Result<ChangePage> {
        let page = Gmail::history(self, start, page_token)?;
        Ok(ChangePage {
            changed: page.history.iter().flat_map(|r| r.changed()).map(str::to_string).collect(),
            deleted: page.history.iter().flat_map(|r| r.deleted()).map(str::to_string).collect(),
            history_id: page.history_id,
            next_page_token: page.next_page_token,
        })
    }
}

pub(crate) fn to_new_message(m: GmailMessage) -> NewMessage {
    NewMessage {
        from: parse_address(m.header("From").unwrap_or_default()),
        to: m.header("To").unwrap_or_default().to_string(),
        subject: m.header("Subject").unwrap_or_default().to_string(),
        snippet: decode_entities(&m.snippet),
        received_at: m.internal_date.parse().unwrap_or_default(),
        gmail_id: m.id,
        thread_id: m.thread_id,
        label_ids: m.label_ids,
        body_text: None,
        body_html: None,
    }
}
