# Animations

Open any `scripts/hudanimations_*.txt` file. Choose **Timeline** to see its events.

## Load order
TF2 reads `hudanimations_manifest.txt` from top to bottom, and **the first file that defines an event wins**. The
panel shows the order, and greys out events that are shadowed by an earlier file. [[Problems]] lists them too.

## Timeline
- Pick an event to see one row per element and property. Each command is a bar coloured by its interpolator:
  Linear, Accel, Deaccel, Spline, Pulse, Flicker, Gain, Bias or Bounce.
- Commands from events started with `RunEvent` appear in the timeline too, nested under the event that starts them.
- Click a bar to jump to that command in the source editor.
- **Play** runs the event on the HUD above, and **Stop** ends it. Turn on **Loop animation playback** in Settings to
  repeat it.
- **Edit text** jumps to the event in the source editor. For an event that comes from TF2's files, **Override in my
  HUD** copies it into your HUD so you can change it.

## Tips
- Colors in `Animate … FgColor "Name"` must exist in the scheme, or the game uses white. They're flagged in [[Problems]].
- `[$X360]` lines are skipped, as on PC.
