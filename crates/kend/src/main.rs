//! kend: a fast local email client.

mod actions;
mod app;
mod compose;
mod dates;
mod html;
mod icons;
mod message_list;
mod palette;
mod reading;
mod samples;
mod session;
mod sidebar;
mod sync_worker;
mod view;

use rune::{AppSpec, Chrome};

use crate::app::App;

fn main() {
    rune::prewarm_gpu();
    let spec = AppSpec {
        setup: Some(App::setup),
        poll: Some(App::poll),
        on_key: Some(App::on_key),
        scenes: view::SCENES,
        min_size: Some((640.0, 420.0)),
        ..AppSpec::new("kend", (1100.0, 720.0), App::new, view::view)
    };
    // The traffic lights sit in the sidebar's header band; only marked regions drag the window.
    let chrome = Chrome { full_size_content: true, traffic_lights: Some((16.0, 25.0)), drag_regions: true };
    rune::App::new(spec).chrome(chrome).run();
}
