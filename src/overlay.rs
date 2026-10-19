//! GNOME Wayland exposes no topmost request to native clients. A dedicated
//! XWayland display lets this small utility window use the standard EWMH request.
//! The overlay also skips the taskbar and pager so it behaves as an extension of
//! the main window instead of showing a second app icon.
use anyhow::{Context, Result};
use gtk::prelude::*;
use x11rb::{
    connection::Connection,
    protocol::xproto::{AtomEnum, ClientMessageEvent, ConnectionExt, EventMask, PropMode},
    wrapper::ConnectionExt as _,
};

pub fn display() -> Result<gtk::gdk::Display> {
    gdk_x11::X11Display::open(None).context(
        "Always-on-top needs X11/XWayland. Install or enable XWayland and reopen the overlay.",
    )
}

pub fn request_above(window: &gtk::ApplicationWindow, before_map: bool) -> Result<()> {
    let surface = window
        .surface()
        .context("Overlay has no native surface")?
        .downcast::<gdk_x11::X11Surface>()
        .map_err(|_| anyhow::anyhow!("Overlay must use the X11 display"))?;
    let xid = u32::try_from(surface.xid())?;
    let (connection, screen) =
        x11rb::connect(Some(&gtk::prelude::WidgetExt::display(window).name()))?;
    let atom =
        |name: &[u8]| -> Result<u32> { Ok(connection.intern_atom(false, name)?.reply()?.atom) };
    let state = atom(b"_NET_WM_STATE")?;
    let above = atom(b"_NET_WM_STATE_ABOVE")?;
    let skip_taskbar = atom(b"_NET_WM_STATE_SKIP_TASKBAR")?;
    let skip_pager = atom(b"_NET_WM_STATE_SKIP_PAGER")?;
    if before_map {
        let reply = connection
            .get_property(false, xid, state, AtomEnum::ATOM, 0, 1024)?
            .reply()?;
        let mut values = reply
            .value32()
            .map(|v| v.collect::<Vec<_>>())
            .unwrap_or_default();
        for wanted in [above, skip_taskbar, skip_pager] {
            if !values.contains(&wanted) {
                values.push(wanted);
            }
        }
        connection
            .change_property32(PropMode::REPLACE, xid, state, AtomEnum::ATOM, &values)?
            .check()?;
    } else {
        // Each _NET_WM_STATE message carries at most two properties.
        for (first, second) in [(above, skip_taskbar), (skip_pager, 0)] {
            let event = ClientMessageEvent::new(32, xid, state, [1, first, second, 1, 0]);
            connection
                .send_event(
                    false,
                    connection.setup().roots[screen].root,
                    EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                    event,
                )?
                .check()?;
        }
    }
    connection.flush()?;
    Ok(())
}

#[cfg(test)]
pub fn assert_above_if_managed(window: &gtk::ApplicationWindow) -> bool {
    let surface = window
        .surface()
        .unwrap()
        .downcast::<gdk_x11::X11Surface>()
        .expect("X11 overlay surface");
    let (connection, screen) =
        x11rb::connect(Some(&gtk::prelude::WidgetExt::display(window).name())).unwrap();
    let atom = |name: &[u8]| {
        connection
            .intern_atom(false, name)
            .unwrap()
            .reply()
            .unwrap()
            .atom
    };
    let above = atom(b"_NET_WM_STATE_ABOVE");
    let supported = connection
        .get_property(
            false,
            connection.setup().roots[screen].root,
            atom(b"_NET_SUPPORTED"),
            AtomEnum::ATOM,
            0,
            4096,
        )
        .unwrap()
        .reply()
        .unwrap();
    if supported
        .value32()
        .is_some_and(|mut values| values.any(|v| v == above))
    {
        let state = connection
            .get_property(
                false,
                surface.xid() as u32,
                atom(b"_NET_WM_STATE"),
                AtomEnum::ATOM,
                0,
                1024,
            )
            .unwrap()
            .reply()
            .unwrap();
        assert!(
            state
                .value32()
                .is_some_and(|mut values| values.any(|v| v == above)),
            "The window manager must report the overlay above other windows"
        );
        let skip_taskbar = atom(b"_NET_WM_STATE_SKIP_TASKBAR");
        assert!(
            state
                .value32()
                .is_some_and(|mut values| values.any(|v| v == skip_taskbar)),
            "The overlay must not show its own taskbar entry"
        );
        true
    } else {
        false
    }
}
