extern crate xcb;

mod backend;
mod event;
mod init;
mod window;

use backend::opengl;
use std::process;
use window::Window;

use std::time::{Duration, Instant};

fn main() {
    // NOTE: This screen_num is seemingly not the same as xcb's
    // setup.roots_len() This one only works for xlib's OpenGL ctx
    // calls (which strangely do not work with xcb's screen_num)
    //
    // Use setup.roots_len() for all other xcb-only calls
    // (or they behave unexpectedly)
    let (conn, screen_num) = xcb::Connection::connect_with_xlib_display()
        .unwrap_or_else(|err| {
            eprintln!("Error opening connection to X server: {}", err);
            process::exit(1);
        });
    conn.set_event_queue_owner(xcb::EventQueueOwner::Xcb);
    let root =
        Window::new(&conn, conn.get_setup().roots().last().unwrap().root())
            .unwrap();

    init::extensions::verify(&conn).unwrap_or_else(|err| {
        eprintln!("Error: extension `{}` not found.", err);
        process::exit(1);
    });

    let overlay =
        init::extensions::redirect_subwindows(&conn).unwrap_or_else(|err| {
            eprintln!("Failed redirecting subwindows: {}", err);
            process::exit(1);
        });

    let _win = init::window::create_window(&conn);

    let mut windows = Window::fetch_windows(&conn);
    init::window::request_events(&conn);

    let backend = opengl::Opengl::init(&conn, screen_num, overlay)
        .unwrap_or_else(|err| {
            eprintln!("Unable to initialize backend: {}", err);
            process::exit(1);
        });

    // initial render
    for win in windows.iter_mut().filter(|w| w.mapped) {
        backend.update_glxpixmap(win);
        backend.update_window_texture(win);
        backend.draw_window(win);
    }
    backend.render();
    let mut last_render = Instant::now();
    let refresh_rate = Duration::from_millis((1000.0 / 60.0) as u64);

    loop {
        match conn.wait_for_event() {
            None => break,
            Some(event) => {
                event::handle_event(
                    &conn,
                    &event,
                    &mut windows,
                    &backend,
                    &root,
                    &mut last_render,
                    &refresh_rate,
                );
            }
        }
    }
}
