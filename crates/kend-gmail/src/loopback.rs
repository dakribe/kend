//! The loopback redirect: a one-shot HTTP listener on 127.0.0.1 that receives the
//! authorization code after the user signs in in their browser.

use std::io::{ErrorKind, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

use url::Url;

use crate::{CancelToken, Error, Result};

pub(crate) struct Loopback {
    listener: TcpListener,
    port: u16,
}

/// What a browser request to the listener carried.
#[derive(Debug, PartialEq)]
enum Callback {
    Code(String),
    Declined(String),
    /// Anything else: a favicon request, or a `state` that isn't this sign-in's.
    Ignored,
}

impl Loopback {
    /// Listens on a free port chosen by the OS, reachable only from this machine.
    pub fn bind() -> Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let port = listener.local_addr()?.port();
        listener.set_nonblocking(true)?;
        Ok(Self { listener, port })
    }

    pub fn redirect_uri(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Waits for the redirect carrying `state`, until `timeout` or `cancel`.
    pub fn wait_for_code(&self, state: &str, timeout: Duration, cancel: &CancelToken) -> Result<String> {
        let deadline = Instant::now() + timeout;
        loop {
            if cancel.is_cancelled() {
                return Err(Error::Cancelled);
            }
            if Instant::now() >= deadline {
                return Err(Error::TimedOut);
            }
            match self.listener.accept() {
                Ok((stream, _)) => match handle(stream, state) {
                    Ok(Callback::Code(code)) => return Ok(code),
                    Ok(Callback::Declined(reason)) => return Err(Error::Declined(reason)),
                    Ok(Callback::Ignored) | Err(_) => {}
                },
                Err(err) if err.kind() == ErrorKind::WouldBlock => thread::sleep(Duration::from_millis(50)),
                Err(err) => return Err(err.into()),
            }
        }
    }
}

fn handle(mut stream: TcpStream, state: &str) -> std::io::Result<Callback> {
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let target = read_request_target(&mut stream)?;
    let callback = parse_callback(&target, state);
    let (status, message) = match &callback {
        Callback::Code(_) => ("200 OK", "You're signed in to kend. You can close this tab."),
        Callback::Declined(_) => ("200 OK", "Sign-in was cancelled. You can close this tab."),
        Callback::Ignored => ("404 Not Found", "Not found."),
    };
    let page = format!(
        "<!doctype html><meta charset=utf-8><title>kend</title>\
         <body style=\"font:16px -apple-system,sans-serif;margin:4em;text-align:center\">{message}</body>"
    );
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}",
        page.len()
    )?;
    Ok(callback)
}

/// The path and query of the request line (`GET /?code=… HTTP/1.1`).
fn read_request_target(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut buf = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    while !buf.windows(2).any(|w| w == b"\r\n") && buf.len() < 16 * 1024 {
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let head = String::from_utf8_lossy(&buf);
    let line = head.lines().next().unwrap_or_default();
    Ok(line.split(' ').nth(1).unwrap_or_default().to_string())
}

fn parse_callback(target: &str, expected_state: &str) -> Callback {
    let Ok(url) = Url::parse("http://127.0.0.1").and_then(|base| base.join(target)) else {
        return Callback::Ignored;
    };
    let param = |name: &str| url.query_pairs().find(|(k, _)| k == name).map(|(_, v)| v.into_owned());
    if param("state").as_deref() != Some(expected_state) {
        return Callback::Ignored;
    }
    match (param("code"), param("error")) {
        (Some(code), None) if !code.is_empty() => Callback::Code(code),
        (_, Some(error)) => Callback::Declined(error),
        _ => Callback::Ignored,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_matching_state_yields_the_code() {
        assert_eq!(parse_callback("/?state=abc&code=4%2F0Ab&scope=x", "abc"), Callback::Code("4/0Ab".into()));
    }

    #[test]
    fn a_wrong_or_missing_state_is_ignored() {
        assert_eq!(parse_callback("/?state=evil&code=123", "abc"), Callback::Ignored);
        assert_eq!(parse_callback("/?code=123", "abc"), Callback::Ignored);
        assert_eq!(parse_callback("/favicon.ico", "abc"), Callback::Ignored);
    }

    #[test]
    fn a_declined_consent_is_reported() {
        assert_eq!(
            parse_callback("/?state=abc&error=access_denied", "abc"),
            Callback::Declined("access_denied".into())
        );
    }

    #[test]
    fn the_listener_takes_the_code_from_a_real_request_and_skips_others() {
        let loopback = Loopback::bind().unwrap();
        let addr = format!("127.0.0.1:{}", loopback.port);
        let browser = thread::spawn(move || {
            let get = |path: &str| {
                let mut s = TcpStream::connect(&addr).unwrap();
                write!(s, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
                let mut reply = String::new();
                s.read_to_string(&mut reply).unwrap();
                reply
            };
            let favicon = get("/favicon.ico");
            let signed_in = get("/?state=s1&code=c1");
            (favicon, signed_in)
        });
        let code = loopback.wait_for_code("s1", Duration::from_secs(5), &CancelToken::default()).unwrap();
        let (favicon, signed_in) = browser.join().unwrap();
        assert_eq!(code, "c1");
        assert!(favicon.starts_with("HTTP/1.1 404"));
        assert!(signed_in.starts_with("HTTP/1.1 200") && signed_in.contains("signed in to kend"));
    }

    #[test]
    fn cancelling_stops_the_wait() {
        let loopback = Loopback::bind().unwrap();
        let cancel = CancelToken::default();
        cancel.cancel();
        let result = loopback.wait_for_code("s", Duration::from_secs(5), &cancel);
        assert!(matches!(result, Err(Error::Cancelled)));
    }
}
