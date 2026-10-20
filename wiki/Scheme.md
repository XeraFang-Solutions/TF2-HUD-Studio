# Scheme

Open `resource/clientscheme.res` (or any scheme file) and choose **Scheme** in the view switch.

## Colors
- Every named color in the scheme and its `#base` files, with a swatch, an RGBA field and an alpha slider.
- The badge shows which file defines the color. Edits go to that file.
- **Add color** adds a new color to the HUD's colors file (the button names the file it will use).
- Colors used in `.res` files and animations but not defined anywhere appear in [[Problems]].

## Fonts
- Each font and its variants, with their size (`tall`).
- The preview uses the HUD's own font files, listed under `CustomFontFiles`, with their real metrics. Fonts from TF2
  itself come from the game.

## Borders
- Every border the scheme defines, and the file it comes from. Edit the details in the source view.

## Scheme files
- The list of files that make up the scheme, in `#base` order. Later files override earlier ones, but the file that
  includes them has the final say, which is how the engine merges them.
