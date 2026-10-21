# How the preview works

## Units
VGUI lays HUDs out in **proportional units**. The screen is always **480 units tall**, and its width is
480 × the aspect ratio (853.3 at 16:9, 768 at 16:10, 640 at 4:3). The studio converts these units to pixels for the
resolution you choose, as the game does.

## Positions (`xpos`, `ypos`)
| Value | Meaning |
| --- | --- |
| `10` | 10 units from the parent's left/top edge |
| `r10` | measured from the **right/bottom** edge: the element's left/top edge sits 10 units in from it (so `r110` with `wide 100` leaves a 10-unit gap) |
| `c-50` | 50 units left/up of the parent's **center** |
| `s0.5`, `cs-0.5`, `rs1` | the number is a multiple of the element's **own** size: `cs-0.5` centers it exactly, `rs1` puts it flush against the right/bottom edge |
| `p0.1`, `rp0.1`, `cp-0.1` | the number is a fraction of the **parent's** size |

## Sizes (`wide`, `tall`)
| Value | Meaning |
| --- | --- |
| `100` | 100 units |
| `f0` | **fill** the parent, minus 0 units |
| `p0.5` | half the parent |
| `o1` | proportional to the **other** axis (`wide o1` = as wide as it is tall) |

## Pinning
`pin_to_sibling "Name"`, with `pin_corner_to_sibling` and `pin_to_sibling_corner` (0–3, or `PIN_TOPLEFT`…), attaches
one of the element's corners to one of the sibling's corners. `xpos` and `ypos` then become offsets that point
**outward from the sibling's corner**. So `-1 -1` on a top-left-to-top-left pin moves the element 1 unit down and
right, which is the usual drop shadow.

## Files and inheritance
- `#base "file.res"` merges another file under this one. **The including file wins**, and nested blocks merge key by key.
- Within one block, the **first** occurrence of a key wins. Duplicate *blocks* both apply, so the later one's values override.
- Lookups follow TF2's search path: your HUD, then `tf/`, then TF2's VPK files. File names are case-insensitive.

## Conditions
`[$WIN32]`, `[$LINUX]`, `[$OSX]`, `[$POSIX]` and negations like `[!$OSX]` are evaluated for the platform chosen in
Settings. Console conditions (`[$X360]`, …) are false. `if_match` blocks are treated as active.

## What's simulated
Some panels only exist in a match, and the preview fills them in:
- player names, health and ammo values, the killfeed, chat, team status and scores come from the chosen scenario;
- class-specific meters follow the *Playing …* class;
- 3D model panels show as placeholder boxes;
- font outlines, blur and drop shadows are close approximations.

Found a difference? Please open a
[render mismatch report](https://github.com/XeraFang-Solutions/TF2-HUD-Studio/issues/new?template=hud_render_mismatch.yml).
