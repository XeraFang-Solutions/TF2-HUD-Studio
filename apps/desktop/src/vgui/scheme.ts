// Scheme lookups for the preview: colors, fonts (with VGUI "tall" → CSS size conversion), borders and label text.
import type { FontData, FontDef, RNode, Scheme } from '../api.ts';

export type RGBA = [number, number, number, number];

export function parseRGBA(raw: string | undefined): RGBA | null {
  if (!raw) return null;
  const parts = raw.trim().split(/\s+/);
  if (parts.length < 3 || parts.length > 4) return null;
  const out: number[] = [];
  for (const p of parts) {
    const m = /^-?\d+/.exec(p);
    if (!m) return null;
    out.push(Math.max(0, Math.min(255, parseInt(m[0], 10))));
  }
  if (out.length === 3) out.push(255);
  return out as RGBA;
}

export class SchemeIndex {
  colors = new Map<string, string>();
  fonts = new Map<string, FontDef>();
  borders = new Map<string, RNode>();
  scheme: Scheme | null;
  constructor(scheme: Scheme | null) {
    this.scheme = scheme;
    for (const c of scheme?.colors ?? []) if (!this.colors.has(c.name.toLowerCase())) this.colors.set(c.name.toLowerCase(), c.raw);
    for (const f of scheme?.fonts ?? []) if (!this.fonts.has(f.name.toLowerCase())) this.fonts.set(f.name.toLowerCase(), f);
    for (const b of scheme?.borders ?? []) if (!this.borders.has(b.key.toLowerCase())) this.borders.set(b.key.toLowerCase(), b);
  }
  color(raw: string | undefined): RGBA | null {
    if (!raw) return null;
    let v: string | undefined = raw;
    for (let i = 0; i < 5 && v; i++) {
      const direct = parseRGBA(v);
      if (direct) return direct;
      v = this.colors.get(v.trim().toLowerCase());
    }
    return null;
  }
  css(raw: string | undefined, alphaMul = 1): string | undefined {
    const c = this.color(raw);
    return c ? rgbaCss(c, alphaMul) : undefined;
  }
  /** Font variant for a screen height, honoring "yres" ranges like the engine does. */
  font(name: string | undefined, screenPx: number): Record<string, string> | null {
    if (!name) return null;
    const def = this.fonts.get(name.trim().toLowerCase());
    if (!def || def.variants.length === 0) return null;
    const ranged = def.variants.find((v) => {
      const y = v.props['yres'];
      if (!y) return false;
      const [lo, hi] = y.split(/\s+/).map((n) => parseInt(n, 10));
      return screenPx >= (lo || 0) && screenPx <= (hi || 99999);
    });
    return (ranged ?? def.variants.find((v) => !v.props['yres']) ?? def.variants[0]).props;
  }
}

export function rgbaCss(c: RGBA, alphaMul = 1): string {
  return `rgba(${c[0]},${c[1]},${c[2]},${((c[3] / 255) * alphaMul).toFixed(3)})`;
}

export function hexOf(c: RGBA): string {
  return '#' + c.slice(0, 3).map((n) => n.toString(16).padStart(2, '0')).join('');
}

// ── fonts ──────────────────────────────────────────────────────────────────

const loadedFaces = new Map<string, { cell: number; ascent: number }>();

/** Registers HUD + TF2 font files with the webview under every family name the scheme might use. */
export async function registerFonts(fonts: FontData[]): Promise<void> {
  for (const f of fonts) {
    const bin = Uint8Array.from(atob(f.base64), (c) => c.charCodeAt(0));
    for (const family of f.families) {
      const key = family.toLowerCase();
      if (loadedFaces.has(key)) continue;
      try {
        const face = new FontFace(cssFamilyName(family), bin.buffer.slice(0));
        await face.load();
        document.fonts.add(face);
        loadedFaces.set(key, { cell: f.cellRatio, ascent: f.ascentRatio });
      } catch {
        /* unsupported font file; the preview falls back to a system face */
      }
    }
  }
}

function cssFamilyName(f: string) {
  return `hud-${f.toLowerCase().replace(/[^a-z0-9]+/g, '-')}`;
}

export type FontStyle = { family: string; sizeUnits: number; weight: number; italic: boolean; outline: boolean; shadow: boolean; additive: boolean; blur: number };

/** VGUI "tall" sets the GDI cell height (ascent + descent); CSS font-size is the em size. */
export function fontStyle(props: Record<string, string> | null, minmode = false): FontStyle {
  const p = props ?? {};
  const name = p['name'] ?? 'Verdana';
  const metrics = loadedFaces.get(name.toLowerCase());
  const tallRaw = (minmode && p['tall_minmode']) || p['tall'] || '12';
  const tall = parseFloat(tallRaw) || 12;
  const cell = metrics?.cell ?? 1.2;
  return {
    family: metrics ? `"${cssFamilyName(name)}", "${name}", sans-serif` : `"${name}", Verdana, sans-serif`,
    sizeUnits: tall / cell,
    weight: Math.min(900, Math.max(100, Math.round((parseInt(p['weight'] ?? '400', 10) || 400) / 100) * 100)),
    italic: p['italic'] === '1',
    outline: p['outline'] === '1',
    shadow: p['dropshadow'] === '1',
    additive: p['additive'] === '1',
    blur: parseFloat(p['blur'] ?? '0') || 0,
  };
}

// ── borders ────────────────────────────────────────────────────────────────

/** Outline colors of a scheme border ({Left,Right,Top,Bottom} line lists); image borders fall back to none. */
export function borderSides(scheme: SchemeIndex, name: string | undefined): { left?: string; right?: string; top?: string; bottom?: string; rounded?: boolean } | null {
  if (!name) return null;
  const b = scheme.borders.get(name.toLowerCase());
  if (!b) return null;
  const side = (k: string) => {
    const s = b.children?.find((c) => c.key.toLowerCase() === k && c.value === undefined);
    const first = s?.children?.find((c) => c.value === undefined);
    const color = first?.children?.find((c) => c.key.toLowerCase() === 'color')?.value;
    return scheme.css(color);
  };
  const bg = b.children?.find((c) => c.key.toLowerCase() === 'backgroundtype')?.value;
  return { left: side('left'), right: side('right'), top: side('top'), bottom: side('bottom'), rounded: bg === '2' };
}

// ── text ───────────────────────────────────────────────────────────────────

/** Sample values for dialog variables so labels look like a live game. */
export const SAMPLE_VARIABLES: Record<string, string> = {
  health: '150', ammo: '6', ammoinreserve: '32', ammoinclip: '6', ammonoclip: '200', playername: 'Pyrotechnic', playerscore: '12',
  playerclass: 'Soldier', classname: 'Soldier', charge: '87%', chargelevel: '87', chargepercent: '87%', chargecount: '1',
  damage: '-62', heal: '+75', metal: '200', kills: '5', killstreak: '5', deaths: '2', assists: '3', dominations: '1', score: '24',
  round: '2', roundtime: '4:59', timer: '4:59', time: '4:59', servertimeleft: '24:13', redscore: '2', bluescore: '1',
  numpipes: '8', pipes: '8', percentage: '100%', progress: '100%', count: '3', value: '42', level: '3', winreason: 'BLU captured all control points',
  winningteam: 'BLU', losingteam: 'RED', mapname: 'cp_process_final', servername: 'Uncletopia | Chicago',
  playercount: '24/24', ping: '32', redplayers: '12', blueplayers: '12', healer: 'Medic', healthbonus: '+18',
  ammocount: '6', meter: '100', streak: '5', points: '3', dmg: '102', killer: 'Sniper', killername: 'Sniper', victim: 'Spy',
};

export function sampleText(raw: string | undefined, localized: Record<string, string>, vars: Record<string, string> = SAMPLE_VARIABLES): string {
  if (!raw) return '';
  let t = raw;
  if (t.startsWith('#')) t = localized[t] ?? t.slice(1);
  return t.replace(/%([a-z0-9_]+)%/gi, (_, v: string) => vars[v.toLowerCase()] ?? guessVariable(v));
}

/** Plausible sample for dialog variables the table doesn't know, based on the name. */
export function guessVariable(name: string): string {
  const n = name.toLowerCase();
  if (/time|timer|clock/.test(n)) return '1:23';
  if (/name|player|killer|victim|owner/.test(n)) return 'Player';
  if (/percent|pct/.test(n)) return '75%';
  if (/score|count|num|kills|deaths|points|level|streak|round|value|amount|ammo|health|metal|damage|charge|rank|cost|price|money|currency|wave/.test(n)) return '3';
  if (/team/.test(n)) return 'BLU';
  if (/map/.test(n)) return 'cp_process_final';
  return name;
}

/** textAlignment → CSS flex alignment. */
export function alignment(raw: string | undefined): { justify: string; items: string; text: 'left' | 'center' | 'right' } {
  const a = (raw ?? 'west').toLowerCase().replace(/-/g, '');
  const h = a.includes('east') ? 'flex-end' : a.includes('west') ? 'flex-start' : 'center';
  const v = a.startsWith('north') ? 'flex-start' : a.startsWith('south') ? 'flex-end' : 'center';
  return { justify: h, items: v, text: h === 'flex-end' ? 'right' : h === 'center' ? 'center' : 'left' };
}

export const ALIGNMENTS = ['north-west', 'north', 'north-east', 'west', 'center', 'east', 'south-west', 'south', 'south-east'];
