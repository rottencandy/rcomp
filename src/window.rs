use xcb::composite;

pub struct Window {
    pub id: xcb::Window,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
    pub border_width: u16,
    pub mapped: bool,
    pub override_redirect: bool,
    pub pixmap: xcb::Pixmap,
}

impl Window {
    /// Returns a vector of mapped `Windows` recieved from `XQueryTree`
    pub fn fetch_windows(conn: &xcb::Connection) -> Vec<Window> {
        let setup = conn.get_setup();
        let screen = setup.roots().last().unwrap();
        let tree = xcb::query_tree(conn, screen.root()).get_reply().unwrap();
        let mut windows: Vec<Window> =
            Vec::with_capacity(tree.children_len() as usize);
        for win in tree.children() {
            match Window::new(conn, *win) {
                Ok(w) => windows.push(w),
                Err(_) => println!("Unable to get info for win: {}", win),
            };
        }
        windows
    }

    /// Checks wether a window is mapped
    pub fn is_mapped(
        conn: &xcb::Connection,
        win: xcb::Window,
    ) -> Option<bool> {
        if let Ok(attr) = xcb::get_window_attributes(conn, win).get_reply() {
            return Some(attr.map_state() == xcb::MAP_STATE_VIEWABLE as u8);
        }
        None
    }

    /// Creates a new `Window`
    pub fn new(
        conn: &xcb::Connection,
        win: xcb::Window,
    ) -> Result<Window, xcb::GenericError> {
        let geometry = xcb::get_geometry(conn, win).get_reply()?;
        let attrs = xcb::get_window_attributes(conn, win).get_reply()?;
        Ok(Window {
            id: win,
            x: geometry.x(),
            y: geometry.y(),
            width: geometry.width(),
            border_width: geometry.border_width(),
            height: geometry.height(),
            mapped: attrs.map_state() == xcb::MAP_STATE_VIEWABLE as u8,
            override_redirect: attrs.override_redirect(),
            pixmap: conn.generate_id(),
        })
    }

    /// Update the properties of an existing `Window`
    pub fn update(&mut self, conn: &xcb::Connection) {
        let geometry = xcb::get_geometry(conn, self.id).get_reply().unwrap();
        self.x = geometry.x();
        self.y = geometry.y();
        self.width = geometry.width();
        self.height = geometry.height();
        self.border_width = geometry.border_width();
    }

    /// Update a window's properties using a ConfigureNotifyEvent
    /// Similar to `Window::update()`, but faster due to not having to
    /// use `xcb::get_geometry()`
    pub fn update_using_event(&mut self, event: &xcb::ConfigureNotifyEvent) {
        self.x = event.x();
        self.y = event.y();
        self.width = event.width();
        self.height = event.height();
        self.border_width = event.border_width();
        self.override_redirect = event.override_redirect();
    }

    pub fn update_pixmap(&mut self, conn: &xcb::Connection) {
        self.pixmap = conn.generate_id();
        composite::name_window_pixmap(conn, self.id, self.pixmap);
        conn.flush();
    }
}
