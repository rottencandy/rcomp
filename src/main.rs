extern crate xcb;

mod backend;
mod event;
mod init;
mod state;
mod window;

use backend::opengl;
use backend::opengl::buffer::Buffer;
use state::State;
use std::process;
use window::Window;

use std::time::{Duration, Instant};

fn main() {
    let state = State::init().unwrap();

    let mut windows = Window::fetch_windows(&state.conn);
    init::window::request_events(&state.conn);

    let backend = opengl::Opengl::init(&state).unwrap_or_else(|err| {
        eprintln!("Unable to initialize backend: {}", err);
        process::exit(1);
    });

    // initial render
    for win in windows.iter_mut().filter(|w| w.mapped) {
        //TODO: enum-based event handler
        backend.init_window(win);
        backend.update_pos(win);
        backend.update_pixmap(win);
        backend.update_texture(win);
        backend.draw_window(win);
    }
    backend.render();
    let mut last_render = Instant::now();
    // 60 FPS = 16.666 ms
    let update_frequency = Duration::from_millis(17);

    loop {
        match state.conn.wait_for_event() {
            None => break,
            Some(event) => {
                event::handle_event(
                    &state,
                    event,
                    &mut windows,
                    &backend,
                    &mut last_render,
                    &update_frequency,
                );
            }
        }
    }
}
