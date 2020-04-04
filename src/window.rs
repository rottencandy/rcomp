pub struct Window {
    pub id: xcb::Window,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

impl Window {
    /// Returns a vector of mapped `Windows` recieved from `XQueryTree`
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

    /// Checks wether a window is mapped
    pub fn is_mapped(conn: &xcb::Connection, win: xcb::Window) -> bool {
        let attrs = xcb::get_window_attributes(conn, win).get_reply().unwrap();
        attrs.map_state() == xcb::MAP_STATE_VIEWABLE as u8
    }

    /// Creates a new `Window`
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

    /// Update the properties of an existing `Window`
    pub fn update(&mut self, conn: &xcb::Connection) {
        let geometry = xcb::get_geometry(conn, self.id).get_reply().unwrap();
        self.x = geometry.x();
        self.y = geometry.y();
        self.width = geometry.width();
        self.height = geometry.height();
    }
}
