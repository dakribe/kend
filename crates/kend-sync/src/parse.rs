//! Turning Gmail's header and snippet text into display text.

use kend_store::Address;

/// `Ana Lima <ana@x.com>`, `"Lima, Ana" <ana@x.com>`, `<ana@x.com>` or `ana@x.com`.
pub(crate) fn parse_address(header: &str) -> Address {
    // Only the first address of a list: the sender is one person.
    let header = header.trim();
    match (header.rfind('<'), header.ends_with('>')) {
        (Some(open), true) => {
            let name = header[..open]
                .trim()
                .trim_matches('"')
                .replace("\\\"", "\"");
            Address::new(name, header[open + 1..header.len() - 1].trim())
        }
        _ => Address::new("", header),
    }
}

/// Gmail escapes snippets as HTML (`&#39;`, `&amp;`). Turns them back into text.
pub(crate) fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let decoded = rest
            .find(';')
            .filter(|&end| end <= 10)
            .and_then(|end| Some((entity(&rest[1..end])?, end)));
        match decoded {
            Some((c, end)) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn entity(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some('\u{a0}'),
        _ => {
            let code = match name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => name.strip_prefix('#')?.parse().ok()?,
            };
            char::from_u32(code)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_parse_in_every_common_form() {
        assert_eq!(
            parse_address("Ana Lima <ana@x.com>"),
            Address::new("Ana Lima", "ana@x.com")
        );
        assert_eq!(
            parse_address("\"Lima, Ana\" <ana@x.com>"),
            Address::new("Lima, Ana", "ana@x.com")
        );
        assert_eq!(parse_address("<ana@x.com>"), Address::new("", "ana@x.com"));
        assert_eq!(parse_address(" ana@x.com "), Address::new("", "ana@x.com"));
        assert_eq!(parse_address(""), Address::new("", ""));
    }

    #[test]
    fn snippet_entities_decode() {
        assert_eq!(
            decode_entities("It&#39;s Tom &amp; Jerry &lt;3"),
            "It's Tom & Jerry <3"
        );
        assert_eq!(decode_entities("&#x1F600; &quot;hi&quot;"), "😀 \"hi\"");
        assert_eq!(
            decode_entities("AT&T & co; &bogus; &"),
            "AT&T & co; &bogus; &"
        );
    }
}
