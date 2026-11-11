//! A message's body from Gmail's `full` format: the MIME tree walked for its text and HTML
//! parts, each decoded from base64url and from its declared charset.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD_INDIFFERENT;
use serde::Deserialize;

/// What a message says, ready to show and store.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MessageBody {
    /// Readable text: the plain part when there is one, else the HTML converted to text.
    pub text: String,
    /// The original HTML, kept for richer rendering later.
    pub html: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Part {
    #[serde(default)]
    mime_type: String,
    #[serde(default)]
    filename: String,
    #[serde(default)]
    headers: Vec<Header>,
    #[serde(default)]
    body: PartBody,
    #[serde(default)]
    parts: Vec<Part>,
}

#[derive(Debug, Deserialize)]
struct Header {
    name: String,
    value: String,
}

#[derive(Debug, Default, Deserialize)]
struct PartBody {
    data: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct FullMessage {
    #[serde(default)]
    pub payload: Part,
}

impl Part {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|h| h.name.eq_ignore_ascii_case(name)).map(|h| h.value.as_str())
    }

    /// The first inline (not attached) part of `mime_type`, depth first, decoded to text.
    fn find(&self, mime_type: &str) -> Option<String> {
        if self.filename.is_empty()
            && self.mime_type.eq_ignore_ascii_case(mime_type)
            && let Some(text) = self.decoded()
        {
            return Some(text);
        }
        self.parts.iter().find_map(|p| p.find(mime_type))
    }

    fn decoded(&self) -> Option<String> {
        let bytes = URL_SAFE_NO_PAD_INDIFFERENT.decode(self.body.data.as_deref()?).ok()?;
        let charset = self.header("Content-Type").and_then(charset);
        let encoding =
            charset.and_then(|c| encoding_rs::Encoding::for_label(c.as_bytes())).unwrap_or(encoding_rs::UTF_8);
        Some(encoding.decode(&bytes).0.into_owned())
    }
}

/// `text/plain; charset="iso-8859-1"` → `iso-8859-1`.
fn charset(content_type: &str) -> Option<&str> {
    content_type.split(';').skip(1).find_map(|param| {
        let (key, value) = param.split_once('=')?;
        key.trim().eq_ignore_ascii_case("charset").then(|| value.trim().trim_matches('"'))
    })
}

impl From<FullMessage> for MessageBody {
    fn from(message: FullMessage) -> Self {
        let root = &message.payload;
        let html = root.find("text/html");
        let text = root
            .find("text/plain")
            .filter(|t| !t.trim().is_empty())
            .or_else(|| html.as_deref().map(html_to_text))
            .unwrap_or_default();
        MessageBody { text: tidy(&text), html }
    }
}

/// HTML as readable text: paragraphs and lists kept, links listed as numbered footnotes.
fn html_to_text(html: &str) -> String {
    // A width this large never wraps; the view wraps to the window.
    html2text::config::plain().no_table_borders().string_from_read(html.as_bytes(), 10_000).unwrap_or_default()
}

/// Unix line endings and at most one blank line in a row, as mail is often padded.
fn tidy(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut blank_run = 0;
    for line in text.replace("\r\n", "\n").lines() {
        let line = line.trim_end();
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run > 1 {
                continue;
            }
        } else {
            blank_run = 0;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b64(s: &[u8]) -> String {
        base64::engine::general_purpose::URL_SAFE.encode(s)
    }

    fn parse(json: String) -> MessageBody {
        serde_json::from_str::<FullMessage>(&json).unwrap().into()
    }

    #[test]
    fn a_multipart_alternative_prefers_the_plain_part() {
        let body = parse(format!(
            r#"{{"payload":{{"mimeType":"multipart/alternative",
              "parts":[{{"mimeType":"text/plain","body":{{"data":"{}"}}}},{{"mimeType":"text/html","body":{{"data":"{}"}}}}]}}}}"#,
            b64(b"Hi Ana,\r\n\r\n\r\n\r\nSee you Friday."),
            b64(b"<p>Hi Ana,</p><p>See you <b>Friday</b>.</p>")
        ));
        assert_eq!(body.text, "Hi Ana,\n\nSee you Friday.");
        assert!(body.html.unwrap().contains("<b>Friday</b>"));
    }

    #[test]
    fn html_only_mail_becomes_readable_text() {
        let html =
            b"<html><body><h1>Sale</h1><p>Everything <a href=\"https://shop.example\">50% off</a>.</p></body></html>";
        let body = parse(format!(r#"{{"payload":{{"mimeType":"text/html","body":{{"data":"{}"}}}}}}"#, b64(html)));
        assert!(body.text.contains("Sale"), "{}", body.text);
        assert!(body.text.contains("50% off"));
        assert!(body.text.contains("https://shop.example"), "links are kept: {}", body.text);
        assert!(!body.text.contains('<'));
    }

    #[test]
    fn nested_parts_and_attachments() {
        // mixed → [alternative → [plain, html], attachment named notes.txt]
        let body = parse(format!(
            r#"{{"payload":{{"mimeType":"multipart/mixed","parts":[
                {{"mimeType":"multipart/alternative","parts":[{{"mimeType":"text/plain","body":{{"data":"{}"}}}}]}},
                {{"mimeType":"text/plain","filename":"notes.txt","body":{{"data":"{}"}}}}]}}}}"#,
            b64(b"The real body"),
            b64(b"an attachment, not the body")
        ));
        assert_eq!(body.text, "The real body");
    }

    #[test]
    fn declared_charsets_are_decoded() {
        let latin1 = b"Caf\xe9 at 8"; // "Café" in ISO-8859-1
        let body = parse(format!(
            r#"{{"payload":{{"mimeType":"text/plain","headers":[{{"name":"Content-Type","value":"text/plain; charset=\"ISO-8859-1\""}}],
              "body":{{"data":"{}"}}}}}}"#,
            b64(latin1)
        ));
        assert_eq!(body.text, "Café at 8");
    }

    #[test]
    fn unpadded_base64_decodes() {
        let data = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("ok?");
        let body = parse(format!(r#"{{"payload":{{"mimeType":"text/plain","body":{{"data":"{data}"}}}}}}"#));
        assert_eq!(body.text, "ok?");
    }
}
