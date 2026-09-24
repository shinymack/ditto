use std::sync::atomic::{AtomicU32, Ordering};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::ConnectionExt as _;
use x11rb::wrapper::ConnectionExt as _;

static CACHED_X11_WIN_ID: AtomicU32 = AtomicU32::new(0);
const ICON_BYTES: &[u8] = include_bytes!("../../../assets/icon.png");

/// Finds and caches the X11 window ID for the current Ditto process by checking `_NET_WM_PID`.
/// Works whether the window is mapped or unmapped.
pub fn get_ditto_x11_window_id() -> Option<u32> {
    let cached = CACHED_X11_WIN_ID.load(Ordering::SeqCst);
    if cached != 0 {
        return Some(cached);
    }

    let (conn, screen_num) = x11rb::connect(None).ok()?;
    let root = conn.setup().roots[screen_num].root;
    let my_pid = std::process::id();

    let intern = |name: &str| -> u32 {
        conn.intern_atom(false, name.as_bytes())
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|r| r.atom)
            .unwrap_or(0)
    };
    let net_wm_pid = intern("_NET_WM_PID");
    if net_wm_pid == 0 {
        return None;
    }

    if let Ok(tree_reply) = conn.query_tree(root) {
        if let Ok(tree) = tree_reply.reply() {
            for win in tree.children {
                if let Ok(pid_reply) = conn.get_property(
                    false,
                    win,
                    net_wm_pid,
                    x11rb::protocol::xproto::AtomEnum::CARDINAL,
                    0,
                    1,
                ) {
                    if let Ok(pid_prop) = pid_reply.reply() {
                        if let Some(mut pids) = pid_prop.value32() {
                            if pids.next() == Some(my_pid) {
                                CACHED_X11_WIN_ID.store(win, Ordering::SeqCst);
                                return Some(win);
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Checks if our X11 window is currently mapped (VIEWABLE) on the screen.
pub fn is_window_mapped() -> bool {
    let Some(win_id) = get_ditto_x11_window_id() else {
        return false;
    };
    let Ok((conn, _)) = x11rb::connect(None) else {
        return false;
    };
    conn.get_window_attributes(win_id)
        .ok()
        .and_then(|c| c.reply().ok())
        .map(|attr| attr.map_state == x11rb::protocol::xproto::MapState::VIEWABLE)
        .unwrap_or(false)
}

/// Sets standard window manager properties, position hints, and icon on the X11 window.
pub fn apply_window_hints(
    win_id: u32,
    pos: Option<(i32, i32)>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (conn, screen_num) = x11rb::connect(None)?;
    let root = conn.setup().roots[screen_num].root;

    let intern = |name: &str| -> u32 {
        conn.intern_atom(false, name.as_bytes())
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|r| r.atom)
            .unwrap_or(0)
    };

    let net_wm_window_type = intern("_NET_WM_WINDOW_TYPE");
    let net_wm_window_type_normal = intern("_NET_WM_WINDOW_TYPE_NORMAL");
    let net_wm_state = intern("_NET_WM_STATE");
    let net_wm_state_above = intern("_NET_WM_STATE_ABOVE");
    let net_wm_icon = intern("_NET_WM_ICON");
    let wm_normal_hints = intern("WM_NORMAL_HINTS");
    let wm_size_hints = intern("WM_SIZE_HINTS");

    // Explicitly set WM_NORMAL_HINTS with USPosition (1) | PPosition (4) so WMs (Mutter/KWin)
    // honor the exact saved position rather than centering the window!
    if let Some((x, y)) = pos {
        if wm_normal_hints != 0 && wm_size_hints != 0 {
            let mut size_hints = [0u32; 18];
            size_hints[0] = 5; // USPosition (1) | PPosition (4)
            size_hints[1] = x as u32;
            size_hints[2] = y as u32;
            size_hints[3] = 800;
            size_hints[4] = 500;
            let _ = conn.change_property32(
                x11rb::protocol::xproto::PropMode::REPLACE,
                win_id,
                wm_normal_hints,
                wm_size_hints,
                &size_hints,
            );
        }
    }
    // Set WM_CLASS = "ditto\0ditto\0" so docks match to ditto.desktop
    let mut class_data = Vec::with_capacity(14);
    class_data.extend_from_slice(b"ditto\0ditto\0");
    let _ = conn.change_property8(
        x11rb::protocol::xproto::PropMode::REPLACE,
        win_id,
        x11rb::protocol::xproto::AtomEnum::WM_CLASS,
        x11rb::protocol::xproto::AtomEnum::STRING,
        &class_data,
    );

    // Set window type = NORMAL
    if net_wm_window_type != 0 && net_wm_window_type_normal != 0 {
        let _ = conn.change_property32(
            x11rb::protocol::xproto::PropMode::REPLACE,
            win_id,
            net_wm_window_type,
            x11rb::protocol::xproto::AtomEnum::ATOM,
            &[net_wm_window_type_normal],
        );
    }

    // Set _NET_WM_ICON
    if net_wm_icon != 0 {
        if let Ok(img) = image::load_from_memory(ICON_BYTES) {
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            let mut icon_data: Vec<u32> = Vec::with_capacity((2 + w * h) as usize);
            icon_data.push(w);
            icon_data.push(h);
            for pixel in rgba.pixels() {
                let [r, g, b, a] = pixel.0;
                let argb = ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                icon_data.push(argb);
            }
            let _ = conn.change_property32(
                x11rb::protocol::xproto::PropMode::REPLACE,
                win_id,
                net_wm_icon,
                x11rb::protocol::xproto::AtomEnum::CARDINAL,
                &icon_data,
            );
        }
    }

    // Request _NET_WM_STATE_ABOVE
    if net_wm_state_above != 0 && net_wm_state != 0 {
        let data = [1u32, net_wm_state_above, 0, 1, 0];
        let msg = x11rb::protocol::xproto::ClientMessageEvent {
            response_type: x11rb::protocol::xproto::CLIENT_MESSAGE_EVENT,
            format: 32,
            sequence: 0,
            window: win_id,
            type_: net_wm_state,
            data: x11rb::protocol::xproto::ClientMessageData::from(data),
        };
        let _ = conn.send_event(
            false,
            root,
            x11rb::protocol::xproto::EventMask::SUBSTRUCTURE_REDIRECT
                | x11rb::protocol::xproto::EventMask::SUBSTRUCTURE_NOTIFY,
            msg,
        );
    }

    let _ = conn.flush();
    Ok(())
}

/// Unmaps the window in X11 and returns its root-relative coordinates.
pub fn unmap_and_get_coords() -> Option<(i32, i32)> {
    let win_id = get_ditto_x11_window_id()?;
    let (conn, _) = x11rb::connect(None).ok()?;
    let root = conn.setup().roots.first()?.root;

    let coords = conn
        .translate_coordinates(win_id, root, 0, 0)
        .ok()?
        .reply()
        .ok()?;
    let x = coords.dst_x as i32;
    let y = coords.dst_y as i32;

    conn.unmap_window(win_id).ok();
    conn.flush().ok();

    Some((x, y))
}

/// Maps and activates the window purely using native Rust X11 protocol without external CLI dependencies.
pub fn native_x11_activate(win_id: u32) -> bool {
    let Ok((conn, screen_num)) = x11rb::connect(None) else {
        return false;
    };
    let root = conn.setup().roots[screen_num].root;

    let intern = |name: &str| -> u32 {
        conn.intern_atom(false, name.as_bytes())
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|r| r.atom)
            .unwrap_or(0)
    };

    let net_current_desktop = intern("_NET_CURRENT_DESKTOP");
    let net_wm_desktop = intern("_NET_WM_DESKTOP");
    let net_active_window = intern("_NET_ACTIVE_WINDOW");

    // 1. If multi-workspace, move window to current workspace
    if net_current_desktop != 0 && net_wm_desktop != 0 {
        if let Ok(reply) = conn.get_property(
            false,
            root,
            net_current_desktop,
            x11rb::protocol::xproto::AtomEnum::CARDINAL,
            0,
            1,
        ) {
            if let Ok(prop) = reply.reply() {
                if let Some(mut current_desktop) = prop.value32() {
                    if let Some(desktop) = current_desktop.next() {
                        let data = [desktop, 2, 0, 0, 0];
                        let msg = x11rb::protocol::xproto::ClientMessageEvent {
                            response_type: x11rb::protocol::xproto::CLIENT_MESSAGE_EVENT,
                            format: 32,
                            sequence: 0,
                            window: win_id,
                            type_: net_wm_desktop,
                            data: x11rb::protocol::xproto::ClientMessageData::from(data),
                        };
                        let _ = conn.send_event(
                            false,
                            root,
                            x11rb::protocol::xproto::EventMask::SUBSTRUCTURE_REDIRECT
                                | x11rb::protocol::xproto::EventMask::SUBSTRUCTURE_NOTIFY,
                            msg,
                        );
                    }
                }
            }
        }
    }

    // 2. Map window
    let _ = conn.map_window(win_id);

    // 3. Send _NET_ACTIVE_WINDOW client message
    if net_active_window != 0 {
        let data = [2u32, 0, 0, 0, 0]; // 2 = direct user action / pager
        let msg = x11rb::protocol::xproto::ClientMessageEvent {
            response_type: x11rb::protocol::xproto::CLIENT_MESSAGE_EVENT,
            format: 32,
            sequence: 0,
            window: win_id,
            type_: net_active_window,
            data: x11rb::protocol::xproto::ClientMessageData::from(data),
        };
        let _ = conn.send_event(
            false,
            root,
            x11rb::protocol::xproto::EventMask::SUBSTRUCTURE_REDIRECT
                | x11rb::protocol::xproto::EventMask::SUBSTRUCTURE_NOTIFY,
            msg,
        );
    }

    let _ = conn.flush();
    true
}

/// Moves and resizes the window purely using native Rust X11 protocol.
pub fn native_x11_moveresize(win_id: u32, x: i32, y: i32, width: u32, height: u32) -> bool {
    let Ok((conn, screen_num)) = x11rb::connect(None) else {
        return false;
    };
    let root = conn.setup().roots[screen_num].root;

    // Direct configure window request
    let values = x11rb::protocol::xproto::ConfigureWindowAux::new()
        .x(x)
        .y(y)
        .width(width)
        .height(height);
    let _ = conn.configure_window(win_id, &values);

    // EWMH _NET_MOVERESIZE_WINDOW client message for window managers
    let net_moveresize = conn
        .intern_atom(false, b"_NET_MOVERESIZE_WINDOW")
        .ok()
        .and_then(|c| c.reply().ok())
        .map(|r| r.atom)
        .unwrap_or(0);

    if net_moveresize != 0 {
        let flags: u32 = 1 | (1 << 8) | (1 << 9) | (1 << 10) | (1 << 11);
        let data = [flags, x as u32, y as u32, width, height];
        let msg = x11rb::protocol::xproto::ClientMessageEvent {
            response_type: x11rb::protocol::xproto::CLIENT_MESSAGE_EVENT,
            format: 32,
            sequence: 0,
            window: win_id,
            type_: net_moveresize,
            data: x11rb::protocol::xproto::ClientMessageData::from(data),
        };
        let _ = conn.send_event(
            false,
            root,
            x11rb::protocol::xproto::EventMask::SUBSTRUCTURE_REDIRECT
                | x11rb::protocol::xproto::EventMask::SUBSTRUCTURE_NOTIFY,
            msg,
        );
    }

    let _ = conn.flush();
    true
}
