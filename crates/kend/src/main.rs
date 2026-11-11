//! kend: a fast local email client.

mod app;
mod view;

use rune::AppSpec;

use crate::app::App;

fn main() {
    rune::prewarm_gpu();
    rune::run(AppSpec {
        setup: Some(App::setup),
        poll: Some(App::poll),
        scenes: view::SCENES,
        ..AppSpec::new("kend", (1100.0, 720.0), App::new, view::view)
    });
}
