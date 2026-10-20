# Designer

The designer is the canvas you get for any `.res` file with panels in it.

## Toolbar
- **Resolution:** common 16:9, 16:10, 4:3, 5:4 and 21:9 sizes, plus Steam Deck. HUDs are laid out in proportional
  units, so changing the aspect ratio shows how `r`, `c` and `s` positions move.
- **Zoom:** fit, or 50–200 %. <kbd>Ctrl</kbd>+<kbd>0</kbd> fits the canvas to the window.
- **Show hidden:** also draws elements with `visible 0`. The game turns many of these on at runtime.
- **Grid / Snap:** snaps moves to 1, 2, 5 or 10 units. Edges and centers of neighbouring elements always snap. Hold <kbd>Alt</kbd> to drag freely.
- **Composite:** shows a `.res` file inside its `hudlayout.res` container, the way the game does.

## Selecting
- **Click** an element, or pick it in the **Elements** tree. <kbd>Shift</kbd>- or <kbd>Ctrl</kbd>-click adds to the selection.
- **Drag on an empty area** to select everything inside a box.
- <kbd>Esc</kbd> clears the selection.
- Big full-screen panels can sit on top of small ones. Use the Elements tree to reach the elements underneath.

## Moving and resizing
- **Drag** to move, or use the **arrow keys** to nudge by 1 unit (<kbd>Shift</kbd> for 10).
- **Resize** with the handles. <kbd>Shift</kbd> keeps the aspect ratio.
- The studio writes values in the notation already used: `r10` stays right-aligned, and `c-50` stays centered.
- **Align tools** in the Inspector line up several elements.

## The Inspector
- Properties are grouped (Identity, Layout, Pinning, Text, Colors, Image, Other) and typed. You get color swatches,
  font, color and border suggestions, alignment and pin-corner dropdowns, and image previews.
- The badge next to each value shows where it comes from:
  - **here:** this file;
  - a file name: a `#base` file;
  - **default:** not set anywhere.

  Click ↺ to remove an override.
- Use **add property** at the bottom to add any key. Suggestions come from the known VGUI properties.

## Editing TF2's own elements
If a value comes from TF2's default HUD files, the studio asks once, copies that file into your HUD, and edits the
copy. Your HUD keeps working when TF2 updates the original, apart from the values you changed.

## Elements
- **Add element** inserts a new control from a template: labels, image panels, progress bars, buttons and more.
- **Duplicate:** <kbd>Ctrl</kbd>+<kbd>D</kbd>.
- **Delete:** <kbd>Delete</kbd>.
- **Rename:** edit `fieldName` in the Inspector. If other elements pin to it, update their `pin_to_sibling` too.

## The whole-HUD view
In `scripts/hudlayout.res`, the canvas loads every element's own file and draws the full HUD. Panels that only appear
in a match get believable stand-ins: the killfeed, chat, the team status bar, the match timer, health and ammo. The
**Playing …** dropdown changes the class, which shows or hides the meters that class has.
