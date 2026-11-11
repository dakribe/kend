//! Times the store's hot paths on a large synthetic mailbox, in a database on disk.
//! The vocabulary is deliberately tiny (each word is in ~40% of messages), so the broad
//! searches are a worst case; the rare-word search is closer to real use.
//! `cargo run --release -p kend-store --example bench [messages]`

use std::time::Instant;

use kend_store::{Address, NewMessage, Store};

const WORDS: &[&str] = &[
    "invoice", "meeting", "travel", "report", "launch", "dinner", "contract", "update", "review", "photos", "budget",
    "flight", "hiring", "design", "release", "notes", "agenda", "receipt", "offer", "welcome",
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let total: usize = std::env::args().nth(1).and_then(|n| n.parse().ok()).unwrap_or(100_000);
    let dir = std::env::temp_dir().join(format!("kend-bench-{}", std::process::id()));
    let mut store = Store::open(&dir.join("mail.sqlite"))?;
    let account = store.upsert_account("me@example.com")?;

    let messages: Vec<NewMessage> = (0..total)
        .map(|i| {
            let word = |k: usize| WORDS[(i * 7 + k * 13) % WORDS.len()];
            let mut labels = vec!["INBOX".to_string()];
            if i % 5 == 0 {
                labels.push("UNREAD".into());
            }
            NewMessage {
                gmail_id: format!("{i:016x}"),
                thread_id: format!("{:016x}", i / 3),
                from: Address::new(format!("Sender {}", i % 500), format!("sender{}@example.com", i % 500)),
                to: "me@example.com".into(),
                subject: format!("{} {} #{i}", word(0), word(1)),
                snippet: format!("{} {} {}", word(2), word(3), word(4)),
                received_at: 1_700_000_000_000 + i as i64 * 60_000,
                label_ids: labels,
                body_text: Some(format!("{} {} {} {} {}", word(5), word(6), word(7), word(8), word(9))),
                body_html: None,
            }
        })
        .collect();

    let start = Instant::now();
    for chunk in messages.chunks(500) {
        store.upsert_messages(account, chunk)?;
    }
    let insert = start.elapsed();
    println!(
        "insert {total} messages: {:.2} s ({:.0} msg/s)",
        insert.as_secs_f64(),
        total as f64 / insert.as_secs_f64()
    );

    let time = |label: &str, runs: u32, f: &mut dyn FnMut()| {
        let start = Instant::now();
        for _ in 0..runs {
            f();
        }
        println!("{label}: {:.3} ms", start.elapsed().as_secs_f64() * 1000.0 / runs as f64);
    };

    time("first inbox page (50 rows)", 200, &mut || {
        store.list(account, "INBOX", None, 50).map(|_| ()).unwrap_or_default();
    });
    // Walk to a page deep in the mailbox, then time fetching the page after it.
    let mut after = None;
    for _ in 0..(total / 50 / 2) {
        after = store.list(account, "INBOX", after, 50)?.last().map(|m| m.cursor());
    }
    time("a page halfway down", 200, &mut || {
        store.list(account, "INBOX", after, 50).map(|_| ()).unwrap_or_default();
    });
    time("unread count", 20, &mut || {
        store.unread_count(account, "INBOX").map(|_| ()).unwrap_or_default();
    });
    time("search \"invoice meeting\" (top 50)", 50, &mut || {
        store.search(account, "invoice meeting", 50).map(|_| ()).unwrap_or_default();
    });
    time("search prefix \"recei\" (top 50)", 50, &mut || {
        store.search(account, "recei", 50).map(|_| ()).unwrap_or_default();
    });
    // Closer to real mail: a word that appears in a handful of messages.
    time("search a rare word (\"4321\")", 200, &mut || {
        store.search(account, "4321", 50).map(|_| ()).unwrap_or_default();
    });

    drop(store);
    std::fs::remove_dir_all(&dir)?;
    Ok(())
}
