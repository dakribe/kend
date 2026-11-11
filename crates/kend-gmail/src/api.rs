//! A small Gmail API client: the calls sync needs, with token refresh and retries built in.
//! One client is shared by sync's worker threads.

use std::sync::{Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::body::{FullMessage, MessageBody};
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

/// Paces requests for every thread using the client. Two limits apply: requests start no
/// faster than the current rate allows (quota units a second), and at most `MAX_IN_FLIGHT` run
/// at once, as Gmail also limits concurrent requests per user. The rate adapts: Gmail pushing
/// back halves it, and each success raises it a little (additive increase, multiplicative
/// decrease, as TCP does), so it settles where Gmail stops refusing.
struct Pacer {
    state: Mutex<Pace>,
    freed: Condvar,
}

struct Pace {
    next: Instant,
    /// Quota units a second.
    rate: f64,
    in_flight: usize,
}

/// Held while a request runs; dropping it frees the slot.
struct Permit<'a>(&'a Pacer);

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        self.0.lock().in_flight -= 1;
        self.0.freed.notify_one();
    }
}

impl Pacer {
    const MAX_IN_FLIGHT: usize = 4;
    const MIN_RATE: f64 = 25.0;

    fn new(units_per_second: u32) -> Self {
        let pace = Pace { next: Instant::now(), rate: f64::from(units_per_second), in_flight: 0 };
        Self { state: Mutex::new(pace), freed: Condvar::new() }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Pace> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Waits for a free slot and this request's turn, then holds the slot until dropped.
    fn begin(&self, units: u32) -> Permit<'_> {
        let mut pace = self.lock();
        while pace.in_flight >= Self::MAX_IN_FLIGHT {
            pace = self.freed.wait(pace).unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        pace.in_flight += 1;
        let now = Instant::now();
        let slot = pace.next.max(now);
        pace.next = slot + Duration::from_secs_f64(f64::from(units) / pace.rate);
        drop(pace);
        thread::sleep(slot.saturating_duration_since(now));
        Permit(self)
    }

    /// Gmail said slow down: halve the rate and hold every thread back for `pause`. Returns
    /// the new rate.
    fn slow_down(&self, pause: Duration) -> f64 {
        let mut pace = self.lock();
        pace.rate = (pace.rate / 2.0).max(Self::MIN_RATE);
        pace.next = pace.next.max(Instant::now() + pause);
        pace.rate
    }

    /// A request went through: creep back toward the full rate.
    fn speed_up(&self) {
        let mut pace = self.lock();
        pace.rate = (pace.rate + 2.0).min(f64::from(UNITS_PER_SECOND));
    }

    #[cfg(test)]
    fn rate(&self) -> f64 {
        self.lock().rate
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

    /// A message's body, or `None` when it no longer exists.
    pub fn body(&self, id: &str) -> Result<Option<MessageBody>> {
        match self.get::<FullMessage>(&format!("/messages/{id}"), &[("format", "full")], 5) {
            Ok(message) => Ok(Some(message.into())),
            Err(Error::NotFound) => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Sends a message. Returns its Gmail id. Sending costs 100 quota units.
    pub fn send(&self, message: &crate::Outgoing) -> Result<String> {
        #[derive(Deserialize)]
        struct Sent {
            id: String,
        }
        Ok(self.post::<Sent>("/messages/send", &message.request(), 100)?.id)
    }

    /// The `Message-ID` and `References` headers a reply to `id` needs, so it threads in
    /// other mail apps too.
    pub fn reply_headers(&self, id: &str) -> Result<(Option<String>, Option<String>)> {
        let query = [("format", "metadata"), ("metadataHeaders", "Message-ID"), ("metadataHeaders", "References")];
        let message: GmailMessage = self.get(&format!("/messages/{id}"), &query, 5)?;
        Ok((message.header("Message-ID").map(str::to_string), message.header("References").map(str::to_string)))
    }

    /// Adds and removes labels on a message (`UNREAD` to mark it unread or read).
    pub fn modify_labels(&self, id: &str, add: &[&str], remove: &[&str]) -> Result<()> {
        let body = serde_json::json!({ "addLabelIds": add, "removeLabelIds": remove });
        self.post::<serde_json::Value>(&format!("/messages/{id}/modify"), &body, 5).map(|_| ())
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
        self.call(units, |bearer| {
            agent().get(&url).query_pairs(query.iter().copied()).header("Authorization", bearer).call()
        })
    }

    /// POSTs `body` as JSON to `path`.
    fn post<T: DeserializeOwned>(&self, path: &str, body: &serde_json::Value, units: u32) -> Result<T> {
        let url = format!("{API}{path}");
        self.call(units, |bearer| agent().post(&url).header("Authorization", bearer).send_json(body))
    }

    /// Sends a request made by `send` (given the `Authorization` value), paced to the quota, with
    /// a token refresh on 401 and backoff on rate limits and transient failures.
    fn call<T: DeserializeOwned>(
        &self,
        units: u32,
        send: impl Fn(&str) -> std::result::Result<ureq::http::Response<ureq::Body>, ureq::Error>,
    ) -> Result<T> {
        let mut refreshed = false;
        let mut attempt = 0;
        loop {
            attempt += 1;
            let permit = self.pacer.begin(units);
            let bearer = format!("Bearer {}", self.bearer()?);
            let result = send(&bearer).map_err(Error::from).and_then(json);
            drop(permit);
            match result {
                Ok(value) => {
                    self.pacer.speed_up();
                    return Ok(value);
                }
                // The token expired early or was replaced: refresh once and try again.
                Err(Error::Unauthorized) if !refreshed => {
                    refreshed = true;
                    self.forget_token();
                }
                // Pause every thread, not just this one; the pacer makes this request wait too.
                Err(Error::RateLimited) if attempt < ATTEMPTS => {
                    let rate = self.pacer.slow_down(backoff(attempt));
                    eprintln!("kend: Gmail rate limit, slowing to {rate:.0} quota units/s");
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
                s.spawn(|| drop(pacer.begin(5)));
            }
        });
        assert!(start.elapsed() >= Duration::from_millis(45), "{:?}", start.elapsed());
    }

    #[test]
    fn no_more_than_four_requests_run_at_once() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let pacer = Pacer::new(100_000);
        let (running, peak) = (AtomicUsize::new(0), AtomicUsize::new(0));
        thread::scope(|s| {
            for _ in 0..12 {
                s.spawn(|| {
                    let _permit = pacer.begin(1);
                    let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(now, Ordering::SeqCst);
                    thread::sleep(Duration::from_millis(20));
                    running.fetch_sub(1, Ordering::SeqCst);
                });
            }
        });
        assert_eq!(peak.load(Ordering::SeqCst), Pacer::MAX_IN_FLIGHT);
    }

    #[test]
    fn pushback_halves_the_rate_and_success_wins_it_back_slowly() {
        let pacer = Pacer::new(UNITS_PER_SECOND);
        assert_eq!(pacer.slow_down(Duration::ZERO), 100.0);
        assert_eq!(pacer.slow_down(Duration::ZERO), 50.0);
        for _ in 0..4 {
            pacer.slow_down(Duration::ZERO);
        }
        assert_eq!(pacer.rate(), Pacer::MIN_RATE, "never stops entirely");
        pacer.speed_up();
        assert_eq!(pacer.rate(), Pacer::MIN_RATE + 2.0);
    }

    #[test]
    fn a_pause_holds_back_the_next_request() {
        let pacer = Pacer::new(1000);
        pacer.slow_down(Duration::from_millis(30));
        let start = Instant::now();
        drop(pacer.begin(1));
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
