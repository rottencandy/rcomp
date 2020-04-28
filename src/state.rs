use std::process;

use crate::init;
use crate::Window;

pub struct State {
    /// The X11 connection
    pub conn: xcb::Connection,
    /// Number of screens(xlib)
    /// NOTE: This screen_num is not the same as xcb's
    /// setup.roots_len() This one only works for xlib's OpenGL ctx
    /// calls (which strangely do not work with xcb's screen_num)
    ///
    /// Use `xcb_screens` for all other xcb-only calls
    /// (or they behave unexpectedly)
    pub xlib_screens: i32,
    /// Number of screens(xcb)
    pub xcb_screens: u8,
    /// The root window object
    pub root: Window,
    /// XComposite overlay window
    pub overlay: xcb::Window,
    /// Window id of the compositor
    pub win_id: xcb::Window,
}

impl State {
    pub fn init() -> Result<State, xcb::ConnError> {
        let (conn, xlib_screens) =
            xcb::Connection::connect_with_xlib_display()?;
        conn.set_event_queue_owner(xcb::EventQueueOwner::Xcb);
        let setup = conn.get_setup();
        let xcb_screens = setup.roots_len();
        let root =
            Window::new(&conn, setup.roots().last().unwrap().root()).unwrap();

        init::extensions::verify(&conn).unwrap_or_else(|err| {
            eprintln!("Error: extension `{}` not found.", err);
            process::exit(1);
        });

        let overlay = init::extensions::redirect_subwindows(&conn)
            .unwrap_or_else(|err| {
                eprintln!("Failed redirecting subwindows: {}", err);
                process::exit(1);
            });

        let win_id = init::window::create_window(&conn);

        Ok(State { conn, xlib_screens, xcb_screens, root, overlay, win_id })
    }
}
