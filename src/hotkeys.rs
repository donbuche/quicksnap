//! Global keyboard shortcuts: the actions listed on the Hotkeys tab, their
//! default keys (⌥⌘ + digit, from the original's hotkey settings item) and
//! registration with the system so they work while another app is in front.

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::GlobalHotKeyManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub enum Action {
    CaptureArea,
    CaptureFrozenArea,
    CaptureAreaUpload,
    CaptureFullscreen,
    CaptureFullscreenDelayed,
    OpenEditor,
    RecordVideo,
    CapturePreviousArea,
    OpenFromClipboard,
    VideoRecordPause,
    VideoStop,
    VideoArrow,
    VideoRectangle,
    VideoFullscreen,
}

impl Action {
    pub const CAPTURE_ROWS: [Action; 9] = [
        Action::CaptureArea,
        Action::CaptureFrozenArea,
        Action::CaptureAreaUpload,
        Action::CaptureFullscreen,
        Action::CaptureFullscreenDelayed,
        Action::OpenEditor,
        Action::RecordVideo,
        Action::CapturePreviousArea,
        Action::OpenFromClipboard,
    ];
    pub const VIDEO_ROWS: [Action; 5] = [
        Action::VideoRecordPause,
        Action::VideoStop,
        Action::VideoArrow,
        Action::VideoRectangle,
        Action::VideoFullscreen,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Action::CaptureArea => "Capture area:",
            Action::CaptureFrozenArea => "Capture frozen area:",
            Action::CaptureAreaUpload => "Capture area & Upload:",
            Action::CaptureFullscreen => "Capture fullscreen:",
            Action::CaptureFullscreenDelayed => "Fullscreen after 10s:",
            Action::OpenEditor => "Open Latest Image:",
            Action::RecordVideo => "Record video:",
            Action::CapturePreviousArea => "Capture previous area:",
            Action::OpenFromClipboard => "Open from clipboard:",
            Action::VideoRecordPause => "Record/Pause:",
            Action::VideoStop => "Stop:",
            Action::VideoArrow => "Arrow:",
            Action::VideoRectangle => "Rectangle:",
            Action::VideoFullscreen => "Fullscreen / Windowed mode:",
        }
    }
}

/// A key combination. `key` is an egui key name such as "5", "A" or "F1".
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Hotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub cmd: bool,
    pub key: String,
}

impl Hotkey {
    pub fn alt_cmd(key: &str) -> Hotkey {
        Hotkey { ctrl: false, alt: true, shift: false, cmd: true, key: key.into() }
    }

    pub fn display(&self) -> String {
        let mut s = String::new();
        if self.ctrl {
            s.push('⌃');
        }
        if self.alt {
            s.push('⌥');
        }
        if self.shift {
            s.push('⇧');
        }
        if self.cmd {
            s.push('⌘');
        }
        s.push_str(&self.key);
        s
    }

    fn to_global(&self) -> Option<HotKey> {
        let mut m = Modifiers::empty();
        if self.ctrl {
            m |= Modifiers::CONTROL;
        }
        if self.alt {
            m |= Modifiers::ALT;
        }
        if self.shift {
            m |= Modifiers::SHIFT;
        }
        if self.cmd {
            m |= Modifiers::SUPER;
        }
        let code = key_code(&self.key)?;
        Some(HotKey::new(Some(m), code))
    }
}

fn key_code(name: &str) -> Option<Code> {
    Some(match name {
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        "A" => Code::KeyA,
        "B" => Code::KeyB,
        "C" => Code::KeyC,
        "D" => Code::KeyD,
        "E" => Code::KeyE,
        "F" => Code::KeyF,
        "G" => Code::KeyG,
        "H" => Code::KeyH,
        "I" => Code::KeyI,
        "J" => Code::KeyJ,
        "K" => Code::KeyK,
        "L" => Code::KeyL,
        "M" => Code::KeyM,
        "N" => Code::KeyN,
        "O" => Code::KeyO,
        "P" => Code::KeyP,
        "Q" => Code::KeyQ,
        "R" => Code::KeyR,
        "S" => Code::KeyS,
        "T" => Code::KeyT,
        "U" => Code::KeyU,
        "V" => Code::KeyV,
        "W" => Code::KeyW,
        "X" => Code::KeyX,
        "Y" => Code::KeyY,
        "Z" => Code::KeyZ,
        "F1" => Code::F1,
        "F2" => Code::F2,
        "F3" => Code::F3,
        "F4" => Code::F4,
        "F5" => Code::F5,
        "F6" => Code::F6,
        "F7" => Code::F7,
        "F8" => Code::F8,
        "F9" => Code::F9,
        "F10" => Code::F10,
        "F11" => Code::F11,
        "F12" => Code::F12,
        _ => return None,
    })
}

/// Defaults from the original: Capture Area ⌥⌘5, Frozen ⌥⌘8, Area & Upload ⌥⌘7,
/// Fullscreen ⌥⌘6, Open Editor ⌥⌘0. Delayed fullscreen and video are created
/// switched off, so they start as "Not set".
pub fn default_hotkeys() -> Vec<(Action, Option<Hotkey>)> {
    let mut v: Vec<(Action, Option<Hotkey>)> = Vec::new();
    for a in Action::CAPTURE_ROWS.iter().chain(Action::VIDEO_ROWS.iter()) {
        let k = match a {
            Action::CaptureArea => Some(Hotkey::alt_cmd("5")),
            Action::CaptureFrozenArea => Some(Hotkey::alt_cmd("8")),
            Action::CaptureAreaUpload => Some(Hotkey::alt_cmd("7")),
            Action::CaptureFullscreen => Some(Hotkey::alt_cmd("6")),
            Action::OpenEditor => Some(Hotkey::alt_cmd("0")),
            _ => None,
        };
        v.push((*a, k));
    }
    v
}

/// Owns the system registration and maps fired ids back to actions.
pub struct GlobalKeys {
    manager: Option<GlobalHotKeyManager>,
    registered: Vec<HotKey>,
    by_id: HashMap<u32, Action>,
}

impl GlobalKeys {
    pub fn new() -> GlobalKeys {
        GlobalKeys { manager: GlobalHotKeyManager::new().ok(), registered: Vec::new(), by_id: HashMap::new() }
    }

    pub fn apply(&mut self, keys: &[(Action, Option<Hotkey>)]) {
        let Some(m) = &self.manager else { return };
        if !self.registered.is_empty() {
            let _ = m.unregister_all(&self.registered);
        }
        self.registered.clear();
        self.by_id.clear();
        for (a, k) in keys {
            // Drawing on a recording isn't supported here, so those keys are kept
            // in the settings but not registered.
            if matches!(a, Action::VideoArrow | Action::VideoRectangle | Action::VideoFullscreen) {
                continue;
            }
            if let Some(hk) = k.as_ref().and_then(|k| k.to_global()) {
                if m.register(hk).is_ok() {
                    self.by_id.insert(hk.id(), *a);
                    self.registered.push(hk);
                }
            }
        }
    }

    pub fn action_for(&self, id: u32) -> Option<Action> {
        self.by_id.get(&id).copied()
    }
}
