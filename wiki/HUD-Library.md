# HUD library

The library is a folder where you keep all your HUDs. From the library, one click changes which HUD TF2 loads.

## Set it up
Open **HUD library**: the books icon at the bottom left, or the button on the start screen. Choose a folder that is **not**
inside `tf/custom` (TF2 would load everything in it at once), for example `Documents/TF2 HUDs`. Every HUD in that folder
shows up as a card, with its icon, name, version, author and description from its [[TFHUD manifest|TFHUD-Manifest]].

## Link or copy
**Settings → HUD library → Install HUDs as** decides how a HUD gets into `tf/custom`:

| Mode | What happens | Good for |
| --- | --- | --- |
| **Link** (default) | `tf/custom/<name>` becomes a junction (Windows) or symlink (Linux, macOS) pointing at the library folder. | Working on HUDs: nothing is copied, and every save in HUD Studio is what TF2 loads. Use **Reload HUD** in the Game panel to see it. |
| **Copy** | The HUD is copied into `tf/custom`. | A fixed snapshot; install again to update it. |

Junctions need no admin rights or Developer Mode. Windows can only make them on local drives, so if linking fails, use Copy.

## Swapping HUDs
- **Use this HUD** installs the HUD and takes every other HUD out of `tf/custom`. HUD Studio lists what will be removed and asks first. Folders that aren't HUDs (configs, hitsounds, sprays) are never touched.
- **Install** adds the HUD without removing anything else.
- **Uninstall** removes the HUD from `tf/custom`.

Nothing is ever deleted. A link is just a pointer, so removing it leaves the library copy alone. A real folder is moved into
HUD Studio's backups folder (`…/backups/_custom`).

Restart TF2, or run `hud_reloadscheme`, after swapping.

## Bringing HUDs in
- **In tf/custom, not in the library** lists HUD folders already installed. **Move to library** moves one into the library and installs it back as a link (or a copy), so the game still loads it.
- **Add a HUD folder…** copies a downloaded or extracted HUD into the library. If the folder holds one HUD folder inside it, as many zips do, that inner folder is used.
- **New HUD** creates new HUDs in the library when one is set.
