use crate::window::Window;

pub fn handle_event(
    conn: &xcb::Connection,
    event: &xcb::GenericEvent,
    windows: &mut Vec<Window>,
) {
    match event.response_type() & !0x80 {
        // New window created
        xcb::CREATE_NOTIFY => {
            let ev: &xcb::CreateNotifyEvent =
                unsafe { xcb::cast_event(&event) };
            let win = ev.window();
            if Window::is_mapped(conn, win) {
                windows.push(Window::new(conn, win));
            }
        }
        // Window destroyed
        // For any window, an event for every child is sent out first
        xcb::DESTROY_NOTIFY => {
            let ev: &xcb::DestroyNotifyEvent =
                unsafe { xcb::cast_event(&event) };
            let win = ev.window();
            if Window::is_mapped(conn, win) {
                windows.retain(|w| w.id == win);
            }
        }
        // Window property(size, border, position, stack order) changed
        xcb::CONFIGURE_NOTIFY => {
            let ev: &xcb::ConfigureNotifyEvent =
                unsafe { xcb::cast_event(&event) };
            let win = ev.window();
            if Window::is_mapped(conn, win) {
                let index = windows.iter().position(|w| w.id == win).unwrap();
                // TODO: utilize event methods instead of `xcb::get_geometry`
                windows[index].update(conn);
            }
        }
        // Existing window mapped
        xcb::MAP_NOTIFY => {
            let ev: &xcb::MapNotifyEvent = unsafe { xcb::cast_event(&event) };
            let win = ev.window();
            // probably no need to check if window is mapped
            windows.push(Window::new(conn, win));
        }
        // Existing window unmapped
        xcb::UNMAP_NOTIFY => {
            let ev: &xcb::UnmapNotifyEvent =
                unsafe { xcb::cast_event(&event) };
            let win = ev.window();
            // probably no need to check if window is mapped
            windows.retain(|w| w.id == win);
        }
        // Window's parent changed
        // TODO
        xcb::REPARENT_NOTIFY => {
            let ev: &xcb::ReparentNotifyEvent =
                unsafe { xcb::cast_event(&event) };
            let win = ev.window();
            if Window::is_mapped(conn, win) {
                let index = windows.iter().position(|w| w.id == win).unwrap();
            }
        }
        // Window's stack position changed
        xcb::CIRCULATE_NOTIFY => {
            let ev: &xcb::CirculateNotifyEvent =
                unsafe { xcb::cast_event(&event) };
            let win = ev.window();
            if Window::is_mapped(conn, win) {
                let index = windows.iter().position(|w| w.id == win).unwrap();
            }
        }
        // TODO
        // ...Window contents updated?
        xcb::EXPOSE => {
            let ev: &xcb::ExposeEvent = unsafe { xcb::cast_event(&event) };
            let win = ev.window();
            // count specifies the number of remaining Expose events which
            // follow for this window
            // To optimize redraws, we only update on the last Expose event
            if Window::is_mapped(conn, win) && ev.count() == 0 {}
        }
        // Window property changed
        // TODO
        xcb::PROPERTY_NOTIFY => {
            let ev: &xcb::PropertyNotifyEvent =
                unsafe { xcb::cast_event(&event) };
        }
        _ => {}
    }
}
