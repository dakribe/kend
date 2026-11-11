use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use kend_gmail::Error as GmailError;
use kend_store::{AccountId, Address, Label, LabelKind, NewMessage, Store};

use super::*;

/// An in-memory Gmail. The inbox is newest first; `history` serves the queued change pages.
#[derive(Default)]
struct Fake {
    messages: Mutex<HashMap<String, NewMessage>>,
    inbox: Mutex<Vec<String>>,
    history_id: Mutex<String>,
    changes: Mutex<Vec<ChangePage>>,
    history_expired: bool,
    failing: HashSet<String>,
    fetches: AtomicUsize,
}

fn message(id: &str, received_at: i64, labels: &[&str]) -> NewMessage {
    NewMessage {
        gmail_id: id.into(),
        thread_id: id.into(),
        from: Address::new("Ana", "ana@x.com"),
        subject: format!("subject {id}"),
        received_at,
        label_ids: labels.iter().map(|l| l.to_string()).collect(),
        ..NewMessage::default()
    }
}

impl Fake {
    fn with_inbox(count: usize) -> Self {
        let fake = Fake { history_id: Mutex::new("100".into()), ..Fake::default() };
        for i in (0..count).rev() {
            fake.add(message(&format!("m{i:03}"), i as i64, &["INBOX"]));
        }
        fake
    }

    fn add(&self, m: NewMessage) {
        let mut inbox = self.inbox.lock().unwrap();
        if m.label_ids.iter().any(|l| l == "INBOX") {
            inbox.push(m.gmail_id.clone());
        }
        self.messages.lock().unwrap().insert(m.gmail_id.clone(), m);
    }
}

impl MailSource for Fake {
    fn history_id(&self) -> kend_gmail::Result<String> {
        Ok(self.history_id.lock().unwrap().clone())
    }

    fn labels(&self) -> kend_gmail::Result<Vec<Label>> {
        Ok(vec![Label { id: "INBOX".into(), name: "INBOX".into(), kind: LabelKind::System }])
    }

    fn list(
        &self,
        _label: &str,
        page_token: Option<&str>,
        max: usize,
    ) -> kend_gmail::Result<(Vec<String>, Option<String>)> {
        let inbox = self.inbox.lock().unwrap();
        let start: usize = page_token.map_or(0, |t| t.parse().unwrap());
        let end = (start + max).min(inbox.len());
        let next = (end < inbox.len()).then(|| end.to_string());
        Ok((inbox[start..end].to_vec(), next))
    }

    fn message(&self, id: &str) -> kend_gmail::Result<Option<NewMessage>> {
        self.fetches.fetch_add(1, Ordering::Relaxed);
        if self.failing.contains(id) {
            return Err(GmailError::Server(503));
        }
        Ok(self.messages.lock().unwrap().get(id).cloned())
    }

    fn history(&self, _start: &str, _page_token: Option<&str>) -> kend_gmail::Result<ChangePage> {
        if self.history_expired {
            return Err(GmailError::NotFound);
        }
        let mut changes = self.changes.lock().unwrap();
        if changes.is_empty() {
            return Ok(ChangePage { history_id: self.history_id()?, ..ChangePage::default() });
        }
        let mut page = changes.remove(0);
        if !changes.is_empty() {
            page.next_page_token = Some("more".into());
        }
        Ok(page)
    }
}

fn setup() -> (Store, AccountId) {
    let store = Store::open_in_memory().unwrap();
    let account = store.upsert_account("me@x.com").unwrap();
    (store, account)
}

fn inbox_ids(store: &Store, account: AccountId) -> Vec<String> {
    store.list(account, "INBOX", None, 1000).unwrap().into_iter().map(|m| m.gmail_id).collect()
}

fn small() -> Options {
    Options { inbox_limit: 25, page_size: 10, workers: 4 }
}

#[test]
fn the_first_sync_fetches_the_newest_inbox_page_by_page() {
    let fake = Fake::with_inbox(40);
    let (mut store, a) = setup();
    let mut progress = Vec::new();
    let report = sync(&fake, &mut store, a, &small(), &mut |n| progress.push(n)).unwrap();

    assert_eq!(report, Report { full: true, fetched: 25, deleted: 0 });
    assert_eq!(progress, [10, 20, 25]);
    assert_eq!(store.full_sync_start(a).unwrap(), None);
    let ids = inbox_ids(&store, a);
    assert_eq!(ids.len(), 25);
    assert_eq!(ids.first().map(String::as_str), Some("m039"));
    assert_eq!(store.history_id(a).unwrap().as_deref(), Some("100"));
    assert_eq!(store.labels(a).unwrap().len(), 1);
}

#[test]
fn a_small_mailbox_stops_at_its_end() {
    let fake = Fake::with_inbox(7);
    let (mut store, a) = setup();
    let report = sync(&fake, &mut store, a, &small(), &mut |_| {}).unwrap();
    assert_eq!(report.fetched, 7);
}

#[test]
fn later_syncs_apply_only_the_changes() {
    let fake = Fake::with_inbox(5);
    let (mut store, a) = setup();
    sync(&fake, &mut store, a, &small(), &mut |_| {}).unwrap();
    let before = fake.fetches.load(Ordering::Relaxed);

    // A new message arrives, one is read, one is deleted.
    fake.add(message("new", 99, &["INBOX", "UNREAD"]));
    fake.messages.lock().unwrap().get_mut("m004").unwrap().label_ids = vec!["INBOX".into(), "STARRED".into()];
    fake.messages.lock().unwrap().remove("m000");
    *fake.history_id.lock().unwrap() = "200".into();
    *fake.changes.lock().unwrap() = vec![
        ChangePage { changed: vec!["new".into(), "m004".into()], history_id: "150".into(), ..ChangePage::default() },
        ChangePage {
            changed: vec!["new".into()],
            deleted: vec!["m000".into()],
            history_id: "200".into(),
            ..ChangePage::default()
        },
    ];

    let report = sync(&fake, &mut store, a, &small(), &mut |_| {}).unwrap();
    assert_eq!(report, Report { full: false, fetched: 2, deleted: 1 });
    assert_eq!(fake.fetches.load(Ordering::Relaxed) - before, 2, "only changed messages are fetched");
    assert_eq!(inbox_ids(&store, a), ["new", "m004", "m003", "m002", "m001"]);
    assert!(store.list(a, "INBOX", None, 1).unwrap()[0].unread);
    assert!(store.list(a, "INBOX", None, 2).unwrap()[1].starred);
    assert_eq!(store.history_id(a).unwrap().as_deref(), Some("200"));
}

#[test]
fn a_changed_message_that_is_gone_is_deleted() {
    let fake = Fake::with_inbox(3);
    let (mut store, a) = setup();
    sync(&fake, &mut store, a, &small(), &mut |_| {}).unwrap();
    fake.messages.lock().unwrap().remove("m001");
    *fake.changes.lock().unwrap() =
        vec![ChangePage { changed: vec!["m001".into()], history_id: "101".into(), ..ChangePage::default() }];

    let report = sync(&fake, &mut store, a, &small(), &mut |_| {}).unwrap();
    assert_eq!(report.deleted, 1);
    assert_eq!(inbox_ids(&store, a), ["m002", "m000"]);
}

#[test]
fn an_expired_history_starts_over_with_a_full_sync() {
    let mut fake = Fake::with_inbox(3);
    let (mut store, a) = setup();
    sync(&fake, &mut store, a, &small(), &mut |_| {}).unwrap();
    // Locally known mail that the server no longer has must not survive the resync.
    store.upsert_messages(a, &[message("stale", 50, &["INBOX"])]).unwrap();

    fake.history_expired = true;
    let report = sync(&fake, &mut store, a, &small(), &mut |_| {}).unwrap();
    assert!(report.full);
    assert_eq!(inbox_ids(&store, a), ["m002", "m001", "m000"]);
}

#[test]
fn an_interrupted_first_sync_resumes_where_it_stopped() {
    let mut fake = Fake::with_inbox(30);
    fake.failing.insert("m015".into());
    let (mut store, a) = setup();
    let result = sync(&fake, &mut store, a, &small(), &mut |_| {});
    assert!(matches!(result, Err(Error::Gmail(GmailError::Server(503)))));
    // The page before the failure is kept, and no change position is set yet.
    assert_eq!(inbox_ids(&store, a).len(), 10);
    assert_eq!(store.history_id(a).unwrap(), None);

    // New mail arrives before the retry. The resumed sync keeps its original start (100), so
    // the next sync replays everything since then.
    *fake.history_id.lock().unwrap() = "150".into();
    fake.failing.clear();
    let before = fake.fetches.load(Ordering::Relaxed);
    let report = sync(&fake, &mut store, a, &small(), &mut |_| {}).unwrap();
    assert!(report.full);
    assert_eq!(report.fetched, 15, "only the 15 messages it didn't have");
    assert_eq!(fake.fetches.load(Ordering::Relaxed) - before, 15);
    assert_eq!(inbox_ids(&store, a).len(), 25);
    assert_eq!(store.history_id(a).unwrap().as_deref(), Some("100"));
}
