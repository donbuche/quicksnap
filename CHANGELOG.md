# Changelog

All notable changes to QuickSnap. Versions follow [Semantic Versioning](https://semver.org).

## [Unreleased]

### Changed
- README: links to the step-by-step guide (with pictures, in 15 languages) for opening
  QuickSnap the first time, and uses macOS's current name for the Screen & System Audio
  Recording pane

## [0.5.1] - 2026-11-17

### Changed
- Home screen: the background's colour glows are scattered at random on every launch
  (no longer one per corner) and wander along their own irregular paths; each reacts
  to the pointer its own way, with its own strength, direction and delay, and some
  drift away from it. They glide smoothly (drawn as meshes at about 30 frames a second
  instead of a picture redrawn a few times a second)
- Hover is violet instead of turquoise everywhere (home screen, language picker, editor
  tools, the Save button): a light tint of the accent, while selected things stay solid
  violet, so the app uses one accent colour
- About box: more room inside (wider margins, more space between the lines) and an
  opaque background, so the settings behind it no longer show through the text
- The background's diagonal gradient is clearer in both themes: violet at the top left
  to blue at the bottom right (dark `#24124A` → `#061C3C`, light `#EEE3FF` → `#DCEEFF`)
- App icon: softer blade colours, halfway between macOS's system colours and pastels
  (`#FF6968`, `#FFA946`, `#FFD546`, `#6AD382`, `#46CFD0`, `#509EFE`, `#BB7CE9`)

## [0.5.0] - 2026-11-17

### Added
- Capture Area has its own selection layer over every display (the screen stays live):
  drag an area (⇧ for a square, size shown with "Show hints"), click to capture, Esc to
  cancel. It covers the menu bar and the Dock, and works on every connected display
- Capture Area: with "Click to snap a window" on, the window under the pointer is
  highlighted and a click captures just that window; with it off, a click captures the
  whole display that was clicked

### Changed
- Home screen: flat capture tiles, bottom buttons and language picker (no sheen along
  the top) on a dark navy shadow instead of the turquoise glow. The shadow lifts wider
  and darker on hover and keyboard focus and tightens when pressed
- Home screen: Open…, Open Latest Image and Preferences… have icons (a cog wheel for
  Preferences), are a little taller and stand further apart from the capture tiles
- Home screen: the capture tiles are taller, with the same room above the icon as below
  the shortcut and more space between name and shortcut; icon and name sit at the same
  height whether a tile has a shortcut or not, and all tiles take the same turquoise
  tint on hover as the other buttons. The main window's minimum height grows to 690 pt
- Home screen: the background's colour glows drift slowly around the window, like
  bubbles, and lean gently towards the pointer with a little parallax. It only moves
  while the window is in front, and then pauses where it is
- Editor: the drop shadow around the picture is tighter and lighter, just enough to show
  where a white screenshot ends once pasted into an email or a document
- Editor: the Shadow toggle has an icon
- Preferences → Appearance: "Show hints" moved to the Capture area group, since it now
  applies to both selection screens; the magnifier stays with the frozen area

### Fixed
- "Click to snap a window" now works. Before, it only enabled the system tool's
  Space-key window mode, and a click never captured a window
- Clicks during Capture Area are no longer lost or misread on secondary displays or
  when made quickly

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

[Unreleased]: https://github.com/donbuche/quicksnap/compare/v0.5.1...HEAD
[0.5.1]: https://github.com/donbuche/quicksnap/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/donbuche/quicksnap/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/donbuche/quicksnap/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/donbuche/quicksnap/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/donbuche/quicksnap/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/donbuche/quicksnap/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/donbuche/quicksnap/releases/tag/v0.1.0
