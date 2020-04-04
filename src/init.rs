pub mod window {
    use std::process;

    /// Creates a dummy window, used to get ownership of atoms,
    /// returns the window ID.
    pub fn create_window(
        conn: &xcb::Connection,
        screen_num: i32,
    ) -> xcb::Window {
        let setup = conn.get_setup();
        let win = conn.generate_id();
        let screen = setup.roots().last().unwrap();

        xcb::create_window(
            conn,
            xcb::COPY_FROM_PARENT as u8,
            win,
            screen.root(),
            0,
            0,
            1,
            1,
            0,
            // INPUT_ONLY does not seem to be able to grab atom ownership
            xcb::WINDOW_CLASS_INPUT_OUTPUT as u16,
            xcb::COPY_FROM_PARENT,
            &[],
        );
        // No need to map the window, since we don't need to display anything
        conn.flush();
        grab_atoms(conn, win, screen_num);
        win
    }

    /// Try and get the ownership of _NET_WM_CM_Sn atoms, one for each screen.
    fn grab_atoms(conn: &xcb::Connection, win: xcb::Window, screens: i32) {
        for screen in 0..screens {
            let atom = xcb::intern_atom(
                conn,
                false,
                &format!("_NET_WM_CM_S{}", screen),
            )
            .get_reply()
            .unwrap()
            .atom();

            if xcb::get_selection_owner(conn, atom)
                .get_reply()
                .unwrap()
                .owner()
                != xcb::ATOM_NONE
            {
                eprintln!("Another compositor is already running");
                process::exit(1);
            }

            xcb::set_selection_owner(conn, win, atom, xcb::CURRENT_TIME);
            // TODO: Check if ownership was successfully grabbed
            conn.flush();
        }
    }

    /// Requests for relevant window change & update events
    pub fn request_events(conn: &xcb::Connection) {
        let setup = conn.get_setup();
        let screen = setup.roots().last().unwrap();
        xcb::change_window_attributes(
            conn,
            screen.root(),
            &[(
                xcb::CW_EVENT_MASK,
                xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY
                    | xcb::EVENT_MASK_EXPOSURE
                    | xcb::EVENT_MASK_STRUCTURE_NOTIFY
                    | xcb::EVENT_MASK_PROPERTY_CHANGE,
            )],
        );
        conn.flush();
    }
}

pub mod extensions {
    use xcb::{composite, randr};

    /// Checks that the required extensions are present in the server.
    // Use hashmap with loop?
    // TODO: Check extension versions, aside from existence
    pub fn verify(conn: &xcb::Connection) -> Result<(), &str> {
        if !conn.get_extension_data(composite::id()).unwrap().present() {
            return Err("composite");
        }
        if !conn.get_extension_data(randr::id()).unwrap().present() {
            return Err("randr");
        }
        Ok(())
    }

    /// Uses the composite extension to request redirection of all windows
    /// to offscreen pixmaps.
    pub fn redirect_subwindows(
        conn: &xcb::Connection,
    ) -> Result<(), xcb::GenericError> {
        let setup = conn.get_setup();

        // Prevent unexpected changes to window tree while we redirect
        xcb::grab_server(&conn);
        for screen in setup.roots() {
            composite::redirect_subwindows(
                conn,
                screen.root(),
                composite::REDIRECT_MANUAL as u8,
            )
            .request_check()?;
        }
        xcb::ungrab_server(&conn);
        Ok(())
    }
}