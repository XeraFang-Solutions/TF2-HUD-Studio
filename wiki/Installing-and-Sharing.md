# Installing and sharing

## Install into TF2
Click **Install** (top right), choose a folder name and click **Install to TF2**. The studio copies your HUD into
`tf/custom/<folder name>`:
- if a folder with that name already exists, it's moved to the backups folder first, so nothing is lost;
- your working copy stays where it is, so you can keep editing and installing again;
- if your HUD already lives in `tf/custom`, TF2 loads it directly and you only need to save.

In TF2, restart the game, or run these in the console to reload most changes:
```
hud_reloadscheme
```
Some changes (new fonts, `hudlayout.res`, menus) need a full restart.


## Export a zip
Use **Export .zip…** in the same *Install & share* dialog to make a zip with one top-level folder. That's the layout people expect when
they drop a HUD into `tf/custom`.

## Before you share
- Check [[Problems]] for errors.
- Add an `info.vdf` with your HUD's name (the New HUD wizard creates one).
- Include the licenses of any fonts you ship.
- Don't ship Valve's files you didn't change. Overrides only need the parts you edited.
