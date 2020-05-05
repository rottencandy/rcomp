use crate::opengl::BackendContext;
use xcb::{composite, damage, render, shape};

pub struct Window {
    pub id: xcb::Window,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
    pub border_width: u16,
    pub mapped: bool,
    pub override_redirect: bool,
    pub alpha: bool,
    pub pixmap: xcb::Pixmap,
    pub context: BackendContext,
    pub damage: damage::Damage,
}

impl Window {
    /// Returns a vector of mapped `Windows` recieved from `XQueryTree`
    pub fn fetch_windows(conn: &xcb::Connection) -> Vec<Window> {
        let setup = conn.get_setup();
        let screen = setup.roots().last().unwrap();
        let tree = xcb::query_tree(conn, screen.root()).get_reply().unwrap();
        let mut windows: Vec<Window> =
            Vec::with_capacity(tree.children_len() as usize);
        for win in tree.children().iter().rev() {
            match Window::new(conn, *win) {
                Ok(w) => windows.push(w),
                Err(_) => println!("Unable to get info for win: {}", win),
            };
        }
        windows
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
            alpha: has_alpha(conn, attrs.colormap()),
            pixmap: conn.generate_id(),
            context: Default::default(),
            damage: 0,
        })
    }

    /// Update the geometry properties of an existing `Window`
    pub fn update_geometry(&mut self, conn: &xcb::Connection) {
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

    // TODO: check the requests
    pub fn update_pixmap(
        &mut self,
        conn: &xcb::Connection,
    ) -> Result<(), xcb::GenericError> {
        self.pixmap = conn.generate_id();
        composite::name_window_pixmap(conn, self.id, self.pixmap)
            .request_check()?;

        // Recreate damage region tracker
        damage::destroy(conn, self.damage);
        self.damage = conn.generate_id();
        damage::create(
            conn,
            self.damage,
            self.id,
            damage::REPORT_LEVEL_NON_EMPTY as u8,
        )
        .request_check()?;

        // Request shape events
        // TODO: this only needs to be done once per window
        shape::select_input(conn, self.id, true).request_check()?;
        Ok(())
    }
}

// TODO: cacke pict_format iterator
fn has_alpha(conn: &xcb::Connection, colormap: xcb::Colormap) -> bool {
    for format in
        render::query_pict_formats(conn).get_reply().unwrap().formats()
    {
        if format.colormap() == colormap {
            return format.type_() == render::PICT_TYPE_DIRECT as u8
                && format.direct().alpha_mask() != 0;
        }
    }
    false
}
