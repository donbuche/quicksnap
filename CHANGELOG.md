# Changelog

All notable changes to QuickSnap. Versions follow [Semantic Versioning](https://semver.org).

## [Unreleased]

### Added
- Capture Area: a click without dragging captures the whole screen when
  "Click to snap a window" is off

### Fixed
- "Click to snap a window" now works: in Capture Area, a click without dragging captures
  the window under the pointer (or the whole screen when there is no window there).
  Before, the option only enabled the system tool's Space-key window mode

## [0.4.0] - 2026-11-16

### Added
- Click sound: a soft click when a button, tab, tile, checkbox or menu item is pressed
  (the same sound as MacControl), on by default and switchable in Preferences → General.
  It falls back to smaller audio buffers when an output device refuses the default one

### Changed
- Nearly flat look: one faint shadow instead of large soft ones, and much weaker glows
- Home screen: the six capture icons are larger and stand on their own (no background
  or border) and turn turquoise on hover, keyboard focus and press; the tiles are wider
  so long labels keep a margin; the line about the menu-bar icon is gone
- Language picker styled like the other buttons, with more padding, in the home screen
  and in Preferences; flags are larger and pixel-aligned, so the nine stripes of the
  Catalan flag stay sharp
- Selected buttons (theme, menus, hotkey being recorded) have white text on the violet,
  for enough contrast in light mode
- Preferences: each label is centred on its row's controls; the magnifier and hints
  options are in their own "Frozen area" group, the only capture they apply to
- README and website screenshots updated

### Fixed
- "About QuickSnap" in Preferences did nothing visible: it opened behind the
  Preferences window; it now opens over it

### Removed
- Preferences that did nothing: "Highlight window under cursor", the menu-bar "Show
  hints" and "Drag and drop copy"

## [0.3.1] - 2026-11-16

No changes in the app itself; this version is about how it is released.

### Added
- Releases are published automatically: every push to `main` with a new version builds
  the DMG on GitHub Actions and creates the GitHub release, with notes from this file
- The latest DMG is always at `releases/latest/download/QuickSnap.dmg`

### Changed
- README: new home screenshot with the current glass look, and a direct download link

## [0.3.0] - 2026-11-16

### Added
- Interface in 15 languages: French, German, Italian, Portuguese (Portugal and Brazil),
  Dutch, Polish, Galician, Basque, Japanese, Chinese (simplified) and Korean join English,
  Spanish and Catalan, each with its flag in the language picker
- Chinese, Japanese and Korean use macOS's own fonts for them (Hiragino Sans, Hiragino
  Sans GB, Apple SD Gothic Neo), loaded only when that language is chosen

### Changed
- Language menus list the languages by their name in the interface language, in that
  language's alphabetical order, with each language's own name in grey
- Home screen tiles a little wider, so labels fit in every language
- Preferences tab names shrink to fit when a translation is long

## [0.2.0] - 2026-11-16

### Added
- Glass look: see-through main window with the desktop blurred behind it (macOS
  vibrancy), a colour-gradient backdrop and see-through panels with a soft sheen
- No visible title bar: the app runs up to the top edge, with only the window buttons
- Violet-to-blue accent gradient and a turquoise hover colour with a glow
- Custom colour picker with a hex code field, fixed size and small markers
- Zoom by pinching the trackpad or ⌘ + scroll, around the pointer
- Zoom menu in one "100% ⌄" button (zoom in/out, fit, original size, picture size);
  double-click switches between fit and 100 %

### Changed
- Editor: two top bars instead of three; Copy and Open in… are icon buttons
- Preferences use the same gradient backdrop
- Language picker on the home screen as far from the right edge as from the top

### Removed
- The darker red from the colour palette (it looked like a second pure red)

## [0.1.0] - 2026-11-15

First version: area, frozen-area, full-screen and delayed captures, screen recording,
annotation editor, menu-bar app, global hotkeys and an English/Spanish/Catalan interface.

[0.3.0]: https://github.com/donbuche/quicksnap/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/donbuche/quicksnap/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/donbuche/quicksnap/releases/tag/v0.1.0
