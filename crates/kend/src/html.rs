//! Email HTML as Markdown for rune-kit's renderer: headings, emphasis, lists, quotes, links and
//! data tables kept; layout tables, images, hidden text and styling dropped. Emails lay out
//! with nested tables, so a table only stays a table when it looks like data (header cells, no
//! tables inside); every other table becomes the blocks it contains.

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use htmd::element_handler::{HandlerResult, Handlers};
use htmd::{Element, HtmlToMarkdown, Node};
use markup5ever_rcdom::NodeData;

const TABLE_TAGS: [&str; 7] = ["table", "thead", "tbody", "tfoot", "tr", "td", "th"];

pub fn to_markdown(html: &str) -> String {
    // Whether each table (by node address) holds data, decided once per table.
    let data_tables: Arc<Mutex<HashMap<usize, bool>>> = Arc::default();
    let converter = HtmlToMarkdown::builder()
        .skip_tags(vec!["head", "style", "script", "noscript", "title", "meta", "link"])
        .add_handler(vec!["div", "span", "p", "center", "font"], |handlers: &dyn Handlers, element: Element| {
            if hidden(&element) { None } else { handlers.fallback(element) }
        })
        .add_handler(vec!["img"], |_: &dyn Handlers, element: Element| {
            // Remote images aren't loaded (they're often trackers); their alt text stands in.
            let alt = attr(&element, "alt").map(str::trim).unwrap_or_default();
            (!alt.is_empty()).then(|| HandlerResult::from(format!("*{}*", alt.replace('*', ""))))
        })
        .add_handler(TABLE_TAGS.to_vec(), move |handlers: &dyn Handlers, element: Element| {
            if hidden(&element) {
                return None;
            }
            let table = nearest_table(element.node)?;
            let mut known = data_tables.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let is_data = *known.entry(Rc::as_ptr(&table) as usize).or_insert_with(|| is_data_table(&table));
            drop(known);
            if is_data {
                // htmd won't write a table inside another table, and email nests every data
                // table in layout tables, so data tables are written here.
                return (element.tag == "table").then(|| data_table(handlers, element.node).into());
            }
            // A layout table: each cell is a block of whatever it holds.
            let inner = handlers.walk_children(element.node).content;
            Some(match element.tag {
                "td" | "th" => format!("\n\n{}\n\n", inner.trim()).into(),
                _ => inner.into(),
            })
        })
        .build();
    tidy(&converter.convert(html).unwrap_or_default())
}

fn attr<'a>(element: &'a Element, name: &str) -> Option<&'a str> {
    element.attrs.iter().find(|a| a.name.local.as_ref() == name).map(|a| a.value.as_ref())
}

/// Hidden by an inline style, as preview-text blocks and tracking spans are.
fn hidden(element: &Element) -> bool {
    let style = attr(element, "style").unwrap_or_default().to_ascii_lowercase().replace(' ', "");
    style.contains("display:none") || style.contains("visibility:hidden") || style.contains("max-height:0")
}

fn tag(node: &Node) -> Option<&str> {
    match &node.data {
        NodeData::Element { name, .. } => Some(name.local.as_ref()),
        _ => None,
    }
}

fn parent(node: &Node) -> Option<Rc<Node>> {
    let weak = node.parent.take();
    let parent = weak.as_ref().and_then(|w| w.upgrade());
    node.parent.set(weak);
    parent
}

fn nearest_table(node: &Rc<Node>) -> Option<Rc<Node>> {
    let mut current = node.clone();
    loop {
        if tag(&current) == Some("table") {
            return Some(current);
        }
        current = parent(&current)?;
    }
}

/// A GitHub-style Markdown table: the first row as the header, cells as one line of inline
/// Markdown each, short rows padded.
fn data_table(handlers: &dyn Handlers, table: &Rc<Node>) -> String {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut stack: Vec<Rc<Node>> = table.children.borrow().iter().rev().cloned().collect();
    while let Some(node) = stack.pop() {
        if tag(&node) == Some("tr") {
            let cells = node
                .children
                .borrow()
                .iter()
                .filter(|c| matches!(tag(c), Some("td" | "th")))
                .map(|cell| {
                    let text = handlers.walk_children(cell).content;
                    text.split_whitespace().collect::<Vec<_>>().join(" ").replace('|', "\\|")
                })
                .collect();
            rows.push(cells);
        } else {
            stack.extend(node.children.borrow().iter().rev().cloned());
        }
    }
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    if columns == 0 {
        return String::new();
    }
    let line = |cells: &[String]| {
        let padded = (0..columns).map(|i| cells.get(i).map_or("", String::as_str));
        format!("| {} |", padded.collect::<Vec<_>>().join(" | "))
    };
    let mut out = format!("\n\n{}\n|{}\n", line(&rows[0]), " --- |".repeat(columns));
    for row in &rows[1..] {
        out.push_str(&line(row));
        out.push('\n');
    }
    out.push('\n');
    out
}

/// Header cells and no tables inside: a receipt's line items, not a newsletter's grid.
fn is_data_table(table: &Rc<Node>) -> bool {
    let (mut headers, mut nested) = (false, false);
    let mut stack: Vec<Rc<Node>> = table.children.borrow().clone();
    while let Some(node) = stack.pop() {
        match tag(&node) {
            Some("th") => headers = true,
            Some("table") => nested = true,
            _ => {}
        }
        stack.extend(node.children.borrow().iter().cloned());
    }
    headers && !nested
}

/// Drops the invisible characters newsletters pad previews with, and runs of blank lines.
fn tidy(markdown: &str) -> String {
    let cleaned: String = markdown
        .chars()
        .filter(|c| !matches!(c, '\u{200b}' | '\u{200c}' | '\u{034f}' | '\u{00ad}' | '\u{feff}'))
        .collect();
    let mut out = String::with_capacity(cleaned.len());
    let mut blank_run = 0;
    for line in cleaned.lines() {
        let line = line.trim_end().replace('\u{a0}', " ");
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run > 1 {
                continue;
            }
            out.push('\n');
            continue;
        }
        blank_run = 0;
        out.push_str(&line);
        out.push('\n');
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structure_survives() {
        let md = to_markdown(
            "<h2>Your order</h2><p>Thanks, <b>Ana</b>! Track it <a href=\"https://x.example/t\">here</a>.</p><ul><li>One</li><li>Two</li></ul>",
        );
        assert!(md.contains("## Your order"), "{md}");
        assert!(md.contains("**Ana**"), "{md}");
        assert!(md.contains("[here](https://x.example/t)"), "{md}");
        assert!(md.contains("One") && md.contains("Two"), "{md}");
    }

    #[test]
    fn layout_tables_become_blocks() {
        let html = "<table><tr><td><table><tr><td><h1>Big sale</h1></td></tr></table></td></tr>\
                    <tr><td><p>Everything is 50% off.</p></td></tr></table>";
        let md = to_markdown(html);
        assert!(!md.contains('|'), "no table pipes: {md}");
        assert!(md.contains("# Big sale"), "{md}");
        assert!(md.contains("Everything is 50% off."), "{md}");
    }

    #[test]
    fn data_tables_stay_tables() {
        let html = "<table><tr><th>Item</th><th>Price</th></tr><tr><td>Coffee</td><td>$4</td></tr></table>";
        let md = to_markdown(html);
        assert!(md.contains("| Item"), "{md}");
        assert!(md.contains("Coffee") && md.contains("$4"), "{md}");
    }

    #[test]
    fn a_data_table_inside_a_layout_table_stays_a_table() {
        let html = "<table><tr><td><p>Your receipt</p>\
                    <table><tr><th>Item</th><th>Price</th></tr><tr><td>Coffee</td><td>$4</td></tr><tr><td>Tea | hot</td></tr></table>\
                    </td></tr></table>";
        let md = to_markdown(html);
        assert!(md.contains("| Item | Price |\n| --- | --- |\n| Coffee | $4 |\n| Tea \\| hot |  |"), "{md}");
        assert!(md.contains("Your receipt"), "{md}");
    }

    #[test]
    fn hidden_text_styles_and_trackers_are_dropped() {
        let html = "<head><style>p{color:red}</style></head><body>\
                    <div style=\"display: none; max-height:0\">Preview text you should not see</div>\
                    <p>Hello</p><img src=\"https://t.example/pixel.gif\" width=1 height=1>\
                    <img src=\"https://cdn.example/logo.png\" alt=\"Acme logo\"></body>";
        let md = to_markdown(html);
        assert!(!md.contains("Preview text"), "{md}");
        assert!(!md.contains("color:red"), "{md}");
        assert!(!md.contains("pixel.gif"), "{md}");
        assert!(md.contains("*Acme logo*"), "{md}");
        assert!(md.contains("Hello"));
    }

    #[test]
    fn invisible_padding_is_removed() {
        let md = to_markdown("<p>Hi\u{200c}\u{034f}\u{200c}\u{034f} there</p><p>&nbsp;</p><p>&nbsp;</p><p>Bye</p>");
        assert_eq!(md, "Hi there\n\nBye");
    }

    #[test]
    fn a_large_newsletter_converts_quickly() {
        let block = "<table><tr><td><h2>Story</h2><p>Some <a href=\"https://news.example/a\">linked</a> text \
                     with <b>bold</b> words and a long sentence that goes on for a while.</p></td></tr></table>";
        let html = block.repeat(1_500); // ~300 KB, larger than most email
        let start = std::time::Instant::now();
        let md = to_markdown(&html);
        let took = start.elapsed();
        assert!(md.contains("## Story"));
        assert!(took.as_millis() < 500, "took {took:?}");
        eprintln!("300 KB email converted in {took:?}");
    }
}
