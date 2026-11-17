//! What a plain click in "Capture Area" meant. The system capture tool exits
//! without an image both on Esc and on a click without dragging, so after it
//! ends we ask the window server which came last, and which window was under
//! the pointer.

/// How long after the click the capture tool may take to exit.
const CLICK_WINDOW_SECS: f64 = 2.0;

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
    pub const LEFT_MOUSE_UP: u32 = 2;
    pub const KEY_DOWN: u32 = 10;
    pub const ON_SCREEN_ONLY: u32 = 1 << 0;
    pub const EXCLUDE_DESKTOP_ELEMENTS: u32 = 1 << 4;
    pub const CF_NUMBER_SINT64: CFIndex = 4;
    pub const CF_NUMBER_FLOAT64: CFIndex = 6;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        pub static kCGWindowNumber: CFTypeRef;
        pub static kCGWindowLayer: CFTypeRef;
        pub static kCGWindowBounds: CFTypeRef;
        pub static kCGWindowOwnerPID: CFTypeRef;
        pub static kCGWindowAlpha: CFTypeRef;
        pub fn CGEventSourceSecondsSinceLastEventType(state: i32, event_type: u32) -> f64;
        pub fn CGEventCreate(source: CFTypeRef) -> CFTypeRef;
        pub fn CGEventGetLocation(event: CFTypeRef) -> CGPoint;
        pub fn CGWindowListCopyWindowInfo(option: u32, relative_to: u32) -> CFTypeRef;
        pub fn CGRectMakeWithDictionaryRepresentation(dict: CFTypeRef, rect: *mut CGRect) -> bool;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        pub fn CFRelease(cf: CFTypeRef);
        pub fn CFArrayGetCount(array: CFTypeRef) -> CFIndex;
        pub fn CFArrayGetValueAtIndex(array: CFTypeRef, idx: CFIndex) -> CFTypeRef;
        pub fn CFDictionaryGetValue(dict: CFTypeRef, key: CFTypeRef) -> CFTypeRef;
        pub fn CFNumberGetValue(number: CFTypeRef, the_type: CFIndex, value: *mut c_void) -> bool;
    }
}

/// True when the capture tool most likely ended because of a mouse click
/// (and not because of Esc or another key).
#[cfg(target_os = "macos")]
pub fn ended_with_click() -> bool {
    use ffi::*;
    let (up, key) = unsafe {
        (
            CGEventSourceSecondsSinceLastEventType(COMBINED_SESSION_STATE, LEFT_MOUSE_UP),
            CGEventSourceSecondsSinceLastEventType(COMBINED_SESSION_STATE, KEY_DOWN),
        )
    };
    up < CLICK_WINDOW_SECS && up < key
}

/// The id of the normal app window under the pointer, if any (not the
/// desktop, the Dock or the menu bar, and never one of QuickSnap's own).
#[cfg(target_os = "macos")]
pub fn window_under_pointer() -> Option<u32> {
    use ffi::*;
    use std::ffi::c_void;
    unsafe {
        let ev = CGEventCreate(std::ptr::null());
        if ev.is_null() {
            return None;
        }
        let p = CGEventGetLocation(ev);
        CFRelease(ev);

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
            let inside = p.x >= r.origin.x && p.x < r.origin.x + r.size.x && p.y >= r.origin.y && p.y < r.origin.y + r.size.y;
            if !inside {
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
            // the topmost window here decides: a normal window is snapped,
            // anything above (menu bar, Dock, overlays) means "no window"
            if int(d, kCGWindowLayer) == Some(0) {
                found = int(d, kCGWindowNumber).map(|n| n as u32);
            }
            break;
        }
        CFRelease(list);
        found
    }
}

#[cfg(not(target_os = "macos"))]
pub fn ended_with_click() -> bool {
    false
}

#[cfg(not(target_os = "macos"))]
pub fn window_under_pointer() -> Option<u32> {
    None
}
