//! Outgoing mail: an RFC 5322 message built from what the user typed, sent with Gmail's
//! `messages.send`. Plain text, UTF-8, base64 bodies; replies carry the headers that keep them
//! in their conversation.

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE};

/// A message to send.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outgoing {
    pub from: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub subject: String,
    pub body: String,
    /// The Gmail thread a reply belongs to.
    pub thread_id: Option<String>,
    /// The `Message-ID` being replied to, and its `References`, so other mail apps thread it.
    pub in_reply_to: Option<String>,
    pub references: Option<String>,
}

/// Splits a typed recipient list ("Ana <ana@x.com>, bo@y.com") into addresses. Commas inside
/// quoted names don't split. Errors name the entry that isn't an address.
pub fn parse_recipients(input: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for c in input.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                current.push(c);
            }
            ',' | ';' if !quoted => out.push(std::mem::take(&mut current)),
            '\r' | '\n' => current.push(' '),
            _ => current.push(c),
        }
    }
    out.push(current);
    let out: Vec<String> = out.into_iter().map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect();
    for address in &out {
        let email = address.rsplit_once('<').map_or(address.as_str(), |(_, rest)| rest.trim_end_matches('>'));
        let valid = email.split_once('@').is_some_and(|(user, host)| !user.is_empty() && host.contains('.'))
            && !email.contains(char::is_whitespace);
        if !valid {
            return Err(format!("\"{address}\" isn't an email address"));
        }
    }
    Ok(out)
}

impl Outgoing {
    /// The message as RFC 5322 text with CRLF line endings.
    pub fn to_rfc5322(&self) -> String {
        let mut headers = vec![("From", self.from.clone()), ("To", self.to.join(", "))];
        if !self.cc.is_empty() {
            headers.push(("Cc", self.cc.join(", ")));
        }
        headers.push(("Subject", encode_header(&self.subject)));
        if let Some(id) = &self.in_reply_to {
            headers.push(("In-Reply-To", id.clone()));
            let references = match &self.references {
                Some(refs) if !refs.trim().is_empty() => format!("{} {id}", refs.trim()),
                _ => id.clone(),
            };
            headers.push(("References", references));
        }
        headers.push(("MIME-Version", "1.0".into()));
        headers.push(("Content-Type", "text/plain; charset=\"UTF-8\"".into()));
        headers.push(("Content-Transfer-Encoding", "base64".into()));

        let mut out = String::new();
        for (name, value) in headers {
            // A value can't start a new header: line breaks typed into a field become spaces.
            let value = value.replace(['\r', '\n'], " ");
            out.push_str(&format!("{name}: {value}\r\n"));
        }
        out.push_str("\r\n");
        let body = self.body.replace("\r\n", "\n").replace('\n', "\r\n");
        let encoded = STANDARD.encode(body.as_bytes());
        for line in encoded.as_bytes().chunks(76) {
            out.push_str(std::str::from_utf8(line).unwrap_or_default());
            out.push_str("\r\n");
        }
        out
    }

    /// What `messages.send` takes: the message, base64url encoded, and its thread.
    pub(crate) fn request(&self) -> serde_json::Value {
        let raw = URL_SAFE.encode(self.to_rfc5322().as_bytes());
        match &self.thread_id {
            Some(thread) => serde_json::json!({ "raw": raw, "threadId": thread }),
            None => serde_json::json!({ "raw": raw }),
        }
    }
}

/// Non-ASCII header text as RFC 2047 encoded words, each under the 75-character limit.
fn encode_header(value: &str) -> String {
    if value.is_ascii() {
        return value.to_string();
    }
    let mut words = Vec::new();
    let mut chunk = String::new();
    for c in value.chars() {
        // 45 bytes of UTF-8 is 60 of base64, plus 12 for "=?UTF-8?B??=".
        if chunk.len() + c.len_utf8() > 45 {
            words.push(std::mem::take(&mut chunk));
        }
        chunk.push(c);
    }
    words.push(chunk);
    words.iter().map(|w| format!("=?UTF-8?B?{}?=", STANDARD.encode(w.as_bytes()))).collect::<Vec<_>>().join(" ")
}

/// "Re: Lunch" stays "Re: Lunch"; "Lunch" becomes "Re: Lunch".
pub fn reply_subject(subject: &str) -> String {
    prefixed(subject, "Re:")
}

pub fn forward_subject(subject: &str) -> String {
    prefixed(subject, "Fwd:")
}

fn prefixed(subject: &str, prefix: &str) -> String {
    let subject = subject.trim();
    if subject.len() >= prefix.len() && subject[..prefix.len()].eq_ignore_ascii_case(prefix) {
        subject.to_string()
    } else {
        format!("{prefix} {subject}").trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipients_split_on_commas_outside_quotes() {
        assert_eq!(
            parse_recipients("\"Lima, Ana\" <ana@x.com>, bo@y.org; "),
            Ok(vec!["\"Lima, Ana\" <ana@x.com>".to_string(), "bo@y.org".to_string()])
        );
        assert_eq!(parse_recipients("  "), Ok(vec![]));
        assert!(parse_recipients("ana@x.com, not an address").unwrap_err().contains("not an address"));
        assert!(parse_recipients("ana@localhost").is_err());
    }

    #[test]
    fn a_message_has_mime_headers_and_a_base64_body() {
        let message = Outgoing {
            from: "me@gmail.com".into(),
            to: vec!["ana@x.com".into()],
            subject: "Hi".into(),
            body: "Line one\nLine two".into(),
            ..Outgoing::default()
        };
        let raw = message.to_rfc5322();
        assert!(raw.starts_with("From: me@gmail.com\r\nTo: ana@x.com\r\nSubject: Hi\r\n"), "{raw}");
        assert!(raw.contains("Content-Type: text/plain; charset=\"UTF-8\"\r\n"));
        assert!(!raw.contains("Cc:"));
        let body = raw.split("\r\n\r\n").nth(1).unwrap().replace("\r\n", "");
        assert_eq!(STANDARD.decode(body).unwrap(), b"Line one\r\nLine two");
    }

    #[test]
    fn typed_line_breaks_cannot_add_headers() {
        let message = Outgoing {
            from: "me@gmail.com".into(),
            to: vec!["ana@x.com".into()],
            subject: "Hi\r\nBcc: everyone@x.com".into(),
            ..Outgoing::default()
        };
        let raw = message.to_rfc5322();
        assert!(!raw.contains("\r\nBcc:"), "{raw}");
    }

    #[test]
    fn replies_carry_threading_headers() {
        let message = Outgoing {
            in_reply_to: Some("<b@mail>".into()),
            references: Some("<a@mail>".into()),
            thread_id: Some("t1".into()),
            ..Outgoing::default()
        };
        let raw = message.to_rfc5322();
        assert!(raw.contains("In-Reply-To: <b@mail>\r\nReferences: <a@mail> <b@mail>\r\n"), "{raw}");
        assert_eq!(message.request()["threadId"], "t1");
    }

    #[test]
    fn non_ascii_subjects_are_encoded_words() {
        let encoded = encode_header("Café ☕ on Friday, and a long tail to force a second encoded word");
        assert!(encoded.starts_with("=?UTF-8?B?"));
        assert!(encoded.split(' ').all(|w| w.len() <= 75), "{encoded}");
        let decoded: String = encoded
            .split(' ')
            .map(|w| String::from_utf8(STANDARD.decode(&w[10..w.len() - 2]).unwrap()).unwrap())
            .collect();
        assert_eq!(decoded, "Café ☕ on Friday, and a long tail to force a second encoded word");
    }

    #[test]
    fn subjects_get_one_prefix() {
        assert_eq!(reply_subject("Lunch"), "Re: Lunch");
        assert_eq!(reply_subject("RE: Lunch"), "RE: Lunch");
        assert_eq!(forward_subject("Lunch"), "Fwd: Lunch");
        assert_eq!(reply_subject(""), "Re:");
    }
}
