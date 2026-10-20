# Getting started

## 1. Download

Get the file for your system from the [latest release](https://github.com/XeraFang-Solutions/TF2-HUD-Studio/releases/latest):

| System | File | Notes |
| --- | --- | --- |
| Windows 10/11 | `…-windows-x64.exe` | If SmartScreen warns you, click **More info → Run anyway**. The build isn't signed yet. |
| Linux, Steam Deck | `…-linux-x86_64.AppImage` | Run `chmod +x` on it, then run it. It needs `libfuse2`, or run it with `--appimage-extract-and-run`. |
| macOS 11+ | `…-macos-universal.zip` | Unzip it. The first time, right-click the app and choose **Open**. |

Nothing gets installed. The app is one file, and your settings are kept in your user profile.

## 2. Point it at TF2

On first launch the studio looks through your Steam libraries for Team Fortress 2. If it can't find it, open
**Settings** (the sliders icon, bottom left) and choose the `tf` folder, for example
`C:\Program Files (x86)\Steam\steamapps\common\Team Fortress 2\tf`.

With TF2 found, the studio can:
- show TF2's own fonts and icons (read directly from the game's VPK files);
- fall back to TF2's default HUD files for anything your HUD doesn't override;
- install your HUD into `tf/custom`.

## 3. Open or create a HUD

- **Open:** choose the HUD's root folder, the one containing `info.vdf`, `resource/` and `scripts/`.
- **New:** pick a name and a starting point:
  - **Start from TF2's default HUD:** copies the default scheme, `hudlayout.res` and the main element files (health,
    ammo, class, über, damage, target ID) so you can restyle them.
  - **Empty HUD:** just `info.vdf` and an animation file. Everything looks like TF2's default HUD until you edit an
    element; its file is copied in the first time you do.
- **Command line:** pass a folder to open it at startup, e.g. `TF2-HUD-Studio.exe "D:\huds\myhud"`.

## 4. Find your way around

| Area | What it's for |
| --- | --- |
| Left bar | Files, Elements, Search, Problems, Images. Tools and Settings are at the bottom. |
| Tabs | Each open file. Right-click a tab to close it or open it in your own editor. |
| View switch | **Design**, **Scheme** or **Timeline** (depending on the file), and **Source**. |
| Canvas | The live preview. The toolbar picks the resolution, zoom, grid and snapping. |
| Inspector | Every property of the selected element, and where each value comes from. |

Open `scripts/hudlayout.res` to see the **whole HUD** the way the game puts it together. The *Playing …* dropdown
picks which class the preview shows.

## 5. Save and try it

Press <kbd>Ctrl</kbd>+<kbd>S</kbd> to save every changed file. A backup is made first. Then click **Install** (top right),
and in TF2 either restart or run `hud_reloadscheme` in the console. See [[Installing and Sharing]].
