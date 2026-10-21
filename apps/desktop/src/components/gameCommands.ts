// Console commands for testing a HUD in a local game (itemtest or any map with sv_lan 1).
// Commands marked `cheat` are sent with `sv_cheats 1` first. Condition numbers are TF2's addcond ids.

export type GameCmd = { label: string; cmd: string; cheat?: boolean; hint?: string };
export type GameCmdGroup = { id: string; title: string; note?: string; items: GameCmd[] };

const cond = (label: string, id: number, hint?: string): GameCmd => ({ label, cmd: `addcond ${id}`, cheat: true, hint: hint ?? `addcond ${id}` });

export const GAME_COMMAND_GROUPS: GameCmdGroup[] = [
  {
    id: 'hud',
    title: 'HUD',
    items: [
      { label: 'Reload HUD', cmd: 'hud_reloadscheme', hint: 'Re-reads every HUD file' },
      { label: 'Hide / show HUD', cmd: 'toggle cl_drawhud' },
      { label: 'VGUI panel tree', cmd: 'toggle vgui_drawtree', hint: 'Lists every panel on screen; great for finding names' },
      { label: 'Panel outlines', cmd: 'toggle vgui_drawfocus', hint: 'Highlights the panel under the mouse' },
      { label: 'FPS counter', cmd: 'toggle cl_showfps' },
      { label: 'Net graph', cmd: 'toggle net_graph' },
      { label: 'Damage numbers', cmd: 'toggle hud_combattext' },
      { label: 'Minimal HUD', cmd: 'toggle cl_hud_minmode', hint: 'Tests the HUD’s minmode layouts' },
    ],
  },
  {
    id: 'player',
    title: 'You',
    items: [
      { label: 'Respawn', cmd: 'kill', hint: 'Dies and respawns: tests the respawn timer and freezecam' },
      { label: 'Explode', cmd: 'explode' },
      { label: 'Take 50 damage', cmd: 'hurtme 50', cheat: true },
      { label: 'Take 100 damage', cmd: 'hurtme 100', cheat: true, hint: 'Shows the low-health warning on most classes' },
      { label: 'Overheal +150', cmd: 'hurtme -150', cheat: true },
      { label: 'Refill ammo & health', cmd: 'impulse 101', cheat: true },
      { label: 'God mode', cmd: 'god', cheat: true },
      { label: 'Buddha (can’t die)', cmd: 'buddha', cheat: true },
      { label: 'Noclip', cmd: 'noclip', cheat: true },
      { label: 'Third person', cmd: 'thirdperson', cheat: true },
      { label: 'First person', cmd: 'firstperson' },
      { label: 'Spectate', cmd: 'spectate', hint: 'Tests the spectator HUD' },
      { label: 'Join RED', cmd: 'jointeam red' },
      { label: 'Join BLU', cmd: 'jointeam blue' },
    ],
  },
  {
    id: 'conditions',
    title: 'Conditions',
    note: 'Adds a condition to you. Use “Clear conditions” to remove them all.',
    items: [
      cond('Crits', 11),
      cond('Übercharge', 5),
      cond('Mini-crits (Buff Banner)', 16),
      cond('Marked for death', 30),
      cond('Jarate', 24),
      cond('Mad Milk', 27),
      cond('Bleeding icon', 101, 'addcond 101: shows the bleed icon'),
      cond('On fire', 22),
      cond('Healing', 21),
      cond('Cloaked', 64),
    ],
  },
  {
    id: 'mannpower',
    title: 'Mannpower power-ups',
    items: [
      cond('Strength', 90),
      cond('Haste', 91),
      cond('Regen', 92),
      cond('Resist', 93),
      cond('Vampire', 94),
      cond('Reflect', 95),
      cond('Precision', 96),
      cond('Agility', 97),
      cond('Knockout', 103),
      cond('King', 109),
      cond('Plague', 110),
      cond('Supernova', 111),
    ],
  },
  {
    id: 'bots',
    title: 'Bots',
    note: 'Puppet bots work on any map, itemtest included.',
    items: [
      { label: 'Add RED Heavy', cmd: 'bot -team red -class heavyweapons', cheat: true },
      { label: 'Add RED Medic', cmd: 'bot -team red -class medic', cheat: true },
      { label: 'Add BLU Soldier', cmd: 'bot -team blue -class soldier', cheat: true },
      { label: 'Add BLU Spy', cmd: 'bot -team blue -class spy', cheat: true },
      { label: 'Add BLU Sniper', cmd: 'bot -team blue -class sniper', cheat: true },
      { label: 'Bots copy you', cmd: 'toggle bot_mimic', cheat: true, hint: 'Bots repeat your movement and buttons' },
      { label: 'Bots fire', cmd: 'toggle bot_forceattack', cheat: true },
      { label: 'Kick all bots', cmd: 'bot_kick all', cheat: true },
    ],
  },
  {
    id: 'round',
    title: 'Round & game mode',
    items: [
      { label: 'Skip waiting for players', cmd: 'mp_waitingforplayers_cancel 1' },
      { label: 'Restart round', cmd: 'mp_restartgame 1' },
      { label: 'RED wins', cmd: 'mp_forcewin 2', cheat: true, hint: 'Shows the win panel' },
      { label: 'BLU wins', cmd: 'mp_forcewin 3', cheat: true },
      { label: 'Tournament mode', cmd: 'mp_tournament 1; mp_tournament_restart', hint: 'Tests the tournament HUD' },
      { label: 'Tournament off', cmd: 'mp_tournament 0; mp_tournament_restart' },
    ],
  },
  {
    id: 'maps',
    title: 'Maps',
    note: 'Loads the map as a local game (sv_lan 1).',
    items: [
      { label: 'itemtest', cmd: 'sv_lan 1; map itemtest' },
      { label: 'Capture the flag (2Fort)', cmd: 'sv_lan 1; map ctf_2fort' },
      { label: 'Control points (Badlands)', cmd: 'sv_lan 1; map cp_badlands' },
      { label: 'King of the hill (Harvest)', cmd: 'sv_lan 1; map koth_harvest_final' },
      { label: 'Payload (Badwater)', cmd: 'sv_lan 1; map pl_badwater' },
      { label: 'Payload race (Hightower)', cmd: 'sv_lan 1; map plr_hightower' },
      { label: 'Mann vs. Machine (Coal Town)', cmd: 'sv_lan 1; map mvm_coaltown' },
      { label: 'Mannpower (Foundry)', cmd: 'sv_lan 1; map ctf_foundry' },
      { label: 'Training (Walkway)', cmd: 'sv_lan 1; map tr_walkway' },
    ],
  },
];

export const CLASS_CMDS: { label: string; id: string }[] = [
  { label: 'Scout', id: 'scout' },
  { label: 'Soldier', id: 'soldier' },
  { label: 'Pyro', id: 'pyro' },
  { label: 'Demoman', id: 'demoman' },
  { label: 'Heavy', id: 'heavyweapons' },
  { label: 'Engineer', id: 'engineer' },
  { label: 'Medic', id: 'medic' },
  { label: 'Sniper', id: 'sniper' },
  { label: 'Spy', id: 'spy' },
];

// "Clear conditions" removes every condition the buttons above can add
const ALL_CONDS = GAME_COMMAND_GROUPS.filter((g) => g.id === 'conditions' || g.id === 'mannpower').flatMap((g) => g.items.map((i) => /^addcond (\d+)$/.exec(i.cmd)?.[1]).filter(Boolean));
GAME_COMMAND_GROUPS.find((g) => g.id === 'conditions')!.items.push({ label: 'Clear conditions', cmd: ALL_CONDS.map((n) => `removecond ${n}`).join('; '), cheat: true, hint: 'Removes the conditions listed here' });

/** The command actually sent for an entry. */
export function commandFor(c: GameCmd): string {
  return c.cheat ? `sv_cheats 1; ${c.cmd}` : c.cmd;
}
