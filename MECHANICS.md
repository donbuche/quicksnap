# Mechanics sheet: rules and values read from the original program

Source: `/Applications/Monosnap.app/Contents/MacOS/Monosnap` (arm64 slice), read by disassembly.
Every number below was looked up in the code; anything marked **estimated** was searched for
and not found, so it is my own choice.

## Settings window tabs

`-[MSSettingsWindow toolbarDefaultItemIdentifiers:]` @ 0x100051ad8 builds the toolbar as:

    flexible space, General, Advanced, Appearance, Hotkeys, Fonts, Account, Feedback, flexible space

(identifier strings at 0x100b46ab8 … 0x100b46b78). The "AI" item (0x100b46bf8) exists in
`toolbar:itemForItemIdentifier:` @ 0x1000513d4 but is not in the default list.

The person asked to drop **the two last tabs → Account and Feedback**. The rebuild keeps
General, Advanced, Appearance, Hotkeys, Fonts in that order, centred between flexible spaces.

## General tab — `-[MSSettingsGeneralView populateComponents]` @ 0x10010e9e0
Rows (top → bottom): After screenshot (popup), Sound effects (check), After upload (popup),
Open in browser, Short links, Apply text recognition, External Image Editor (popup +
"Choose application..."), Status bar / Launch at login, About button.

Defaults — `-[MSGeneralSettingItem initWithKey:]` @ 0x1000d5fa0:
| key | default | range |
|---|---|---|
| after screenshot | 0 (Open editor) | enum 0..6 |
| sound effects | false | |
| after upload | 1 (Copy site link) | enum 0..3 |
| close after upload | true | |
| open in browser | true | |
| short links | false | |
| Apply OCR | false | |
| external image editor | /System/Applications/Preview.app | |
| start at launch | false | |

After-screenshot options (order of popup items @ 0x10010fbd4…0x10010fd90): Open editor, Save
screenshot, Upload screenshot, Open external editor, Copy to clipboard.
After-upload options: Do not copy, Copy site link, Copy direct image link.

## Advanced tab — `-[MSSettingsAdvancedView populateComponents]` @ 0x10012a4a4
Rows (top → bottom): Image File Format (JPEG / PNG / Auto (smaller size)), JPEG Quality slider,
Save folder + Choose..., Autosave screenshots, Shrink Retina snaps, Video File Format
(MPEG / GIF), GIF FPS slider, GIF Quality slider, Autosave to "Movies" folder,
Drag and drop copy (Image / URL / Path), Copy to clipboard (Image / URL / Path),
Filename template + "Apply to reopened files" + tip, Reset all settings + tip.

Defaults — `-[MSAdvancedSettingsItem initWithKey:]` @ 0x1000e8cec:
| key | default | min | max |
|---|---|---|---|
| Image File Format | 1 (PNG) | 0 | 2 |
| JPEG Quality | 100 | 50 | 100 |
| Always Save | false | | |
| Always Save Movie | false | | |
| Shrink Retina | false | | |
| Video File Format | 0 (MPEG) | 0 | 1 |
| GIF Frames per second | 10 | 1 | 30 |
| GIF Image quality | 100 | 10 | 100 |
| ClipboardCopyFlags | 1 (Image) | 0 | 7 (bit 1 Image, 2 URL, 4 Path) |
| DragAndDropCopyFlags | 7 (all) | 0 | 7 |
| Filename Template | `Monosnap %W %Y-%m-%d %H-%M-%S` (brand word replaced by app name) | | |
| Apply template to opened files | false | | |
| Save folder | `~/Pictures/Monosnap` (@ 0x10012baf8; brand word replaced) | | |

Slider labels: quality sliders show `%ld%%`, FPS shows `%ld`
(`restoreVideoSettings` @ 0x10012cc4c).

Filename template tokens (tip string): %Y 4-digit year, %y 2-digit year, %m month, %d day,
%H hour, %M minutes, %S seconds, %W window title, %R 5 random symbols, %T file type.

## Appearance tab — `-[MSSettingsAppearanceView populateComponents]` @ 0x100111c18
Groups (top → bottom): Capture area: Show magnifier, Highlight window under cursor,
Add shadow to window, Click to snap a window, Show hints. Status bar: Show hints. Hide
animation, Hide all notifications. Editor: Show tooltips, Close the editor after uploading,
Use default browser to sign in.

Defaults — `-[MSAppearanceSettingItem initWithKey:]` @ 0x1000f0930:
Dont show notif false, Show Tooltip true, Show Magnifier true, Highlight Windows true,
Window Shadow true, Realtime Crop true, Show Startup Tooltip true, Lite Mode false,
Show tooltip editor true, Show animation progress false, Simple share window false,
Upload button type 0, Image shadow false, shot on click true.

## Hotkeys tab — `-[MSSettingsHotkeysView populateComponents]` @ 0x10011a254
Rows: Capture area, Capture frozen area, Capture area & Upload, Capture fullscreen,
Fullscreen after 10s, Open Latest Image, Record video, Capture previous area,
Open from clipboard; "Video record hotkeys:" Record/Pause, Stop, Arrow, Rectangle,
Fullscreen / Windowed mode. Prompt: "Press hotkey with ⌘/⌃"; empty = "Not set".

Defaults — `-[MSMonosnapHotkeysSettingsItem initWithKey:]` @ 0x10007b9cc, modifier mask
0x180000 = ⌥⌘:
| action | default |
|---|---|
| Capture Area | ⌥⌘5 |
| Capture Frozen Area | ⌥⌘8 |
| Capture Area And Upload | ⌥⌘7 |
| Capture Fullscreen | ⌥⌘6 |
| Capture Fullscreen Delayed | ⌥⌘9, created disabled (call with 0 @ 0x10007bbb4) → shown as Not set |
| Open Editor | ⌥⌘0 |
| Capture screencast | created disabled (@ 0x10007bc4c) → Not set |
| others | Not set |

Delayed fullscreen delay: 10 s (label "Fullscreen after 10s:").

## Fonts tab — Swift `MSSettingsFontsView`
Strings: "Main font:", "Additional fonts:", "Reset fonts", "Attention: some fonts could not
correctly displayed in the editor." Control layout **estimated** (Swift view not decoded).

## Image editor
Defaults — `-[MSImageEditorSettingItem initWithKey:]` @ 0x10006a8a8:
Primitive = arrow, Drawing type = Solid, Color = red (`[NSColor redColor]` @ 0x10006a998),
Shadow = true, Highlight = true, Line Width = 6.0.

Line width slider — `-[MSEditorPanelWidthShadowStrokeView initWithFrame:]` @ 0x10028c5cc:
min 1, max 12, 10 tick marks.

Colour palette — function around 0x1000df338: e504c3, e50404, 53e504, 1f63be, ffea00,
ff7200, 898989, 000000, colour wheel; white ffffff.

Tools and shortcuts (MSHotkeyMap.strings): Arrow ⌘1, Rectangle ⌘2, Line ⌘3, Ellipse ⌘4,
Brush ⌘5, Text ⌘6, Text with arrow ⌘7, Blur ⌘8, Save ⌘S, Copy ⌘C, Paste ⌘V,
External editor ⌘E, Thickness −/+ ⌘[ / ⌘], Rotate ⌘→ / ⌘←, Keyboard shortcuts ⌘/.
Modifier rules (ImageEditorTooltips.strings): ⇧ = 45° lines / square / circle;
⌘ on rectangle/ellipse = filled; ⌥ on rectangle = darken everything outside;
⌘ on pen = highlighter; ⇧ on blur = square area.

## Video
`-[MSVideoSettingItem initWithKey:]` @ 0x10009bc14: Capture Mouse Clicks true,
Capture Mouse Cursor true, Framerate 60 (5..60).

## Values of my own (estimated — looked for in the code, not found)
| value | used | why |
|---|---|---|
| Blur | 3 box-blur passes, radius 8 px | same |
| Shadow & outline | black at alpha 80, offset 1 + w/4 | same |
| Undo depth | 40 steps | same |
| Magnifier (frozen capture) | 15 × 15 screen px shown 8× | same |
| Settings toolbar height | 62 pt | toolbar drawn by the system in the original |

## How the rebuild uses them
- Capture area / fullscreen / delayed use the system capture tool; "Add shadow to window" maps
  to its options, "Sound effects" off = silent capture. A click
  without dragging in Capture area is detected afterwards (last event = mouse up, not a key):
  it snaps the window under the pointer when "Click to snap a window" is on, else the whole
  screen (the system tool itself only snaps windows after pressing Space).
- Capture frozen area freezes the screen and shows a full-screen selection with the magnifier
  and size hints (Appearance settings); its rectangle is remembered for "Capture previous area".
- Anything that needs an account (upload, links, sign-in) is shown greyed out, because the
  Account tab was removed on request.
- Brand name replaced with "QuickSnap" in visible text, the default template and save folder.

## Additions requested later (own design, not from the original)
- Light / dark / automatic theme; rounded, semi-opaque floating panels over a blurred copy of the picture.
- App icon: a lens with a seven-colour aperture diaphragm, drawn in code.
- Text: typed in place, its own size (10–120 px, default 24) and style (Plain, Outline, Box);
  a Select tool moves, re-edits (double-click) or deletes items; the style bar always edits the
  selected item and sets the defaults for new ones.


## Drawing engine ("picasso", C++, decoded through its class tables)
The editor's drawing engine is a C++ library built into the main program (classes
`picasso::ArrowRenderer`, `picasso::TextRenderer`, `picasso::Bubble`, ...). Their function tables
were found from the C++ type names: ArrowRenderer vtable 0x100b3b618, TextRenderer 0x100b33a70,
CommandsHolder 0x100b24188, Arrow shape 0x100b212a0, Text shape 0x100b26028.

### Arrow — `sub_1007b2d00` (ArrowRenderer geometry), shading near 0x1007b1d7c
- Length L from tail to tip; arrows with L < 2 px are not drawn ("Can't draw an extremely small arrow").
- Unit k = 0.7 × line width (constant 0x1009013f0).
- Short arrows shrink: f = 12k / L when L < 12k, else 1.
- Head points, measured back from the tip and to each side, × k / f (constants 0x10090d728,
  0x10090d730, 0x10090d738): (7.92, ±1.14) shaft join, (5.7, ±2.0), (4.7, ±2.65) widest, then tip.
- Tail half-width: min(width / 5 / f, width / 9).
- Two-tone shading: one half of the arrow uses colour × 1.04 (capped at 1; 0x3f851eb8), the other
  half that value − 0.08 (0xbda3d70a); which half depends on the direction (|angle| ≤ 90°).
  Rebuild: the half on the upper side of the screen is the lighter one (side choice inferred).

### Text — typeset setup at 0x10079a940..0x10079a9b8 (same rule at 0x10079add0 and 0x10079ae78)
- Font size = round(3.5 × line width), at least 4, rounded down to an even number. Width 6 → 20 px.
- Outline thickness = max(1, line width / 2.6) (constant 0x100907e3c).
- Outline colour (0x10079a1a8): black when r + g + b > 2.82353 (0–1 per channel, constant
  0x100907d84), otherwise white. Text is drawn as an outline texture plus a letter texture.
- The rebuild keeps its separate text-size slider (the person asked for it); it starts from this
  formula and follows it whenever the line width is changed on a text.

### Text with arrow — `picasso::Bubble` (sub_10076e990: "[Bubble] arrow_ / text_")
- It is an arrow plus an outlined text, not a box. The arrow leaves the text at
  centre ± 1.3 × half-size on the axis facing the pointed spot (code at 0x10076fd84, constants
  ±1.3 at 0x1009044e0 / 0x1009044e4; up / down / left / right chosen by comparing dx and dy).

### Crop highlighter — 0x100775c28
- Outside the crop is covered with colour (0, 0, 0, 0.3) (constant 0x100904c20).

### Searched, not settled (still estimated)
- Blur: done on the graphics card by `BlurProgram` with uniforms seed, x_min/x_max/y_min/y_max and
  buffer_width/height (0x1007b0764..0x1007b0820), i.e. a randomised sampling blur inside the
  rectangle. The shader text isn't stored readably, so the strength stays estimated (box blur, radius 8).
- Shape shadows: drawn by `ShadowProgram` with a `shadow_color` uniform (0x1007602f4); the offset and
  colour weren't found as constants (still estimated).
- Undo: `CommandsHolder` has no size limit in the code; the rebuild keeps 40 steps because each of
  its steps stores a full copy of the picture.
