# Testing in TF2

HUD Studio can start TF2 for you, show the game's console, and send it commands. Saving a HUD can reload it in game straight away.

## Launching
Use the **Play** button (top right), or **Launch TF2** in the **Game** panel (the gamepad icon). **Steam must be running.**

| System | What starts |
| --- | --- |
| Windows | `tf_win64.exe` (64-bit, the standard build). From Play's menu, or Settings, you can pick `tf.exe` (32-bit, legacy). Settings can also hand the launch to Steam instead. |
| Linux / Steam Deck | Steam starts TF2 (64-bit only). Steam can be on your PATH or the Flatpak. |
| macOS | Not possible: Valve ended TF2's Mac support in 2024. |

### Start in itemtest
On by default. TF2 starts as a quick local game on `itemtest`, with:

```
-window -noborder -w <width> -h <height> -noipx -nosteamcontroller -softparticlesdefaultoff -nohltv -nojoy -enablefakeip
+sv_lan 1 +map itemtest
```

The width and height come from your display the first time you launch. Change them in **Settings → Launching TF2** (or
press **Use my display size** there). Extra launch options of your own go in the same place. Switches like `-novid` are placed
before the `+` commands, and `+` commands keep their values (`+exec my config.cfg`).

## The Game panel
- **Log**: the game's console, read from `tf/console.log` (`-condebug -conclearlog`). Filter it with the box above.
- **Command line**: type any console command; ↑ and ↓ go through what you've typed before.
- **Reload HUD**: runs `hud_reloadscheme`. With **Reload HUD after saving** on, every save does this for you. That only helps when the HUD is installed as a **link** (or opened straight from `tf/custom`), because a copy doesn't change when you save.

### The remote console
Commands go through TF2's remote console. HUD Studio launches TF2 with:

```
-usercon -port <free port> +ip 127.0.0.1 +rcon_password <random password> +net_start
```

- The password is generated for every launch and never saved. The game's command line in the log shows it as `••••••`.
- The console listens on `127.0.0.1` only, so nothing on your network can reach it.

Turn the remote console off in Settings if you don't want it.

## Test commands
The Game panel groups buttons for things a HUD has to handle:

| Group | Buttons |
| --- | --- |
| Class | Join any class (`joinclass`). |
| HUD | Reload HUD, hide/show HUD, **VGUI panel tree** (`vgui_drawtree`, shows every panel's name), panel outlines, FPS, net graph, damage numbers, minimal HUD. |
| You | Respawn, explode, take damage, overheal, refill, god mode, buddha, noclip, third/first person, spectate, join RED/BLU. |
| Conditions | Crits (11), Übercharge (5), Buff Banner (16), marked for death (30), Jarate (24), Mad Milk (27), bleed icon (101), on fire (22), healing (21), cloak (64), and *Clear conditions*. |
| Mannpower power-ups | Strength 90, Haste 91, Regen 92, Resist 93, Vampire 94, Reflect 95, Precision 96, Agility 97, Knockout 103, King 109, Plague 110, Supernova 111. |
| Bots | Add puppet bots (`bot -team … -class …`), make them copy you (`bot_mimic`) or fire, kick them all. |
| Round & game mode | Skip waiting for players, restart, RED/BLU wins (win panel), tournament mode on/off. |
| Maps | itemtest, 2Fort, Badlands, Harvest, Badwater, Hightower, Coal Town (MvM HUD), Foundry (Mannpower) and Walkway. All load as a local game (`sv_lan 1`). |

Buttons with a red dot first turn on `sv_cheats 1`, which only works in a local game such as itemtest. Hover a button to see the
exact command it sends.
