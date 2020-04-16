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
                Err(e) => println!("Error getting window info: {}", e),
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

    pub fn get_opacity(&self, conn: &xcb::Connection) {
        let atom = xcb::intern_atom(conn, false, &"_NET_WM_WINDOW_OPACITY")
            .get_reply()
            .unwrap()
            .atom();
        match xcb::get_property(
            conn,
            false,
            self.id,
            atom,
            xcb::ATOM_CARDINAL,
            0,
            1,
        )
        .get_reply()
        {
            Ok(data) => {
                println!(
                    "type: {}, format: {}, len: {}, after: {}",
                    data.type_(),
                    data.format(),
                    data.value_len(),
                    data.bytes_after()
                );
                if data.type_() == xcb::ATOM_CARDINAL
                    && data.format() == 32
                    && data.value_len() == 1
                {
                    //return data.value<f32>()[0]
                    let val: u32 = data.value()[0];
                    println!("Got! {}", val);
                }
            }
            _ => {
                println!("No data");
            }
        };
    }
}
