use crate::opengl::Opengl;
use crate::window::Window;

pub fn handle_event(
    conn: &xcb::Connection,
    event: &xcb::GenericEvent,
    windows: &mut Vec<Window>,
    _backend: &Opengl,
) {
    match event.response_type() & !0x80 {
        // New window created
        xcb::CREATE_NOTIFY => {
            let ev: &xcb::CreateNotifyEvent =
                unsafe { xcb::cast_event(&event) };
            let win = ev.window();
            windows.push(Window::new(conn, win));
        }
        // Window destroyed
        // For any window, an event for every child is sent out first
        xcb::DESTROY_NOTIFY => {
            let ev: &xcb::DestroyNotifyEvent =
                unsafe { xcb::cast_event(&event) };
            windows.retain(|w| w.id == ev.window());
        }
        // Window property(size, border, position, stack order) changed
        xcb::CONFIGURE_NOTIFY => {
            let ev: &xcb::ConfigureNotifyEvent =
                unsafe { xcb::cast_event(&event) };
            let win = ev.window();
            if let Some(index) = windows.iter().position(|w| w.id == win) {
                // TODO: utilize event methods instead of `xcb::get_geometry`
                // TODO: Check if root window is updated
                // TODO: restack
                windows[index].update(conn);
            }
        }
        // Existing window mapped
        xcb::MAP_NOTIFY => {
            let ev: &xcb::MapNotifyEvent = unsafe { xcb::cast_event(&event) };
            let win = ev.window();
            if let Some(index) = windows.iter().position(|w| w.id == win) {
                windows[index].mapped = true;
            }
        }
        // Existing window unmapped
        xcb::UNMAP_NOTIFY => {
            let ev: &xcb::UnmapNotifyEvent =
                unsafe { xcb::cast_event(&event) };
            let win = ev.window();
            if let Some(index) = windows.iter().position(|w| w.id == win) {
                windows[index].mapped = false;
            }
        }
        // Window's parent changed
        // TODO
        xcb::REPARENT_NOTIFY => {
            let ev: &xcb::ReparentNotifyEvent =
                unsafe { xcb::cast_event(&event) };
            let win = ev.window();
            if let Some(true) = Window::is_mapped(conn, win) {
                let _index = windows.iter().position(|w| w.id == win).unwrap();
            }
        }
        // Window's stack position changed
        xcb::CIRCULATE_NOTIFY => {
            let ev: &xcb::CirculateNotifyEvent =
                unsafe { xcb::cast_event(&event) };
            let win = ev.window();
            if let Some(true) = Window::is_mapped(conn, win) {
                let _index = windows.iter().position(|w| w.id == win).unwrap();
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
            if let Some(true) = Window::is_mapped(conn, win) {
                if ev.count() == 0 {}
            }
        }
        // Window property changed
        // TODO
        xcb::PROPERTY_NOTIFY => {
            let _ev: &xcb::PropertyNotifyEvent =
                unsafe { xcb::cast_event(&event) };
        }
        _ => {}
    }
}
