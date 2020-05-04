use crate::opengl::Opengl;
use crate::state::State;
use crate::window::Window;
use xcb::damage;

use std::time::{Duration, Instant};

pub fn handle_event(
    state: &State,
    base_event: xcb::GenericEvent,
    windows: &mut Vec<Window>,
    backend: &Opengl,
    last_render: &mut Instant,
    refresh_rate: &Duration,
) {
    match base_event.response_type() {
        // New window created
        xcb::CREATE_NOTIFY => {
            println!("CREATE_NOTIFY");
            let ev: &xcb::CreateNotifyEvent =
                unsafe { xcb::cast_event(&base_event) };
            match Window::new(&state.conn, ev.window()) {
                Ok(mut win) => {
                    if win.mapped {
                        backend.init_window(&mut win);
                        backend.update_pos(&win);
                        backend.update_pixmap(&mut win);
                        backend.update_texture(&mut win);
                    }
                    windows.push(win);
                }
                Err(e) => {
                    println!("CreateNotify: cannot get window info: {}", e)
                }
            };
            for win in windows.iter_mut().filter(|w| w.mapped) {
                backend.draw_window(win);
            }
            backend.render();
        }
        // Window destroyed
        // For any window, an event for every child is sent out first
        xcb::DESTROY_NOTIFY => {
            println!("DESTROY_NOTIFY");
            let ev: &xcb::DestroyNotifyEvent =
                unsafe { xcb::cast_event(&base_event) };
            windows.retain(|w| w.id != ev.window());

            for win in windows.iter_mut().filter(|w| w.mapped) {
                backend.draw_window(win);
            }
            backend.render();
        }
        // Window property(size, border, position, stack order) changed
        // TODO: check if window is root
        xcb::CONFIGURE_NOTIFY => {
            println!("CONFIGURE_NOTIFY");
            let ev: &xcb::ConfigureNotifyEvent =
                unsafe { xcb::cast_event(&base_event) };
            let win_id = ev.window();
            if let Some(i) = windows.iter().position(|w| w.id == win_id) {
                let w = &mut windows[i];
                // New pixmap is generated for every resize
                if w.mapped && ev.width() != w.width
                    || ev.height() != w.height
                    || ev.override_redirect() != w.override_redirect
                    || ev.border_width() != w.border_width
                {
                    backend.update_pixmap(w);
                    backend.update_texture(w);
                }
                w.update_using_event(ev);
                backend.update_pos(w);
                restack_window(win_id, ev.above_sibling(), windows);
            } else if win_id == state.root.id {
                //backend.update_pixmap(win_id);
                //backend.update_texture(win_id);
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
            println!("MAP_NOTIFY");
            let ev: &xcb::MapNotifyEvent =
                unsafe { xcb::cast_event(&base_event) };
            if let Some(i) = windows.iter().position(|w| w.id == ev.window()) {
                let w = &mut windows[i];
                w.mapped = true;
                backend.init_window(w);
                // New pixmap is generated for every map
                backend.update_pos(w);
                backend.update_pixmap(w);
                backend.update_texture(w);
                for win in windows.iter_mut().filter(|w| w.mapped) {
                    backend.draw_window(win);
                }
                backend.render();
            }
        }
        // Existing window unmapped
        xcb::UNMAP_NOTIFY => {
            println!("UNMAP_NOTIFY");
            let ev: &xcb::UnmapNotifyEvent =
                unsafe { xcb::cast_event(&base_event) };
            if let Some(i) = windows.iter().position(|w| w.id == ev.window()) {
                windows[i].mapped = false;
                for win in windows.iter_mut().filter(|w| w.mapped) {
                    backend.draw_window(win);
                }
                backend.render();
            }
        }
        // Window's parent changed
        xcb::REPARENT_NOTIFY => {
            println!("REPARENT_NOTIFY");
            let event: &xcb::ReparentNotifyEvent =
                unsafe { xcb::cast_event(&base_event) };
            let win_id = event.window();
            if event.parent() == state.root.id {
                if windows.iter().position(|w| w.id == win_id).is_none() {
                    match Window::new(&state.conn, win_id) {
                        Ok(mut win) => {
                            if win.mapped {
                                backend.update_pos(&win);
                                backend.update_pixmap(&mut win);
                                backend.update_texture(&mut win);
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
        // Currently does not do anything useful
        xcb::CIRCULATE_NOTIFY => {
            println!("CIRCULATE_NOTIFY");
            let ev: &xcb::CirculateNotifyEvent =
                unsafe { xcb::cast_event(&base_event) };
            let win_above = if ev.place() == xcb::PLACE_ON_TOP as u8 {
                windows[0].id
            } else {
                xcb::NONE
            };
            restack_window(ev.window(), win_above, windows);
            for win in windows.iter_mut().filter(|w| w.mapped) {
                backend.draw_window(win);
            }
            backend.render();
        }
        // Window unhidden
        xcb::EXPOSE => {
            println!("EXPOSE");
            let ev: &xcb::ExposeEvent =
                unsafe { xcb::cast_event(&base_event) };
            // TODO: check if window is root
            //let win_id = ev.window();

            // Check number of remaining expose events
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
            println!("PROPERTY_NOTIFY");
            let _ev: &xcb::PropertyNotifyEvent =
                unsafe { xcb::cast_event(&base_event) };
        }
        // TODO: check for root property changes
        _ => {
            // Window damage detected
            if base_event.response_type() == damage::NOTIFY {
                let event: &damage::NotifyEvent =
                    unsafe { xcb::cast_event(&base_event) };
                damage::subtract(
                    &state.conn,
                    event.damage(),
                    xcb::NONE,
                    xcb::NONE,
                )
                .request_check()
                .unwrap();
                if let Some(i) =
                    windows.iter().position(|w| w.id == event.drawable())
                {
                    backend.update_texture(&mut windows[i]);
                }
                if last_render.elapsed() > *refresh_rate {
                    for win in windows.iter_mut().filter(|w| w.mapped) {
                        backend.draw_window(win);
                    }
                    backend.render();
                    *last_render = Instant::now();
                }
            }
        }
    }
}

fn restack_window(
    window: xcb::Window,
    above: xcb::Window,
    list: &mut Vec<Window>,
) {
    if let Some(i) = list.iter().position(|w| w.id == window) {
        let win = list.remove(i);

        if above != xcb::NONE {
            if let Some(pos) = list.iter().position(|w| w.id == above) {
                list.insert(pos + 1, win);
            } else {
                println!("Invalid above window: {}", above);
                list.push(win);
            }
        } else {
            list.push(win);
        }
    }
}
