pub struct Window {
    id: xcb::Window,
    x: i16,
    y: i16,
    width: u16,
    height: u16,
}

impl Window {
    pub fn fetch_windows(conn: &xcb::Connection) -> Vec<Window> {
        let setup = conn.get_setup();

        let mut windows = Vec::new();
        for screen in setup.roots() {
            let tree =
                xcb::query_tree(conn, screen.root()).get_reply().unwrap();

            for win in tree.children().iter() {
                let attrs = xcb::get_window_attributes(conn, *win)
                    .get_reply()
                    .unwrap();
                // We only care about mapped windows
                if attrs.map_state() == xcb::MAP_STATE_VIEWABLE as u8 {
                    windows.push(Window::new(conn, *win));
                    //let pix = conn.generate_id();
                    //composite::name_window_pixmap(conn, *win, pix)
                    //    .request_check()
                    //    .unwrap();
                }
            }
        }
        windows
    }

    pub fn new(conn: &xcb::Connection, win: xcb::Window) -> Window {
        let geometry = xcb::get_geometry(conn, win).get_reply().unwrap();
        Window {
            id: win,
            x: geometry.x(),
            y: geometry.y(),
            width: geometry.width(),
            height: geometry.height(),
        }
    }
}
