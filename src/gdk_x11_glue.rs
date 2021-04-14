use gdk4_x11::glib::translate::ToGlibPtr;
use gdk4_x11::{ffi, X11Display, X11Surface};
use gtk::prelude::*;
use x11::xlib;

pub trait X11DisplayExt {
    fn get_xdisplay(&self) -> *mut xlib::Display;
}

impl X11DisplayExt for X11Display {
    fn get_xdisplay(&self) -> *mut xlib::Display {
        unsafe { ffi::gdk_x11_display_get_xdisplay(self.to_glib_none().0) }
    }
}

pub trait WindowGeometry {
    fn get_window_geometry(&self) -> Option<gdk::Rectangle>;
    fn set_window_geometry(&self, rect: &gdk::Rectangle);
    fn get_window_screen(&self) -> Option<gdk::Rectangle>;
}

impl<W: IsA<gtk::Window> + IsA<gtk::Native>> WindowGeometry for W {
    fn get_window_geometry(&self) -> Option<gdk::Rectangle> {
        let surface = self.surface()?;
        let xs = surface.clone().downcast::<X11Surface>().ok()?;
        let xd = surface.display()?.downcast::<X11Display>().ok()?;
        unsafe {
            let screen = x11::xlib::XDefaultRootWindow(xd.get_xdisplay());
            let mut _child: u64 = 0;
            let mut x: i32 = 0;
            let mut y: i32 = 0;

            x11::xlib::XTranslateCoordinates(
                xd.get_xdisplay(),
                xs.xid(),
                screen,
                0,
                0,
                &mut x,
                &mut y,
                &mut _child,
            );
            let (width, height) = self.default_size();
            Some(gdk::Rectangle { x, y, width, height })
        }
    }

    fn set_window_geometry(&self, rect: &gdk::Rectangle) {
        fn get<W: IsA<gtk::Window> + IsA<gtk::Native>>(
            window: &W,
        ) -> Option<(X11Surface, X11Display)> {
            let surface = window.surface()?;
            let xs = surface.clone().downcast::<X11Surface>().ok()?;
            let xd = surface.display()?.downcast::<X11Display>().ok()?;
            Some((xs, xd))
        }
        self.set_default_size(rect.width, rect.height);
        if let Some((xs, xd)) = get(self) {
            unsafe {
                // https://tronche.com/gui/x/xlib/window/configure.html#XWindowChanges
                let mut hints = x11::xlib::XWindowChanges {
                    x: rect.x,
                    y: rect.y,
                    width: 0,
                    height: 0,
                    border_width: 0,
                    sibling: 0,
                    stack_mode: 0,
                };
                // See link above, 3 = (1 << 0) | (1 << 1);
                let mask = 3;
                // _res is always 1, even if the window is not moved to x, y
                let _res =
                    x11::xlib::XConfigureWindow(xd.get_xdisplay(), xs.xid(), mask, &mut hints);
            }
        }
    }

    fn get_window_screen(&self) -> Option<gdk::Rectangle> {
        let surface = self.surface()?;
        let xd = surface.display()?.downcast::<X11Display>().ok()?;
        unsafe {
            let screen = x11::xlib::XDefaultRootWindow(xd.get_xdisplay());
            let mut _root: u64 = 0;
            let mut x: i32 = 0;
            let mut y: i32 = 0;
            let mut w: u32 = 0;
            let mut h: u32 = 0;
            let mut _border: u32 = 0;
            let mut _depth: u32 = 0;
            x11::xlib::XGetGeometry(
                xd.get_xdisplay(),
                screen,
                &mut _root,
                &mut x,
                &mut y,
                &mut w,
                &mut h,
                &mut _border,
                &mut _depth,
            );
            Some(gdk::Rectangle { x, y, width: w as i32, height: h as i32 })
        }
    }
}
