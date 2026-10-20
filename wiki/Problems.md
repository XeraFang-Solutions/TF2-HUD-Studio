# Problems

The Problems panel (the warning icon on the left) checks every file TF2 loads from your HUD, plus everything those
files pull in through `#base`. Click a problem to jump to it, or right-click it to open the file in your own editor at
that line.

| Message | What it means | What to do |
| --- | --- | --- |
| *Syntax error* (unexpected `}`, missing value…) | The file won't parse. TF2 may stop reading it at that point. | Fix the braces or quotes on that line. |
| `#base file '…' not found — TF2 skips it` | The file named in `#base` doesn't exist. | Usually harmless. Many HUDs use optional override files this way. Fix the path if it's a typo. |
| `'X' is defined again here` | The same block appears twice in one file. TF2 applies both, so the later values override the earlier ones. | Merge the blocks if you didn't mean it. |
| `color '…' is not defined in the scheme` | A `.res` file uses a named color the scheme doesn't have. The game draws it white. | Add the color in [[Scheme]], or fix the name. |
| `font '…' is not defined in the scheme` | The font name doesn't exist, so the game uses a default font. | Add it in [[Scheme]], or fix the name. |
| `image '…' not found` | No `.vmt` with that name in your HUD or in TF2. | Check the path. Image names are relative to `materials/vgui/`. |
| `pinned to '…', which isn't a sibling` | The `pin_to_sibling` target isn't in this file. | Fine if the game creates that panel itself. Otherwise check the name. |
| `references unknown event` | `RunEvent` or `StopEvent` names an event that doesn't exist. | Fix the name, or add the event. |
| `Event '…' is ignored` | An earlier file in the manifest already defines this event. | Move or rename it. See [[Animations]]. |

Errors are red, warnings yellow and notes grey. Only errors are likely to break the HUD in game.
