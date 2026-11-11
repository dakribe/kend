use crate::{AccountId, Address, NewMessage, Store};

pub(crate) fn msg(gmail_id: &str, received_at: i64, labels: &[&str]) -> NewMessage {
    NewMessage {
        gmail_id: gmail_id.into(),
        thread_id: format!("t-{gmail_id}"),
        from: Address::new("Ana Lima", "ana@example.com"),
        to: "me@example.com".into(),
        subject: format!("Subject {gmail_id}"),
        snippet: "snippet".into(),
        received_at,
        label_ids: labels.iter().map(|l| l.to_string()).collect(),
        body_text: None,
        body_html: None,
    }
}

fn store() -> (Store, AccountId) {
    let store = Store::open_in_memory().unwrap();
    let account = store.upsert_account("me@example.com").unwrap();
    (store, account)
}

fn ids(rows: &[crate::MessageSummary]) -> Vec<&str> {
    rows.iter().map(|m| m.gmail_id.as_str()).collect()
}

#[test]
fn a_mailbox_lists_newest_first_with_flags() {
    let (mut store, a) = store();
    store
        .upsert_messages(
            a,
            &[msg("old", 1, &["INBOX"]), msg("new", 3, &["INBOX", "UNREAD", "STARRED"]), msg("sent", 2, &["SENT"])],
        )
        .unwrap();
    let inbox = store.list(a, "INBOX", None, 10).unwrap();
    assert_eq!(ids(&inbox), ["new", "old"]);
    assert!(inbox[0].unread && inbox[0].starred);
    assert!(!inbox[1].unread && !inbox[1].starred);
    assert_eq!(store.unread_count(a, "INBOX").unwrap(), 1);
    assert_eq!(store.count(a, "INBOX").unwrap(), 2);
}

#[test]
fn unread_counts_cover_every_label_at_once() {
    let (mut store, a) = store();
    store
        .upsert_messages(
            a,
            &[
                msg("m1", 1, &["INBOX", "UNREAD"]),
                msg("m2", 2, &["INBOX", "Label_1", "UNREAD"]),
                msg("m3", 3, &["Label_1"]),
            ],
        )
        .unwrap();
    let counts = store.unread_counts(a).unwrap();
    assert_eq!(counts.get("INBOX"), Some(&2));
    assert_eq!(counts.get("Label_1"), Some(&1));
    assert_eq!(counts.get("SENT"), None);
}

#[test]
fn missing_lists_ids_not_yet_stored() {
    let (mut store, a) = store();
    store.upsert_messages(a, &[msg("have", 1, &["INBOX"])]).unwrap();
    let ids = ["new1".to_string(), "have".to_string(), "new2".to_string()];
    assert_eq!(store.missing(a, &ids).unwrap(), ["new1", "new2"]);
}

#[test]
fn pages_continue_from_the_cursor_without_gaps_or_repeats() {
    let (mut store, a) = store();
    // Several messages share a timestamp, so the cursor must break ties on id.
    let all: Vec<_> = (0..25).map(|i| msg(&format!("m{i:02}"), i / 3, &["INBOX"])).collect();
    store.upsert_messages(a, &all).unwrap();

    let mut seen = Vec::new();
    let mut after = None;
    loop {
        let page = store.list(a, "INBOX", after, 10).unwrap();
        let Some(last) = page.last() else { break };
        after = Some(last.cursor());
        seen.extend(page.iter().map(|m| m.gmail_id.clone()));
    }
    assert_eq!(seen.len(), 25);
    let mut unique = seen.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), 25);
}

#[test]
fn upserting_again_updates_in_place_and_keeps_the_body() {
    let (mut store, a) = store();
    let mut first = msg("m1", 1, &["INBOX", "UNREAD"]);
    first.body_text = Some("hello there".into());
    let [id] = store.upsert_messages(a, &[first]).unwrap()[..] else { panic!() };

    let again = msg("m1", 1, &["INBOX"]); // read now; sync sent no body this time
    assert_eq!(store.upsert_messages(a, &[again]).unwrap(), [id]);

    let m = store.message(id).unwrap().unwrap();
    assert!(!m.summary.unread);
    assert_eq!(m.body_text.as_deref(), Some("hello there"));
    assert_eq!(m.label_ids, ["INBOX"]);
}

#[test]
fn set_labels_moves_a_message_between_mailboxes() {
    let (mut store, a) = store();
    store.upsert_messages(a, &[msg("m1", 1, &["INBOX", "UNREAD"])]).unwrap();
    assert!(store.set_labels(a, "m1", &["Label_9".into()]).unwrap());
    assert!(store.list(a, "INBOX", None, 10).unwrap().is_empty());
    let archived = store.list(a, "Label_9", None, 10).unwrap();
    assert_eq!(ids(&archived), ["m1"]);
    assert!(!archived[0].unread);
    assert!(!store.set_labels(a, "missing", &[]).unwrap());
}

#[test]
fn deleting_removes_the_message_from_lists_and_search() {
    let (mut store, a) = store();
    store.upsert_messages(a, &[msg("m1", 1, &["INBOX"]), msg("m2", 2, &["INBOX"])]).unwrap();
    assert_eq!(store.delete_messages(a, &["m1".into(), "nope".into()]).unwrap(), 1);
    assert_eq!(ids(&store.list(a, "INBOX", None, 10).unwrap()), ["m2"]);
    assert_eq!(ids(&store.search(a, "m1", 10).unwrap()), Vec::<&str>::new());
}

#[test]
fn accounts_never_see_each_others_mail() {
    let (mut store, a) = store();
    let b = store.upsert_account("other@example.com").unwrap();
    store.upsert_messages(a, &[msg("m1", 1, &["INBOX"])]).unwrap();
    store.upsert_messages(b, &[msg("m1", 1, &["INBOX"])]).unwrap();
    assert_eq!(store.search(a, "subject", 10).unwrap().len(), 1);
    store.delete_account(b).unwrap();
    assert!(store.search(b, "subject", 10).unwrap().is_empty());
    assert_eq!(store.list(a, "INBOX", None, 10).unwrap().len(), 1);
    assert!(store.list(b, "INBOX", None, 10).unwrap().is_empty());
}

#[test]
fn search_matches_words_and_prefixes_ranking_subjects_first() {
    let (mut store, a) = store();
    let mut in_body = msg("body", 5, &["INBOX"]);
    in_body.body_text = Some("the invoice is attached".into());
    let mut in_subject = msg("subject", 1, &["INBOX"]);
    in_subject.subject = "Invoice for October".into();
    let mut other = msg("other", 9, &["INBOX"]);
    other.from = Address::new("Bo", "bo@example.com");
    store.upsert_messages(a, &[in_body, in_subject, other]).unwrap();

    assert_eq!(ids(&store.search(a, "invoice", 10).unwrap()), ["subject", "body"]);
    assert_eq!(ids(&store.search(a, "invo", 10).unwrap()), ["subject", "body"]);
    assert_eq!(ids(&store.search(a, "invoice october", 10).unwrap()), ["subject"]);
    assert_eq!(ids(&store.search(a, "bo@example", 10).unwrap()), ["other"]);
}

#[test]
fn a_body_fetched_later_becomes_searchable() {
    let (mut store, a) = store();
    let [id] = store.upsert_messages(a, &[msg("m1", 1, &["INBOX"])]).unwrap()[..] else { panic!() };
    assert!(store.search(a, "quarterly", 10).unwrap().is_empty());
    store.set_body(id, Some("quarterly report"), None).unwrap();
    assert_eq!(ids(&store.search(a, "quarterly", 10).unwrap()), ["m1"]);
}

#[test]
fn opens_are_counted() {
    let (mut store, a) = store();
    let [id] = store.upsert_messages(a, &[msg("m1", 1, &["INBOX"])]).unwrap()[..] else { panic!() };
    store.mark_opened(id, 1_000).unwrap();
    store.mark_opened(id, 2_000).unwrap();
    let (count, last): (i64, i64) = store
        .conn
        .query_row("SELECT open_count, last_opened_at FROM messages WHERE id = ?1", [id.0], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!((count, last), (2, 2_000));
}

#[test]
fn mailbox_pages_read_from_the_covering_index() {
    let (store, a) = store();
    let plan: Vec<String> = store
        .conn
        .prepare(
            "EXPLAIN QUERY PLAN
             SELECT m.id FROM message_labels ml JOIN messages m ON m.id = ml.message_id
             WHERE ml.account_id = ?1 AND ml.label_id = ?2 AND (ml.received_at, ml.message_id) < (?3, ?4)
             ORDER BY ml.received_at DESC, ml.message_id DESC LIMIT 50",
        )
        .unwrap()
        .query_map(rusqlite::params![a.0, "INBOX", 0, 0], |r| r.get::<_, String>(3))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let plan = plan.join("\n");
    assert!(plan.contains("SEARCH ml USING PRIMARY KEY"), "{plan}");
    assert!(!plan.contains("TEMP B-TREE"), "no sort step expected:\n{plan}");
}
