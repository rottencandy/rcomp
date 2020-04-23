use crate::opengl::Opengl;
use crate::window::Window;
use xcb::damage;

pub fn handle_event(
    conn: &xcb::Connection,
    base_event: &xcb::GenericEvent,
    windows: &mut Vec<Window>,
    backend: &Opengl,
    root_win: &Window,
) {
    match base_event.response_type() & !0x80 {
        // New window created
        xcb::CREATE_NOTIFY => {
            let ev: &xcb::CreateNotifyEvent =
                unsafe { xcb::cast_event(&base_event) };
            match Window::new(conn, ev.window()) {
                Ok(mut win) => {
                    if win.mapped {
                        backend.update_glxpixmap(&mut win);
                        backend.update_window_texture(&mut win);
                    }
                    windows.push(win);
                }
                Err(e) => println!("Cannot get created window info: {}", e),
            };
            for win in windows.iter_mut().filter(|w| w.mapped) {
                backend.draw_window(win);
            }
            backend.render();
        }
        // Window destroyed
        // For any window, an event for every child is sent out first
        xcb::DESTROY_NOTIFY => {
            let ev: &xcb::DestroyNotifyEvent =
                unsafe { xcb::cast_event(&base_event) };
            windows.retain(|w| w.id != ev.window());

            for win in windows.iter_mut().filter(|w| w.mapped) {
                backend.draw_window(win);
            }
            backend.render();
        }
        // Window property(size, border, position, stack order) changed
        // TODO: restack
        xcb::CONFIGURE_NOTIFY => {
            let ev: &xcb::ConfigureNotifyEvent =
                unsafe { xcb::cast_event(&base_event) };
            let win_id = ev.window();
            if let Some(index) = windows.iter().position(|w| w.id == win_id) {
                let w = &mut windows[index];
                // New pixmap is generated for every resize
                if w.mapped && ev.width() != w.width
                    || ev.height() != w.height
                    || ev.override_redirect() != w.override_redirect
                    || ev.border_width() != w.border_width
                {
                    backend.update_glxpixmap(w);
                    backend.update_window_texture(w);
                }
                w.update_using_event(ev);
            } else if win_id == root_win.id {
                //backend.update_glxpixmap(win_id);
                //backend.update_window_texture(win_id);
            } else {
                println!("ConfigureEvent: No window in list: {}", win_id);
            }
            for win in windows.iter_mut().filter(|w| w.mapped) {
                backend.draw_window(win);
            }
            backend.render();
        }
        // Existing window mapped
        xcb::MAP_NOTIFY => {
            let ev: &xcb::MapNotifyEvent =
                unsafe { xcb::cast_event(&base_event) };
            let win_id = ev.window();
            if let Some(index) = windows.iter().position(|w| w.id == win_id) {
                let w = &mut windows[index];
                w.mapped = true;
                // New pixmap is generated for every map
                backend.update_glxpixmap(w);
                backend.update_window_texture(w);
                for win in windows.iter_mut().filter(|w| w.mapped) {
                    backend.draw_window(win);
                }
                backend.render();
            } else {
                println!("MapNotify: No window in list: {}", win_id);
            }
        }
        // Existing window unmapped
        xcb::UNMAP_NOTIFY => {
            let ev: &xcb::UnmapNotifyEvent =
                unsafe { xcb::cast_event(&base_event) };
            let win_id = ev.window();
            if let Some(index) = windows.iter().position(|w| w.id == win_id) {
                windows[index].mapped = false;
                for win in windows.iter_mut().filter(|w| w.mapped) {
                    backend.draw_window(win);
                }
                backend.render();
            } else {
                println!("UnmapNotify: No window in list: {}", win_id);
            }
        }
        // Window's parent changed
        xcb::REPARENT_NOTIFY => {
            let event: &xcb::ReparentNotifyEvent =
                unsafe { xcb::cast_event(&base_event) };
            let win_id = event.window();
            if event.parent() == root_win.id {
                if let None = windows.iter().position(|w| w.id == win_id) {
                } else {
                    match Window::new(conn, win_id) {
                        Ok(mut win) => {
                            if win.mapped {
                                backend.update_glxpixmap(&mut win);
                                backend.update_window_texture(&mut win);
                            }
                            windows.push(win);
                        }
                        Err(e) => {
                            println!("Cannot get created window info: {}", e)
                        }
                    };
                }
            } else {
                windows.retain(|w| w.id != win_id);
            }
        }
        // Window's stack position changed
        xcb::CIRCULATE_NOTIFY => {
            let ev: &xcb::CirculateNotifyEvent =
                unsafe { xcb::cast_event(&base_event) };
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
                    backend.draw_window(win);
                }
                backend.render();
            } else {
                println!("CirculateNotify: No window in list: {}", win_id);
            }
        }
        // Window unhidden
        xcb::EXPOSE => {
            let ev: &xcb::ExposeEvent =
                unsafe { xcb::cast_event(&base_event) };
            // TODO: check if window is root
            //let win_id = ev.window();

            // count specifies the number of remaining Expose events which
            // follow for this window
            // To optimize redraws, we only update on the last Expose event
            if ev.count() != 0 {
                return;
            }
            for win in windows.iter_mut().filter(|w| w.mapped) {
                backend.draw_window(win);
            }
            backend.render();
        }
        // Window property(atom) changed
        // TODO
        xcb::PROPERTY_NOTIFY => {
            let _ev: &xcb::PropertyNotifyEvent =
                unsafe { xcb::cast_event(&base_event) };
        }
        // TODO: check for root property changes
        _ => {
            // Window damage detected
            if base_event.response_type() == damage::NOTIFY {
                let event: &damage::NotifyEvent =
                    unsafe { xcb::cast_event(&base_event) };
                let win_id = event.drawable();
                println!(
                    "DamageNotify: Got event: {}, {}",
                    base_event.response_type(),
                    win_id
                );
                if let Some(index) =
                    windows.iter().position(|w| w.id == win_id)
                {
                    backend.update_window_texture(&mut windows[index]);
                } else {
                    println!("DamageNotify: no window in list: {}", win_id);
                }
            }
        }
    }
}
