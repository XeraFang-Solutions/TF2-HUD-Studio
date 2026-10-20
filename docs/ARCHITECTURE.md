# Architecture

TF2 HUD Studio is a [Tauri 2](https://v2.tauri.app) app. A Rust backend understands HUD files. A React + TypeScript
frontend lays them out and draws them the way VGUI does.

```
┌────────────────────── apps/desktop/src (React) ──────────────────────┐
│ Canvas · Inspector · Elements · Scheme · Animations · Source · Assets│
│        vgui/layout.ts  vgui/anim.ts  vgui/scheme.ts  vgui/model.ts   │
└───────────────▲─────────────────────────────────────┬────────────────┘
                │ invoke() / JSON                     │
┌───────────────┴──── apps/desktop/src-tauri ─────────▼────────────────┐
│ lib.rs (commands) · session.rs (buffers, undo) · settings.rs         │
│ editors.rs (external text/image editors) · tools.rs (Crowbar, …)     │
└───────────────▲──────────────────────────────────────────────────────┘
                │
┌───────────────┴──── crates/hud-core ─────────────────────────────────┐
│ vfs (HUD → tf/custom → tf → VPKs) · resolve (#base/#include)         │
│ scheme · anim · images + vtf + vpk · fontmeta · lint · project       │
└───────────────▲──────────────────────────────────────────────────────┘
┌───────────────┴──── crates/hud-syntax ───────────────────────────────┐
│ lossless KeyValues lexer/parser · tree with spans · minimal edits    │
└──────────────────────────────────────────────────────────────────────┘
```

## hud-syntax

A lossless KeyValues parser. It keeps every byte (comments, whitespace, quotes, conditions, odd casing), so any file
can be parsed and written back unchanged. Edits are text splices at the spans of the values they touch: set a value,
add a key or block, remove a key, rename a block. A file you only change one value in has a one-value diff.

`tests/solar_light_corpus.rs` round-trips every file of a real HUD (`HUD_CORPUS_PATH`) byte-for-byte.

## hud-core

| Module | What it does |
| --- | --- |
| `vfs` | Layered, case-insensitive lookups the way TF2 sees a custom HUD: the project, then the `tf/` folder, then `tf2_misc_dir.vpk` / `tf2_textures_dir.vpk`. |
| `vpk` | Read-only VPK v1/v2 reader. |
| `resolve` | `#base` / `#include` merging like `KeyValues::MergeBaseKeys`. It records which file each value came from, plus missing bases and cycles. |
| `scheme` | Client scheme colors, fonts, borders and custom font files, resolved through `#base`. |
| `anim` | Parses `hudanimations_*.txt` and the manifest. Load order decides which event wins. |
| `images`, `vtf` | Turns VGUI image names into `.vmt` → `.vtf` files and decodes them to RGBA (DXT1/3/5, BGRA, …). |
| `fontmeta` | Reads TTF/OTF names and vertical metrics, so text sits where the game puts it. |
| `lint` | The Problems list. It only covers files TF2 loads, plus anything they reach through `#base`. |
| `project` | Scanning, search, new-HUD scaffolding, installing to `tf/custom`, zip export and Steam library discovery. |

## The Tauri backend

- **`lib.rs`:** every `#[tauri::command]` the UI calls. All paths are checked to be inside the open HUD or the TF2 folder.
- **`session.rs`:** the open project, in-memory file buffers with undo/redo across files, change detection and caches.
- **`settings.rs`:** versioned JSON settings in the OS config folder: TF2 path, platform, editors, tools and UI options.
- **`editors.rs`:** catalogs of text and image editors for each OS. It handles detection (PATH, install folders,
  `.app` bundles) and launching, with line-number argument templates and terminal editors.
- **`tools.rs`:** optional asset tools (Crowbar, Blender).

## The frontend

- **`vgui/model.ts`** turns resolved files into an element tree. It merges duplicate blocks, applies the active
  conditions, and adds runtime-only panels for the whole-HUD view.
- **`vgui/layout.ts`** is the VGUI layout engine:
  - proportional scaling;
  - `r`/`c`/`s`/`p`/`cs`/`rs` positions and `f`/`p`/`o` sizes;
  - `pin_to_sibling` corners;
  - auto-resize and `zpos`.

  It is pure TypeScript and is tested with `node --test` (`layout.test.ts`).
- **`vgui/anim.ts`** samples animation commands over time with the engine's interpolators (`anim.test.ts`).
- **`components/Canvas.tsx`** draws elements and handles selection, dragging, resizing, snapping and nudging. Each
  edit is turned into the smallest change to the right file: the element's own file, or an override when the value
  was inherited.
- **`store.ts`** holds the shared app state, with a small hook-based store and the edit helpers.
- **`components/Select.tsx`** contains the themed `Select` and `SuggestInput` that replace the native popups.

## Data flow for an edit

1. The user drags an element. The canvas works out the new `xpos` in the value's original notation (for example `c-120`).
2. `editElements` asks the backend to set that key in the file and block where the value is defined. If that's a
   stock or `#base` file, the change goes into the file that inherits it instead.
3. `hud-syntax` splices the text. The session records an undo step and marks the buffer dirty.
4. The model is rebuilt from the new text, and the canvas re-renders.
5. **Save all** writes the dirty buffers (backing up the old ones first) in their original encoding.
