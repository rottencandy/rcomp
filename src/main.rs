extern crate xcb;

mod event;
mod init;
mod opengl;
mod window;

use std::error::Error;
use std::process;
use window::Window;

fn main() -> Result<(), Box<dyn Error>> {
    let (conn, screen_num) = xcb::Connection::connect_with_xlib_display()
        .unwrap_or_else(|err| {
            eprintln!("Error opening connection to X server: {}", err);
            process::exit(1);
        });
    conn.set_event_queue_owner(xcb::EventQueueOwner::Xcb);

    init::extensions::verify(&conn).unwrap_or_else(|err| {
        eprintln!("Error: extension `{}` not found.", err);
        process::exit(1);
    });

    let overlay =
        init::extensions::redirect_subwindows(&conn).unwrap_or_else(|err| {
            eprintln!("Failed redirecting subwindows: {}", err);
            process::exit(1);
        });

    let _win = init::window::create_window(&conn, screen_num);

    // TODO use linked list
    let mut windows = Window::fetch_windows(&conn);
    init::window::request_events(&conn);

    let backend = opengl::Opengl::init(&conn, screen_num, overlay);
    backend.draw();

    loop {
        match conn.wait_for_event() {
            None => break,
            Some(e) => event::handle_event(&conn, &e, &mut windows),
        }
    }

    backend.destroy();

    Ok(())
}
