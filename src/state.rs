use std::process::exit;

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
            exit(1);
        });

        let overlay = init::extensions::redirect_subwindows(&conn)
            .unwrap_or_else(|err| {
                eprintln!("Failed redirecting subwindows: {}", err);
                exit(1);
            });

        let win_id = init::window::create_window(&conn);

        Ok(State { conn, xlib_screens, xcb_screens, root, overlay, win_id })
    }
    pub fn update_root_pixmap(&mut self) {
        self.root.pixmap = match get_root_pixmap(&self.conn, &self.root) {
            Ok(pixmap) => pixmap,
            // TODO: create new 1x1 pixmap
            Err(message) => {
                eprintln!("{}", message);
                xcb::NONE
            }
        };
    }
}

fn get_root_pixmap(
    conn: &xcb::Connection,
    root: &Window,
) -> Result<xcb::Pixmap, &'static str> {
    let root_atoms = [
        xcb::intern_atom(conn, false, &"ESETROOT_PMAP_ID")
            .get_reply()
            .unwrap()
            .atom(),
        xcb::intern_atom(conn, false, &"_XROOTPMAP_ID")
            .get_reply()
            .unwrap()
            .atom(),
        xcb::intern_atom(conn, false, &"_XSETROOT_ID")
            .get_reply()
            .unwrap()
            .atom(),
    ];
    for atom in root_atoms.iter() {
        if let Ok(result) = xcb::get_property(
            conn,
            false,
            root.id,
            *atom,
            xcb::ATOM_PIXMAP,
            0,
            4,
        )
        .get_reply()
        {
            if result.type_() == xcb::ATOM_PIXMAP
                && result.format() == 32
                && result.value_len() == 1
            {
                return Ok(result.value::<u32>()[0] as xcb::Pixmap);
            }
        }
    }
    Err("unable to get root pixmap")
}
