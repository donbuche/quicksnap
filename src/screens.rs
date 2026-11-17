//! Displays, windows and the pointer as the window server sees them, for
//! QuickSnap's own "Capture Area" selector. All rectangles are in global
//! screen points with a top-left origin (the main display starts at 0,0;
//! a display to its left has negative x), which is also what the system
//! capture tool's -R takes.

/// A rectangle in global screen points.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct ScreenRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl ScreenRect {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }

    fn covers(&self, o: &ScreenRect) -> bool {
        self.x <= o.x && self.y <= o.y && self.x + self.w >= o.x + o.w && self.y + self.h >= o.y + o.h
    }

    /// `x,y,w,h` as the capture tool's -R option wants it.
    pub fn arg(&self) -> String {
        format!("{},{},{},{}", self.x.round(), self.y.round(), self.w.round(), self.h.round())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Display {
    pub rect: ScreenRect,
    /// The display with the menu bar's "main" role (origin 0,0).
    pub main: bool,
}

#[cfg(target_os = "macos")]
mod ffi {
    use std::ffi::c_void;

    pub type CFTypeRef = *const c_void;
    pub type CFIndex = isize;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct CGPoint {
        pub x: f64,
        pub y: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct CGRect {
        pub origin: CGPoint,
        pub size: CGPoint,
    }

    pub const COMBINED_SESSION_STATE: i32 = 0;
    pub const ON_SCREEN_ONLY: u32 = 1 << 0;
    pub const EXCLUDE_DESKTOP_ELEMENTS: u32 = 1 << 4;
    pub const CF_NUMBER_SINT64: CFIndex = 4;
    pub const CF_NUMBER_FLOAT64: CFIndex = 6;
    /// The level of the window that draws the mouse pointer.
    pub const CURSOR_LAYER: i64 = 2147483630;
    pub const KEY_ESCAPE: u16 = 0x35;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        pub static kCGWindowNumber: CFTypeRef;
        pub static kCGWindowLayer: CFTypeRef;
        pub static kCGWindowBounds: CFTypeRef;
        pub static kCGWindowOwnerPID: CFTypeRef;
        pub static kCGWindowAlpha: CFTypeRef;
        pub fn CGEventCreate(source: CFTypeRef) -> CFTypeRef;
        pub fn CGEventGetLocation(event: CFTypeRef) -> CGPoint;
        pub fn CGEventSourceKeyState(state: i32, key: u16) -> bool;
        pub fn CGWindowListCopyWindowInfo(option: u32, relative_to: u32) -> CFTypeRef;
        pub fn CGRectMakeWithDictionaryRepresentation(dict: CFTypeRef, rect: *mut CGRect) -> bool;
        pub fn CGGetActiveDisplayList(max: u32, displays: *mut u32, count: *mut u32) -> i32;
        pub fn CGDisplayBounds(display: u32) -> CGRect;
        pub fn CGMainDisplayID() -> u32;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        pub fn CFRelease(cf: CFTypeRef);
        pub fn CFArrayGetCount(array: CFTypeRef) -> CFIndex;
        pub fn CFArrayGetValueAtIndex(array: CFTypeRef, idx: CFIndex) -> CFTypeRef;
        pub fn CFDictionaryGetValue(dict: CFTypeRef, key: CFTypeRef) -> CFTypeRef;
        pub fn CFNumberGetValue(number: CFTypeRef, the_type: CFIndex, value: *mut c_void) -> bool;
    }

    pub fn rect(r: CGRect) -> super::ScreenRect {
        super::ScreenRect { x: r.origin.x, y: r.origin.y, w: r.size.x, h: r.size.y }
    }
}

/// The pointer position in global screen points.
#[cfg(target_os = "macos")]
pub fn pointer() -> Option<(f64, f64)> {
    use ffi::*;
    unsafe {
        let ev = CGEventCreate(std::ptr::null());
        if ev.is_null() {
            return None;
        }
        let p = CGEventGetLocation(ev);
        CFRelease(ev);
        Some((p.x, p.y))
    }
}

/// Whether Esc is held down right now (a backup for when the selector
/// window does not have the keyboard).
#[cfg(target_os = "macos")]
pub fn escape_down() -> bool {
    unsafe { ffi::CGEventSourceKeyState(ffi::COMBINED_SESSION_STATE, ffi::KEY_ESCAPE) }
}

/// Every active display.
#[cfg(target_os = "macos")]
pub fn displays() -> Vec<Display> {
    use ffi::*;
    let mut ids = [0u32; 16];
    let mut count = 0u32;
    unsafe {
        if CGGetActiveDisplayList(ids.len() as u32, ids.as_mut_ptr(), &mut count) != 0 {
            return Vec::new();
        }
        let main = CGMainDisplayID();
        ids[..count as usize].iter().map(|&id| Display { rect: rect(CGDisplayBounds(id)), main: id == main }).collect()
    }
}

/// The normal app window at a point, with its id and bounds: not the
/// desktop, the Dock or the menu bar, and never one of QuickSnap's own.
#[cfg(target_os = "macos")]
pub fn window_at(x: f64, y: f64, screen: Option<ScreenRect>) -> Option<(u32, ScreenRect)> {
    use ffi::*;
    use std::ffi::c_void;
    unsafe {
        let list = CGWindowListCopyWindowInfo(ON_SCREEN_ONLY | EXCLUDE_DESKTOP_ELEMENTS, 0);
        if list.is_null() {
            return None;
        }
        let me = std::process::id() as i64;
        let int = |d: CFTypeRef, k: CFTypeRef| -> Option<i64> {
            let v = CFDictionaryGetValue(d, k);
            let mut out: i64 = 0;
            (!v.is_null() && CFNumberGetValue(v, CF_NUMBER_SINT64, &mut out as *mut i64 as *mut c_void)).then_some(out)
        };
        let mut found = None;
        // front to back: the first window containing the point is the visible one
        for i in 0..CFArrayGetCount(list) {
            let d = CFArrayGetValueAtIndex(list, i);
            let bounds = CFDictionaryGetValue(d, kCGWindowBounds);
            let mut r = CGRect::default();
            if bounds.is_null() || !CGRectMakeWithDictionaryRepresentation(bounds, &mut r) {
                continue;
            }
            let r = rect(r);
            if !r.contains(x, y) {
                continue;
            }
            let alpha_v = CFDictionaryGetValue(d, kCGWindowAlpha);
            let mut alpha: f64 = 1.0;
            if !alpha_v.is_null() {
                CFNumberGetValue(alpha_v, CF_NUMBER_FLOAT64, &mut alpha as *mut f64 as *mut c_void);
            }
            if alpha <= 0.0 || int(d, kCGWindowOwnerPID) == Some(me) {
                continue;
            }
            let layer = int(d, kCGWindowLayer);
            // the pointer itself is drawn in a small window above everything
            if layer == Some(CURSOR_LAYER) {
                continue;
            }
            // the Dock (and similar tools) keep a see-through window over the
            // whole display above the apps: it does not hide anything
            if layer != Some(0) && screen.is_some_and(|s| r.covers(&s)) {
                continue;
            }
            // the topmost window here decides: a normal window is snapped,
            // anything above (menu bar, Dock icons, panels) means "no window"
            if layer == Some(0) {
                found = int(d, kCGWindowNumber).map(|n| (n as u32, r));
            }
            break;
        }
        CFRelease(list);
        found
    }
}

/// Makes the selector window titled `title` cover `r` above everything
/// (menu bar and Dock included), on every Space, and catch clicks on its
/// see-through parts. Returns false while the window does not exist yet.
#[cfg(target_os = "macos")]
pub fn raise_selector_window(title: &str, r: ScreenRect) -> bool {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSWindowCollectionBehavior};
    use objc2_foundation::{NSPoint, NSRect, NSSize};
    let Some(mtm) = MainThreadMarker::new() else { return false };
    let app = NSApplication::sharedApplication(mtm);
    let main_h = unsafe { ffi::CGDisplayBounds(ffi::CGMainDisplayID()).size.y };
    for w in app.windows().iter() {
        if w.title().to_string() != title {
            continue;
        }
        // NSScreenSaverWindowLevel: above the menu bar and the Dock
        w.setLevel(1000);
        w.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle,
        );
        w.setIgnoresMouseEvents(false);
        // Cocoa frames have a bottom-left origin on the main display
        let frame = NSRect::new(NSPoint::new(r.x, main_h - (r.y + r.h)), NSSize::new(r.w, r.h));
        w.setFrame_display(frame, true);
        w.orderFrontRegardless();
        return true;
    }
    false
}

/// Brings QuickSnap to the front so its selector gets the keyboard (Esc).
#[cfg(target_os = "macos")]
pub fn activate_app() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;
    if let Some(mtm) = MainThreadMarker::new() {
        #[allow(deprecated)]
        NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn pointer() -> Option<(f64, f64)> {
    None
}

#[cfg(not(target_os = "macos"))]
pub fn escape_down() -> bool {
    false
}

#[cfg(not(target_os = "macos"))]
pub fn displays() -> Vec<Display> {
    Vec::new()
}

#[cfg(not(target_os = "macos"))]
pub fn window_at(_x: f64, _y: f64, _screen: Option<ScreenRect>) -> Option<(u32, ScreenRect)> {
    None
}

#[cfg(not(target_os = "macos"))]
pub fn raise_selector_window(_title: &str, _r: ScreenRect) -> bool {
    true
}

#[cfg(not(target_os = "macos"))]
pub fn activate_app() {}
