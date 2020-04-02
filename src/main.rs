extern crate xcb;

mod init;
mod window;

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
    let windows = Window::fetch_windows(&conn);
    init::window::request_events(&conn);

    loop {
        let event = conn.wait_for_event();
        match event {
            None => break,
            Some(event) => match event.response_type() & !0x80 {
                xcb::CREATE_NOTIFY => {
                    let ev: &xcb::CreateNotifyEvent =
                        unsafe { xcb::cast_event(&event) };
                }
                xcb::DESTROY_NOTIFY => {
                    let ev: &xcb::DestroyNotifyEvent =
                        unsafe { xcb::cast_event(&event) };
                }
                xcb::CONFIGURE_NOTIFY => {
                    let ev: &xcb::ConfigureNotifyEvent =
                        unsafe { xcb::cast_event(&event) };
                }
                xcb::MAP_NOTIFY => {
                    let ev: &xcb::MapNotifyEvent =
                        unsafe { xcb::cast_event(&event) };
                }
                xcb::UNMAP_NOTIFY => {
                    let ev: &xcb::UnmapNotifyEvent =
                        unsafe { xcb::cast_event(&event) };
                }
                xcb::REPARENT_NOTIFY => {
                    let ev: &xcb::ReparentNotifyEvent =
                        unsafe { xcb::cast_event(&event) };
                }
                xcb::CIRCULATE_NOTIFY => {
                    let ev: &xcb::CirculateNotifyEvent =
                        unsafe { xcb::cast_event(&event) };
                }
                xcb::EXPOSE => {
                    let ev: &xcb::ExposeEvent =
                        unsafe { xcb::cast_event(&event) };
                }
                xcb::PROPERTY_NOTIFY => {
                    let ev: &xcb::PropertyNotifyEvent =
                        unsafe { xcb::cast_event(&event) };
                }
                _ => {}
            },
        }
    }

    Ok(())
}
