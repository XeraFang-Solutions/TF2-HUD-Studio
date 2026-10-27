// App themes: the colors, fonts and shapes of HUD Studio's own interface (never the HUD preview).
// A theme is plain data, applied as CSS custom properties on <html>, saved in settings.ui and shareable as a JSON file.

export type ThemeColors = {
  bg: string; panel: string; raised: string; sunken: string; line: string; lineSoft: string;
  text: string; muted: string; faint: string;
  accent: string; accentHover: string; accentInk: string;
  red: string; blu: string; ok: string; warn: string;
};
export type CodeColors = { bg: string; key: string; string: string; number: string; comment: string; keyword: string; meta: string; brace: string; type: string; var: string };
export type ThemeFonts = { display: string; ui: string; mono: string };
export type ThemeShape = { radius: number; bevel: boolean; caps: boolean };
export type Theme = { id: string; name: string; author?: string; dark: boolean; colors: ThemeColors; code: CodeColors; fonts: ThemeFonts; shape: ThemeShape };

export const THEME_FORMAT = 'tf2-hud-studio-theme';

// ── fonts ──────────────────────────────────────────────────────────────────

/** Font choices offered in the editor. TF2's fonts are Valve's: they are loaded from the user's own TF2 install. */
export const FONT_CHOICES: { id: string; label: string; stack: string; tf2?: string }[] = [
  { id: 'tf2build', label: 'TF2 Build (from your TF2)', stack: "'hs-tf2build', 'Barlow Semi Condensed', 'Segoe UI', sans-serif", tf2: 'tf2build' },
  { id: 'tf2secondary', label: 'TF2 Secondary (from your TF2)', stack: "'hs-tf2secondary', 'Segoe UI', system-ui, sans-serif", tf2: 'tf2secondary' },
  { id: 'tf2professor', label: 'TF2 Professor (from your TF2)', stack: "'hs-tf2professor', 'Segoe UI', system-ui, sans-serif", tf2: 'tf2professor' },
  { id: 'barlow', label: 'Barlow Semi Condensed', stack: "'Barlow Semi Condensed', 'Segoe UI', sans-serif" },
  { id: 'system', label: 'System font', stack: "'Segoe UI', system-ui, -apple-system, Roboto, sans-serif" },
  { id: 'serif', label: 'Serif', stack: "Georgia, 'Times New Roman', serif" },
  { id: 'mono', label: 'Monospace (Cascadia, Consolas…)', stack: "'Cascadia Code', 'JetBrains Mono', Consolas, 'Courier New', monospace" },
];

/** A font id from FONT_CHOICES, or the name of a font installed on the computer. */
export function fontStack(value: string, role: keyof ThemeFonts): string {
  const known = FONT_CHOICES.find((f) => f.id === value);
  if (known) return known.stack;
  const fallback = role === 'mono' ? FONT_CHOICES.find((f) => f.id === 'mono')!.stack : FONT_CHOICES.find((f) => f.id === 'system')!.stack;
  const name = cleanFontName(value);
  return name ? `'${name}', ${fallback}` : fallback;
}

export function cleanFontName(v: string): string {
  return v.replace(/[^A-Za-z0-9 _-]/g, '').trim().slice(0, 60);
}

// ── presets ────────────────────────────────────────────────────────────────

const TF2_CODE: CodeColors = { bg: '#1b1918', key: '#e7d3a8', string: '#a6be7d', number: '#cf6a32', comment: '#75695f', keyword: '#d0605a', meta: '#8fb0c8', brace: '#b2a68f', type: '#c39ad0', var: '#ebe2ca' };

export const PRESETS: Theme[] = [
  {
    id: 'tf2', name: 'TF2', dark: true,
    colors: {
      bg: '#24211f', panel: '#2e2b2a', raised: '#3d3936', sunken: '#1b1918', line: '#564e46', lineSoft: '#3b3735',
      text: '#ebe2ca', muted: '#b2a68f', faint: '#7d7266',
      accent: '#cf6a32', accentHover: '#e27c43', accentInk: '#211d1b',
      red: '#bd3b3b', blu: '#5885a2', ok: '#8bab5b', warn: '#e7b53b',
    },
    code: TF2_CODE,
    fonts: { display: 'tf2build', ui: 'tf2secondary', mono: 'mono' },
    shape: { radius: 5, bevel: true, caps: true },
  },
  {
    id: 'red', name: 'TF2 RED', dark: true,
    colors: {
      bg: '#251e1d', panel: '#302625', raised: '#413330', sunken: '#1c1615', line: '#5e4743', lineSoft: '#3e3230',
      text: '#ebe2ca', muted: '#bba596', faint: '#836b62',
      accent: '#c94a42', accentHover: '#dc5b52', accentInk: '#fbf2e2',
      red: '#c94a42', blu: '#5885a2', ok: '#8bab5b', warn: '#e7b53b',
    },
    code: { ...TF2_CODE, bg: '#1c1615', number: '#e07a62' },
    fonts: { display: 'tf2build', ui: 'tf2secondary', mono: 'mono' },
    shape: { radius: 5, bevel: true, caps: true },
  },
  {
    id: 'blu', name: 'TF2 BLU', dark: true,
    colors: {
      bg: '#1e2125', panel: '#272b30', raised: '#353b42', sunken: '#16191c', line: '#4a535d', lineSoft: '#333a41',
      text: '#e6e1d3', muted: '#a8b0b8', faint: '#6d7882',
      accent: '#6b9cbf', accentHover: '#80b0d2', accentInk: '#13181d',
      red: '#bd3b3b', blu: '#6b9cbf', ok: '#8bab5b', warn: '#e7b53b',
    },
    code: { ...TF2_CODE, bg: '#16191c', key: '#d7dbc8', number: '#80b0d2', brace: '#a8b0b8', var: '#e6e1d3', comment: '#6d7882' },
    fonts: { display: 'tf2build', ui: 'tf2secondary', mono: 'mono' },
    shape: { radius: 5, bevel: true, caps: true },
  },
  {
    id: 'workshop', name: 'Workshop (classic)', dark: true,
    colors: {
      bg: '#241b16', panel: '#2c221c', raised: '#382a22', sunken: '#1c1511', line: '#4a382e', lineSoft: '#3a2c24',
      text: '#f1e4ca', muted: '#b7a288', faint: '#7e6a58',
      accent: '#f0a43a', accentHover: '#f6b555', accentInk: '#2a1d17',
      red: '#c4483f', blu: '#5b8db3', ok: '#8fbf6a', warn: '#e8b84a',
    },
    code: { bg: '#1f1713', key: '#e9c98f', string: '#a9c98a', number: '#f0a43a', comment: '#7e6a58', keyword: '#e1736a', meta: '#8fb3d1', brace: '#b7a288', type: '#c99ad6', var: '#f1e4ca' },
    fonts: { display: 'barlow', ui: 'system', mono: 'mono' },
    shape: { radius: 6, bevel: false, caps: false },
  },
  {
    id: 'light', name: 'Mann Co. Light', dark: false,
    colors: {
      bg: '#f3eee4', panel: '#ebe4d6', raised: '#fbf8f1', sunken: '#e1d8c6', line: '#c6b9a2', lineSoft: '#d8cdba',
      text: '#2a221c', muted: '#685a4c', faint: '#988877',
      accent: '#b5562a', accentHover: '#c76835', accentInk: '#fff8ee',
      red: '#b0362f', blu: '#3d6c92', ok: '#4f7d32', warn: '#946a10',
    },
    code: { bg: '#faf6ee', key: '#8a5a12', string: '#4f7d32', number: '#b5562a', comment: '#9a8b7a', keyword: '#a8322c', meta: '#3d6c92', brace: '#685a4c', type: '#7e4c96', var: '#2a221c' },
    fonts: { display: 'tf2build', ui: 'system', mono: 'mono' },
    shape: { radius: 5, bevel: false, caps: true },
  },
];

export const DEFAULT_THEME_ID = 'tf2';

// ── settings storage ───────────────────────────────────────────────────────

/** What settings.ui keeps: the chosen theme id and the user's own themes. */
export type ThemePrefs = { themeId: string; customThemes: Theme[] };

export function themePrefs(ui: Record<string, unknown> | null | undefined): ThemePrefs {
  const raw = (ui?.customThemes as unknown[] | undefined) ?? [];
  const customThemes = (Array.isArray(raw) ? raw : []).map((t) => parseTheme(t)).filter((t): t is Theme => !!t && t.id.startsWith('custom-'));
  const themeId = typeof ui?.themeId === 'string' ? ui.themeId : DEFAULT_THEME_ID;
  return { themeId, customThemes };
}

export function findTheme(prefs: ThemePrefs): Theme {
  return prefs.customThemes.find((t) => t.id === prefs.themeId) ?? PRESETS.find((t) => t.id === prefs.themeId) ?? PRESETS[0];
}

export function newThemeId(): string {
  return `custom-${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;
}

// ── validation (themes can come from files other people made) ──────────────

const HEX = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i;

export function cleanHex(v: unknown): string | null {
  if (typeof v !== 'string') return null;
  const m = HEX.exec(v.trim());
  if (!m) return null;
  const h = m[1].length === 3 ? m[1].split('').map((c) => c + c).join('') : m[1];
  return '#' + h.toLowerCase();
}

function pickColors<T extends Record<string, string>>(raw: unknown, base: T): T {
  const src = raw && typeof raw === 'object' ? (raw as Record<string, unknown>) : {};
  const out = { ...base };
  for (const k of Object.keys(base) as (keyof T)[]) {
    const c = cleanHex(src[k as string]);
    if (c) out[k] = c as T[keyof T];
  }
  return out;
}

/**
 * Turns untrusted JSON into a complete, safe theme: only known keys, colors as #rrggbb, plain font names, bounded
 * numbers. Missing values come from the closest preset. Returns null when it isn't a theme at all.
 */
export function parseTheme(raw: unknown): Theme | null {
  if (!raw || typeof raw !== 'object') return null;
  const r = raw as Record<string, unknown>;
  if (!r.colors || typeof r.colors !== 'object') return null;
  const dark = r.dark !== false;
  const base = dark ? PRESETS[0] : PRESETS.find((p) => p.id === 'light')!;
  const fontsRaw = (r.fonts && typeof r.fonts === 'object' ? r.fonts : {}) as Record<string, unknown>;
  const font = (k: keyof ThemeFonts) => {
    const v = typeof fontsRaw[k] === 'string' ? (fontsRaw[k] as string) : '';
    return FONT_CHOICES.some((f) => f.id === v) ? v : cleanFontName(v) || base.fonts[k];
  };
  const shapeRaw = (r.shape && typeof r.shape === 'object' ? r.shape : {}) as Record<string, unknown>;
  const radius = typeof shapeRaw.radius === 'number' && Number.isFinite(shapeRaw.radius) ? Math.max(0, Math.min(16, Math.round(shapeRaw.radius))) : base.shape.radius;
  const text = (v: unknown, max: number) => (typeof v === 'string' ? v.replace(/[\u0000-\u001f<>]/g, '').trim().slice(0, max) : '');
  return {
    id: /^[a-z0-9-]{1,40}$/.test(String(r.id ?? '')) ? String(r.id) : newThemeId(),
    name: text(r.name, 40) || 'Untitled theme',
    ...(text(r.author, 40) ? { author: text(r.author, 40) } : {}),
    dark,
    colors: pickColors(r.colors, base.colors),
    code: pickColors(r.code, base.code),
    fonts: { display: font('display'), ui: font('ui'), mono: font('mono') },
    shape: { radius, bevel: typeof shapeRaw.bevel === 'boolean' ? shapeRaw.bevel : base.shape.bevel, caps: typeof shapeRaw.caps === 'boolean' ? shapeRaw.caps : base.shape.caps },
  };
}

/** A shareable theme file. */
export function exportTheme(t: Theme): string {
  const { id: _id, ...rest } = t;
  return JSON.stringify({ format: THEME_FORMAT, version: 1, ...rest }, null, 2) + '\n';
}

/** Reads a theme file; the imported theme always gets a fresh custom id. */
export function importTheme(text: string): Theme {
  let json: unknown;
  try {
    json = JSON.parse(text);
  } catch {
    throw new Error('That file isn’t valid JSON.');
  }
  if (!json || typeof json !== 'object' || (json as Record<string, unknown>).format !== THEME_FORMAT) throw new Error('That file isn’t a TF2 HUD Studio theme.');
  const t = parseTheme(json);
  if (!t) throw new Error('That theme file has no colors.');
  return { ...t, id: newThemeId() };
}

// ── applying ───────────────────────────────────────────────────────────────

/** Mixes two #rrggbb colors; `amount` is how much of `b`. */
export function mix(a: string, b: string, amount: number): string {
  const pa = [1, 3, 5].map((i) => parseInt(a.slice(i, i + 2), 16));
  const pb = [1, 3, 5].map((i) => parseInt(b.slice(i, i + 2), 16));
  return '#' + pa.map((c, i) => Math.round(c + (pb[i] - c) * amount).toString(16).padStart(2, '0')).join('');
}

const kebab = (k: string) => k.replace(/[A-Z]/g, (c) => '-' + c.toLowerCase());
const rgb = (hex: string) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16)).join(', ');

/** CSS custom properties for a theme (each color also as an "r, g, b" triple for translucent tints). */
export function themeVars(t: Theme): Record<string, string> {
  const v: Record<string, string> = {};
  for (const [k, c] of Object.entries(t.colors)) {
    v[`--${kebab(k)}`] = c;
    v[`--${kebab(k)}-rgb`] = rgb(c);
  }
  for (const [k, c] of Object.entries(t.code)) v[`--code-${k}`] = c;
  v['--display'] = fontStack(t.fonts.display, 'display');
  v['--ui'] = fontStack(t.fonts.ui, 'ui');
  v['--mono'] = fontStack(t.fonts.mono, 'mono');
  v['--r'] = `${t.shape.radius}px`;
  v['--bevel'] = t.shape.bevel
    ? (t.dark ? 'inset 0 1px 0 rgba(255,255,255,.07), inset 0 -1px 0 rgba(0,0,0,.35)' : 'inset 0 1px 0 rgba(255,255,255,.7), inset 0 -1px 0 rgba(0,0,0,.08)')
    : '0 0 0 transparent';
  v['--heading-case'] = t.shape.caps ? 'uppercase' : 'none';
  v['--heading-spacing'] = t.shape.caps ? '.03em' : '0';
  v['--shadow-rgb'] = t.dark ? '0, 0, 0' : '60, 40, 20';
  v['--sheen'] = t.shape.bevel ? `linear-gradient(180deg, rgba(255,255,255,${t.dark ? '.035' : '.5'}), rgba(255,255,255,0) 70%)` : 'none';
  // readable text versions of the team colors on this theme's panels
  v['--red-soft'] = mix(t.colors.red, t.colors.text, 0.4);
  v['--blu-soft'] = mix(t.colors.blu, t.colors.text, 0.4);
  return v;
}

export function applyTheme(t: Theme, root: HTMLElement = document.documentElement) {
  for (const [k, val] of Object.entries(themeVars(t))) root.style.setProperty(k, val);
  root.style.setProperty('color-scheme', t.dark ? 'dark' : 'light');
  root.dataset.theme = t.id;
  root.dataset.themeMode = t.dark ? 'dark' : 'light';
}

/** The TF2 font files a theme uses (ids from FONT_CHOICES with a `tf2` file). */
export function tf2FontsUsed(t: Theme): string[] {
  return [...new Set(Object.values(t.fonts).map((f) => FONT_CHOICES.find((c) => c.id === f)?.tf2).filter((x): x is string => !!x))];
}
