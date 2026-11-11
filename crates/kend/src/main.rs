//! kend: a fast local email client.

use rune::tok::{self, type_};
use rune::{AppSpec, Cx, El, col, text};

#[derive(Default)]
struct State {}

fn view(_s: &State, cx: &mut Cx<State>) -> El {
    let t = tok::theme(cx.system_dark());
    let title = text("Inbox").key("title").font(type_::TITLE).color(t.text);
    let empty = text("No messages yet").key("empty").font(type_::BODY).color(t.text_secondary);
    col().w_full().h_full().center().gap(8.0).fill(t.bg).child(title).child(empty)
}

fn main() {
    rune::prewarm_gpu();
    rune::run(AppSpec::new("kend", (1100.0, 720.0), State::default, view));
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune::{Env, Runtime};

    #[test]
    fn the_empty_inbox_renders() {
        let env = Env { system_dark: true, scale: 2.0 };
        let mut rt = Runtime::new(State::default(), view, (1100.0, 720.0), env);
        rt.frame(0.0);
        assert!(rt.find("title").is_some());
        assert!(rt.find("empty").is_some());
    }
}
