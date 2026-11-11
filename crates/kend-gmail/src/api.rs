//! A small Gmail API client: the calls sync needs, with token refresh and retries built in.
//! One client is shared by sync's worker threads.

use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::http::{agent, json};
use crate::{AccessToken, Account, ClientCredentials, Error, Result, access_token};

const API: &str = "https://gmail.googleapis.com/gmail/v1/users/me";
/// Attempts per request: the first try plus retries after 1, 2, 4, 8 and 16 seconds.
const ATTEMPTS: u32 = 6;
/// Gmail allows each user 250 quota units a second. Staying at 200 leaves headroom for
/// bursts and for other clients on the same account.
const UNITS_PER_SECOND: u32 = 200;

pub struct Gmail {
    client: ClientCredentials,
    account: Account,
    token: Mutex<Option<AccessToken>>,
    pacer: Pacer,
}

/// Spaces requests to a quota-unit budget shared by every thread using the client. Each
/// request reserves the next free slot, then sleeps until it.
struct Pacer {
    next: Mutex<Instant>,
    per_unit: Duration,
}

impl Pacer {
    fn new(units_per_second: u32) -> Self {
        Self { next: Mutex::new(Instant::now()), per_unit: Duration::from_secs(1) / units_per_second }
    }

    fn wait(&self, units: u32) {
        let now = Instant::now();
        let slot = {
            let mut next = self.next.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let slot = (*next).max(now);
            *next = slot + self.per_unit * units;
            slot
        };
        thread::sleep(slot.saturating_duration_since(now));
    }

    /// Holds every thread back for `pause`: when Gmail says slow down, it means all of us.
    fn cool_down(&self, pause: Duration) {
        let mut next = self.next.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        *next = (*next).max(Instant::now() + pause);
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GmailLabel {
    pub id: String,
    pub name: String,
    /// `system` or `user`.
    #[serde(rename = "type", default)]
    pub kind: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageRef {
    pub id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageList {
    #[serde(default)]
    pub messages: Vec<MessageRef>,
    pub next_page_token: Option<String>,
}

/// A message in `metadata` format: labels, snippet and a few headers, no body.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GmailMessage {
    pub id: String,
    pub thread_id: String,
    #[serde(default)]
    pub label_ids: Vec<String>,
    /// HTML-escaped, as Gmail sends it.
    #[serde(default)]
    pub snippet: String,
    /// Unix milliseconds, as a string.
    #[serde(default)]
    pub internal_date: String,
    #[serde(default)]
    payload: Payload,
}

#[derive(Debug, Default, Deserialize)]
struct Payload {
    #[serde(default)]
    headers: Vec<Header>,
}

#[derive(Debug, Deserialize)]
struct Header {
    name: String,
    value: String,
}

impl GmailMessage {
    /// A header's value, matched case-insensitively.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.payload.headers.iter().find(|h| h.name.eq_ignore_ascii_case(name)).map(|h| h.value.as_str())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    #[serde(default)]
    pub history: Vec<HistoryRecord>,
    pub history_id: String,
    pub next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRecord {
    #[serde(default)]
    pub messages_added: Vec<HistoryMessage>,
    #[serde(default)]
    pub messages_deleted: Vec<HistoryMessage>,
    #[serde(default)]
    pub labels_added: Vec<HistoryMessage>,
    #[serde(default)]
    pub labels_removed: Vec<HistoryMessage>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryMessage {
    pub message: MessageRef,
}

impl HistoryRecord {
    /// Messages whose labels or existence changed in this record.
    pub fn changed(&self) -> impl Iterator<Item = &str> {
        self.messages_added.iter().chain(&self.labels_added).chain(&self.labels_removed).map(|m| m.message.id.as_str())
    }

    pub fn deleted(&self) -> impl Iterator<Item = &str> {
        self.messages_deleted.iter().map(|m| m.message.id.as_str())
    }
}

impl Gmail {
    pub fn new(client: ClientCredentials, account: Account) -> Self {
        Self { client, account, token: Mutex::new(None), pacer: Pacer::new(UNITS_PER_SECOND) }
    }

    pub fn account(&self) -> &Account {
        &self.account
    }

    /// The mailbox's current history id: the point a sync started now will be up to date with.
    pub fn history_id(&self) -> Result<String> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Profile {
            history_id: String,
        }
        Ok(self.get::<Profile>("/profile", &[], 1)?.history_id)
    }

    pub fn labels(&self) -> Result<Vec<GmailLabel>> {
        #[derive(Deserialize)]
        struct Labels {
            #[serde(default)]
            labels: Vec<GmailLabel>,
        }
        Ok(self.get::<Labels>("/labels", &[], 1)?.labels)
    }

    /// One page of message ids in `label`, newest first, optionally narrowed by a Gmail
    /// search (`before:1700000000`).
    pub fn list_messages(
        &self,
        label: &str,
        search: Option<&str>,
        page_token: Option<&str>,
        max: u32,
    ) -> Result<MessageList> {
        let max = max.to_string();
        let mut query = vec![("labelIds", label), ("maxResults", max.as_str())];
        query.extend(search.map(|q| ("q", q)));
        query.extend(page_token.map(|t| ("pageToken", t)));
        self.get("/messages", &query, 5)
    }

    /// A message's metadata, or `None` when it no longer exists.
    pub fn message(&self, id: &str) -> Result<Option<GmailMessage>> {
        let query = [
            ("format", "metadata"),
            ("metadataHeaders", "From"),
            ("metadataHeaders", "To"),
            ("metadataHeaders", "Subject"),
        ];
        match self.get(&format!("/messages/{id}"), &query, 5) {
            Ok(message) => Ok(Some(message)),
            Err(Error::NotFound) => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Changes since `start`. [`Error::NotFound`] means `start` is too old and a full sync is
    /// needed.
    pub fn history(&self, start: &str, page_token: Option<&str>) -> Result<History> {
        let mut query = vec![("startHistoryId", start), ("maxResults", "500")];
        query.extend(page_token.map(|t| ("pageToken", t)));
        self.get("/history", &query, 2)
    }

    /// GETs `path`, spending `units` of the quota (Gmail's published cost of the call).
    fn get<T: DeserializeOwned>(&self, path: &str, query: &[(&str, &str)], units: u32) -> Result<T> {
        let url = format!("{API}{path}");
        let mut refreshed = false;
        let mut attempt = 0;
        loop {
            attempt += 1;
            self.pacer.wait(units);
            let bearer = self.bearer()?;
            let request = agent()
                .get(&url)
                .query_pairs(query.iter().copied())
                .header("Authorization", format!("Bearer {bearer}"));
            match request.call().map_err(Error::from).and_then(json) {
                Ok(value) => return Ok(value),
                // The token expired early or was replaced: refresh once and try again.
                Err(Error::Unauthorized) if !refreshed => {
                    refreshed = true;
                    self.forget_token();
                }
                // Pause every thread, not just this one; the pacer makes this request wait too.
                Err(Error::RateLimited) if attempt < ATTEMPTS => {
                    let pause = backoff(attempt);
                    eprintln!("kend: Gmail rate limit, pausing all requests for {:.1} s", pause.as_secs_f64());
                    self.pacer.cool_down(pause);
                }
                Err(err) if err.is_transient() && attempt < ATTEMPTS => thread::sleep(backoff(attempt)),
                Err(err) => return Err(err),
            }
        }
    }

    /// The current access token, refreshed when it's missing or about to expire. Workers
    /// wait on the lock rather than all refreshing at once.
    fn bearer(&self) -> Result<String> {
        let mut token = self.token.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        match token.as_ref() {
            Some(t) if t.is_fresh() => Ok(t.token.clone()),
            _ => {
                let fresh = access_token(&self.client, &self.account)?;
                let bearer = fresh.token.clone();
                *token = Some(fresh);
                Ok(bearer)
            }
        }
    }

    fn forget_token(&self) {
        *self.token.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
    }
}

/// 1, 2, 4, 8, 16 seconds, plus up to a quarter more so parallel workers don't retry in lockstep.
fn backoff(attempt: u32) -> Duration {
    let base = Duration::from_secs(1 << (attempt - 1).min(5));
    base + base.mul_f64(rand::random::<f64>() * 0.25)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pacer_spaces_requests_to_the_budget() {
        // 1000 units a second: ten 5-unit requests need at least 45 ms between the first and last.
        let pacer = Pacer::new(1000);
        let start = Instant::now();
        thread::scope(|s| {
            for _ in 0..10 {
                s.spawn(|| pacer.wait(5));
            }
        });
        assert!(start.elapsed() >= Duration::from_millis(45), "{:?}", start.elapsed());
    }

    #[test]
    fn a_cool_down_holds_back_the_next_request() {
        let pacer = Pacer::new(1000);
        pacer.cool_down(Duration::from_millis(30));
        let start = Instant::now();
        pacer.wait(1);
        assert!(start.elapsed() >= Duration::from_millis(25));
    }

    #[test]
    fn backoff_doubles_with_jitter() {
        for (attempt, secs) in [(1, 1.0), (2, 2.0), (3, 4.0)] {
            let wait = backoff(attempt).as_secs_f64();
            assert!(wait >= secs && wait <= secs * 1.25, "attempt {attempt}: {wait}");
        }
    }

    #[test]
    fn a_metadata_message_parses_with_headers() {
        let json = r#"{"id":"18c","threadId":"18a","labelIds":["INBOX","UNREAD"],"snippet":"Hi &amp; bye",
            "internalDate":"1700000000000","payload":{"headers":[{"name":"From","value":"Ana <ana@x.com>"},
            {"name":"subject","value":"Hello"}]}}"#;
        let m: GmailMessage = serde_json::from_str(json).unwrap();
        assert_eq!(m.header("From"), Some("Ana <ana@x.com>"));
        assert_eq!(m.header("Subject"), Some("Hello"));
        assert_eq!(m.header("To"), None);
        assert_eq!(m.label_ids, ["INBOX", "UNREAD"]);
    }

    #[test]
    fn history_lists_changed_and_deleted_messages() {
        let json = r#"{"historyId":"900","history":[
            {"id":"1","messagesAdded":[{"message":{"id":"a","threadId":"t"}}]},
            {"id":"2","labelsRemoved":[{"message":{"id":"b","threadId":"t"},"labelIds":["UNREAD"]}]},
            {"id":"3","messagesDeleted":[{"message":{"id":"c","threadId":"t"}}]}]}"#;
        let h: History = serde_json::from_str(json).unwrap();
        let changed: Vec<_> = h.history.iter().flat_map(HistoryRecord::changed).collect();
        let deleted: Vec<_> = h.history.iter().flat_map(HistoryRecord::deleted).collect();
        assert_eq!((changed, deleted), (vec!["a", "b"], vec!["c"]));
        assert_eq!(h.history_id, "900");
    }
}
