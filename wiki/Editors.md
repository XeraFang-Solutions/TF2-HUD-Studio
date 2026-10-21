# Editors

## The built-in source editor
Every file can be edited as text: choose **Source** in the view switch. It has KeyValues highlighting, bracket
matching and search. Edits made in the designer and in the source editor share one undo history.

## Your own text editor
Choose it in **Settings → Text editor**:

| Choice | What happens |
| --- | --- |
| **System default app** | The file opens in whatever your OS uses for it. |
| **Found on this computer** | Editors the studio detected. The hint shows the program it found and whether it can jump to a line. |
| **Other known apps** | About 45 editors the studio knows how to launch: VS Code, VSCodium, Cursor, Windsurf, Zed, Sublime Text, Notepad++, Notepad3, EmEditor, UltraEdit, Kate, gedit, Geany, JetBrains IDEs, TextMate, BBEdit, CotEditor, Vim, Neovim, Emacs, Helix, Micro, nano and more. If the one you pick wasn't found, choose its program. |
| **Custom program…** | Any program, with an argument template. `{file}` and `{line}` are replaced. For example: `-g "{file}:{line}"`, `+{line} "{file}"`, `--line {line} "{file}"`. |

Then use:
- the **↗** button next to Source, or <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>E</kbd>, which opens the current file at the cursor's line;
- right-click a file or tab and choose **Open in …**;
- right-click a problem and choose **Open in … at line N**.

Terminal editors such as Vim, Neovim, nano, Micro, Helix, Kakoune and `emacs -nw` open in a new terminal window.
On Linux the terminal comes from `$TERMINAL`, then `x-terminal-emulator`, GNOME Terminal, Konsole and others. When you
save in your editor, the studio notices the change and reloads the file.

## Your own image editor
Choose it in **Settings → Image editor**. The list includes GIMP, Photoshop, Photoshop Elements, Krita, Paint.NET,
Affinity Photo, Pixelmator Pro, Acorn, Preview, PaintShop Pro, Clip Studio Paint, Aseprite, Pinta, Paint, Inkscape,
PhotoFiltre, IrfanView, XnView MP, MyPaint, KolourPaint, VTFEdit and VTFEdit Reloaded. You can also choose any program.

In **Images** (the picture icon), pick a material and choose one of these:
- **Edit in …** opens your HUD's own `.vtf` (or `.png`/`.tga`) file. Use this with an editor that reads VTF, such as
  VTFEdit, or GIMP or Photoshop with a VTF plugin.
- **Edit a PNG copy** decodes the texture, including TF2's stock ones, to a PNG in your temp folder and opens that.
  When you're done, convert it back to VTF (for example with VTFEdit) and save it into your HUD's `materials/` folder.

After editing a PNG copy, **Save PNG copy into HUD** writes it back as a VTF. If the texture is one of TF2's own, it's saved in
your HUD as an override.

## Importing images
**Images → Import** turns a picture into a HUD material, `materials/vgui/<name>.vtf` and a matching `.vmt`. The default name,
`replay/thumbnails/<file>`, can be used in an element's `image` straight away.

| Input or option | Needs |
| --- | --- |
| PNG | Nothing: HUD Studio writes a lossless BGRA8888 VTF without mipmaps itself. |
| TGA, JPG, BMP, PSD, WebP, GIF | [ImageMagick](https://imagemagick.org), which converts to PNG first. |
| Compress (DXT5) | VTFCmd, which comes with VTFEdit and VTFEdit Reloaded. |
| Scale to a power of two | Nothing. |

## Other tools
The **Tools** panel (wrench icon) finds and launches **Crowbar**, **Blender**, **VTFEdit** and **VTFEdit Reloaded**. It also locates the
command-line converters **VTFCmd** and **ImageMagick** that image import uses. If one isn't found, press **Choose…** and point
at its program.
