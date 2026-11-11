#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AccountId(pub(crate) i64);

/// A message's local id: stable for the life of the database, and the FTS row id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MessageId(pub(crate) i64);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Address {
    pub name: String,
    pub email: String,
}

impl Address {
    pub fn new(name: impl Into<String>, email: impl Into<String>) -> Self {
        Self { name: name.into(), email: email.into() }
    }

    /// The name when there is one, else the address.
    pub fn display(&self) -> &str {
        if self.name.is_empty() { &self.email } else { &self.name }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelKind {
    /// Gmail's own: INBOX, SENT, UNREAD, STARRED, …
    System,
    /// Labels the user made.
    User,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    /// Gmail's label id (`INBOX`, `Label_12`).
    pub id: String,
    pub name: String,
    pub kind: LabelKind,
}

/// A message as sync hands it to the store. Bodies may come later (`None` keeps what's stored).
#[derive(Clone, Debug, Default)]
pub struct NewMessage {
    pub gmail_id: String,
    pub thread_id: String,
    pub from: Address,
    /// Recipients as shown ("Ana <ana@x.com>, bo@y.com").
    pub to: String,
    pub subject: String,
    pub snippet: String,
    /// Unix milliseconds (Gmail's `internalDate`).
    pub received_at: i64,
    /// Gmail label ids, including `UNREAD` and `STARRED`.
    pub label_ids: Vec<String>,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
}

/// One row of a mailbox list: everything the list shows, nothing it doesn't.
#[derive(Clone, Debug, PartialEq)]
pub struct MessageSummary {
    pub id: MessageId,
    pub gmail_id: String,
    pub thread_id: String,
    pub from: Address,
    pub subject: String,
    pub snippet: String,
    pub received_at: i64,
    pub unread: bool,
    pub starred: bool,
}

impl MessageSummary {
    /// Where the page after this row starts.
    pub fn cursor(&self) -> Cursor {
        Cursor { received_at: self.received_at, id: self.id }
    }
}

/// A position in a mailbox, newest first. Ties on time break on id, so no row is skipped
/// or repeated between pages.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cursor {
    pub(crate) received_at: i64,
    pub(crate) id: MessageId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub summary: MessageSummary,
    pub to: String,
    pub label_ids: Vec<String>,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
}
