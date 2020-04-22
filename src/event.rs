use crate::opengl::Opengl;
use crate::window::Window;

pub fn handle_event(
    conn: &xcb::Connection,
    baseEvent: &xcb::GenericEvent,
    windows: &mut Vec<Window>,
    backend: &Opengl,
) {
    match baseEvent.response_type() & !0x80 {
        // New window created
        xcb::CREATE_NOTIFY => {
            let ev: &xcb::CreateNotifyEvent =
                unsafe { xcb::cast_event(&baseEvent) };
            match Window::new(conn, ev.window()) {
                Ok(win) => {
                    windows.push(win);
                    for win in windows.iter_mut().filter(|w| w.mapped) {
                        backend.update_glxpixmap(win);
                        backend.update_window_texture(win);
                        backend.draw_window(win);
                    }
                    backend.render();
                }
                Err(e) => println!("Cannot get created window info: {}", e),
            };
        }
        // Window destroyed
        // For any window, an event for every child is sent out first
        xcb::DESTROY_NOTIFY => {
            let ev: &xcb::DestroyNotifyEvent =
                unsafe { xcb::cast_event(&baseEvent) };
            windows.retain(|w| w.id != ev.window());
            for win in windows.iter_mut().filter(|w| w.mapped) {
                backend.update_glxpixmap(win);
                backend.update_window_texture(win);
                backend.draw_window(win);
            }
            backend.render();
        }
        // Window property(size, border, position, stack order) changed
        xcb::CONFIGURE_NOTIFY => {
            let ev: &xcb::ConfigureNotifyEvent =
                unsafe { xcb::cast_event(&baseEvent) };
            let win_id = ev.window();
            if let Some(index) = windows.iter().position(|w| w.id == win_id) {
                // TODO: Check if this is root window and is updated
                // TODO: restack?
                let w = &mut windows[index];
                w.update_using_event(ev);
                // New pixmap is generated for every resize
                // TODO: also check border_width, override_redirect
                if w.mapped && ev.width() != w.width || ev.height() != w.height
                {
                    backend.update_glxpixmap(w);
                }
                for win in windows.iter_mut().filter(|w| w.mapped) {
                    backend.update_window_texture(win);
                    backend.draw_window(win);
                }
                backend.render();
            } else {
                println!("No window found: {}", win_id);
            }
        }
        // Existing window mapped
        xcb::MAP_NOTIFY => {
            let ev: &xcb::MapNotifyEvent =
                unsafe { xcb::cast_event(&baseEvent) };
            let win_id = ev.window();
            if let Some(index) = windows.iter().position(|w| w.id == win_id) {
                let w = &mut windows[index];
                w.mapped = true;
                // New pixmap is generated for every map
                backend.update_glxpixmap(w);
                for win in windows.iter_mut().filter(|w| w.mapped) {
                    backend.update_window_texture(win);
                    backend.draw_window(win);
                }
                backend.render();
            } else {
                println!("No window found: {}", win_id);
            }
        }
        // Existing window unmapped
        xcb::UNMAP_NOTIFY => {
            let ev: &xcb::UnmapNotifyEvent =
                unsafe { xcb::cast_event(&baseEvent) };
            let win_id = ev.window();
            if let Some(index) = windows.iter().position(|w| w.id == win_id) {
                windows[index].mapped = false;
                for win in windows.iter_mut().filter(|w| w.mapped) {
                    backend.update_window_texture(win);
                    backend.draw_window(win);
                }
                backend.render();
            } else {
                println!("No window found: {}", win_id);
            }
        }
        // Window's parent changed
        xcb::REPARENT_NOTIFY => {
            let ev: &xcb::ReparentNotifyEvent =
                unsafe { xcb::cast_event(&baseEvent) };
            let win_id = ev.window();
            // TODO:
            // Check that the parent is root
            // Remove from list if not
            if let Some(index) = windows.iter().position(|w| w.id == win_id) {
                //windows[index].mapped = false;
            } else {
                // TODO: Add this window to the list
            }
        }
        // Window's stack position changed
        xcb::CIRCULATE_NOTIFY => {
            let ev: &xcb::CirculateNotifyEvent =
                unsafe { xcb::cast_event(&baseEvent) };
            let win_id = ev.window();
            if let Some(index) = windows.iter().position(|w| w.id == win_id) {
                let win = windows.remove(index);
                // Window is placed below all its siblings
                if ev.place() == xcb::PLACE_ON_BOTTOM as u8 {
                    windows.push(win);
                } else {
                    windows.insert(0, win);
                }
                for win in windows.iter_mut().filter(|w| w.mapped) {
                    backend.update_window_texture(win);
                    backend.draw_window(win);
                }
                backend.render();
            } else {
                println!("No window found: {}", win_id);
            }
        }
        // Window unhidden
        xcb::EXPOSE => {
            let ev: &xcb::ExposeEvent = unsafe { xcb::cast_event(&baseEvent) };
            let win_id = ev.window();
            // count specifies the number of remaining Expose events which
            // follow for this window
            // To optimize redraws, we only update on the last Expose event
            if ev.count() != 0 {
                return;
            }
            for win in windows.iter_mut().filter(|w| w.mapped) {
                backend.update_glxpixmap(win);
                backend.update_window_texture(win);
                backend.draw_window(win);
            }
            backend.render();
            // TODO: check if window is root
        }
        // Window property(atom) changed
        // TODO
        xcb::PROPERTY_NOTIFY => {
            let _ev: &xcb::PropertyNotifyEvent =
                unsafe { xcb::cast_event(&baseEvent) };
        }
        // TODO: check for root property changes
        _ => {
            // Window damage detected
            println!("Got unknown event: {}", baseEvent.response_type());
        }
    }
}
