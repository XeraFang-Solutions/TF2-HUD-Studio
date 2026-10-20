# Troubleshooting and FAQ

**"TF2 files not loaded" / stock fonts and icons are missing**
Open **Settings** and choose your `…/Team Fortress 2/tf` folder. It's the folder with `tf2_misc_dir.vpk` in it.

**Windows says "Windows protected your PC"**
The builds aren't code-signed yet. Click **More info → Run anyway**.

**macOS says the app "can't be opened" or "is damaged"**
Right-click the app, choose **Open** and confirm. You can also run `xattr -dr com.apple.quarantine "TF2 HUD Studio.app"`.

**The AppImage won't start on Linux**
Install FUSE 2 (`sudo apt install libfuse2`, or your distro's package), or run it with `--appimage-extract-and-run`.
On a Steam Deck, run it from desktop mode.

**The window is blank on Linux (NVIDIA or Wayland)**
Start it with `WEBKIT_DISABLE_DMABUF_RENDERER=1 ./TF2-HUD-Studio-….AppImage`.

**"Open in editor" does nothing, or opens the wrong program**
- Check **Settings → Text editor**. If the editor shows as not found, choose its program.
- For a custom program, make sure the argument template includes `{file}`.
- Terminal editors need a terminal emulator. On Linux, set `$TERMINAL` if yours isn't found.

**Element X is in a different place in game**
- Check that the toolbar resolution matches your game's.
- Check the platform in Settings (`[$WIN32]` and similar conditions).
- Check whether TF2 moves the element at runtime: some panels are placed by code.

  If it's still wrong, please file a [render mismatch](https://github.com/XeraFang-Solutions/TF2-HUD-Studio/issues/new?template=hud_render_mismatch.yml).

**I saved something by mistake**
Use undo, then save again. Or restore a previous version from the backups folder; **Settings** shows where it is.

**Does the studio change files I didn't touch?**
No. Edits change only the values you edited, and comments, spacing and conditions are kept. Saving only writes files
that have changes.

**Can I use it on HUDs I didn't make?**
Yes, for your own use. Before you share a modified HUD, check its license.

**Does it work with other Source games?**
It's built and tested for TF2. Other games' HUDs use the same VGUI rules but different panels and files.
