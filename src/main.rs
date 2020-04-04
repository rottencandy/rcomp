extern crate xcb;

mod init;
mod window;
mod event;

use std::error::Error;
use std::process;
use window::Window;

fn main() -> Result<(), Box<dyn Error>> {
    let (conn, screen_num) =
        xcb::Connection::connect(None).unwrap_or_else(|err| {
            eprintln!("Error opening connection to X server: {}", err);
            process::exit(1);
        });

    init::extensions::verify(&conn).unwrap_or_else(|err| {
        eprintln!("Error: extension `{}` not found.", err);
        process::exit(1);
    });

    let win = init::window::create_window(&conn, screen_num);

    //let scr_info =
    //    xcb::randr::get_screen_info(&conn, win).get_reply().unwrap();
    //println!("Screen rate: {}", scr_info.rate());

    init::extensions::redirect_subwindows(&conn).unwrap_or_else(|err| {
        eprintln!("Failed redirecting subwindows: {}", err);
        process::exit(1);
    });
    // TODO use linked list
    let mut windows = Window::fetch_windows(&conn);
    init::window::request_events(&conn);

    loop {
        match conn.wait_for_event() {
            None => break,
            Some(e) => event::handle_event(&conn, &e, &mut windows),
        }
    }

    Ok(())
}
