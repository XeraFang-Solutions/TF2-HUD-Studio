// Appearance: pick a built-in or saved theme, edit your own (live), and import/export theme files.
import React, { useEffect, useRef, useState } from 'react';
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';
import { Copy, Download, Trash2, Upload } from 'lucide-react';
import { api, errorText } from '../api.ts';
import { ask, getState, set, toast, useStore } from '../store.ts';
import { currentPrefs, saveThemePrefs, useTheme } from '../appearance.ts';
import {
  FONT_CHOICES, PRESETS, cleanFontName, cleanHex, exportTheme, findTheme, fontStack, importTheme, newThemeId, themePrefs, tf2FontsUsed,
  type CodeColors, type Theme, type ThemeColors, type ThemeFonts,
} from '../theme.ts';
import { Select } from './Select.tsx';

const COLOR_GROUPS: { title: string; keys: [keyof ThemeColors, string][] }[] = [
  { title: 'Surfaces', keys: [['bg', 'Background'], ['panel', 'Panels'], ['raised', 'Buttons & raised'], ['sunken', 'Fields & sunken'], ['line', 'Borders'], ['lineSoft', 'Dividers']] },
  { title: 'Text', keys: [['text', 'Text'], ['muted', 'Secondary text'], ['faint', 'Faint text & icons']] },
  { title: 'Accent', keys: [['accent', 'Accent'], ['accentHover', 'Accent (hover)'], ['accentInk', 'Text on accent']] },
  { title: 'Teams & status', keys: [['red', 'RED / errors'], ['blu', 'BLU / info'], ['ok', 'Success'], ['warn', 'Warnings']] },
];
const CODE_KEYS: [keyof CodeColors, string][] = [
  ['bg', 'Background'], ['key', 'Keys'], ['string', 'Strings'], ['number', 'Numbers'], ['comment', 'Comments'],
  ['keyword', 'Keywords'], ['meta', 'Conditions & #base'], ['brace', 'Braces'], ['type', 'Control names'], ['var', 'Plain text'],
];
const FONT_ROLES: [keyof ThemeFonts, string][] = [['display', 'Headings & main buttons'], ['ui', 'Interface text'], ['mono', 'Code & values']];

function ColorField({ label, value, onChange }: { label: string; value: string; onChange: (v: string) => void }) {
  const [text, setText] = useState(value);
  useEffect(() => setText(value), [value]);
  return (
    <label className="theme-color">
      <span className="swatch" style={{ ['--c' as string]: value }}><input type="color" value={value} onChange={(e) => onChange(e.target.value)} aria-label={label} /></span>
      <span className="theme-color-name">{label}</span>
      <input className="prop-input narrow-hex" value={text} spellCheck={false} onChange={(e) => { setText(e.target.value); const c = cleanHex(e.target.value); if (c) onChange(c); }} onBlur={() => setText(value)} />
    </label>
  );
}

function FontField({ role, label, value, onChange }: { role: keyof ThemeFonts; label: string; value: string; onChange: (v: string) => void }) {
  const known = FONT_CHOICES.some((f) => f.id === value);
  const [custom, setCustom] = useState(known ? '' : value);
  return (
    <label className="field"><span>{label}</span>
      <span className="field-row">
        <Select value={known ? value : 'custom'} onChange={(e) => onChange(e.target.value === 'custom' ? cleanFontName(custom) || 'Arial' : e.target.value)}>
          {FONT_CHOICES.map((f) => <option key={f.id} value={f.id}>{f.label}</option>)}
          <option value="custom">A font installed on this computer…</option>
        </Select>
        {!known && <input value={custom} placeholder="Font name, e.g. Verdana" onChange={(e) => { setCustom(e.target.value); const n = cleanFontName(e.target.value); if (n) onChange(n); }} />}
      </span>
      <span className="font-preview" style={{ fontFamily: fontStack(value, role) }}>The quick brown fox · 0123456789</span>
    </label>
  );
}

function ThemeCard({ t, chosen, onPick }: { t: Theme; chosen: boolean; onPick: () => void }) {
  const c = t.colors;
  return (
    <button className={'theme-card' + (chosen ? ' on' : '')} onClick={onPick} aria-pressed={chosen}>
      <span className="theme-sample" style={{ background: c.bg, borderRadius: t.shape.radius }}>
        <span className="theme-sample-bar" style={{ background: c.panel, borderColor: c.line }}>
          <i style={{ background: c.accent }} /><i style={{ background: c.red }} /><i style={{ background: c.blu }} />
        </span>
        <span className="theme-sample-text" style={{ color: c.text, fontFamily: fontStack(t.fonts.display, 'display'), textTransform: t.shape.caps ? 'uppercase' : 'none' }}>Aa</span>
        <span className="theme-sample-btn" style={{ background: c.accent, color: c.accentInk, borderRadius: t.shape.radius }}>Play</span>
      </span>
      <strong>{t.name}</strong>
      <span className="muted small">{t.id.startsWith('custom-') ? (t.author ? `Yours · by ${t.author}` : 'Yours') : 'Built-in'}{t.dark ? '' : ' · light'}</span>
    </button>
  );
}

export function ThemeDialogBody({ onClose }: { onClose: () => void }) {
  const ui = useStore((s) => s.app?.settings.ui);
  const uiFonts = useStore((s) => s.uiFonts);
  const tfDir = useStore((s) => s.app?.settings.tfDir);
  const prefs = themePrefs(ui);
  const shown = useTheme();
  const editing = shown.id.startsWith('custom-');
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pending = useRef<Theme | null>(null);

  const flush = async () => {
    if (timer.current) clearTimeout(timer.current);
    timer.current = null;
    const t = pending.current;
    pending.current = null;
    if (!t) return;
    const p = currentPrefs();
    await saveThemePrefs({ themeId: t.id, customThemes: p.customThemes.map((x) => (x.id === t.id ? t : x)) });
    if (getState().themeDraft?.id === t.id && !pending.current) set({ themeDraft: null });
  };
  // save what's pending when the dialog closes
  useEffect(() => () => void flush(), []);

  const edit = (patch: Partial<Theme>) => {
    const t = { ...shown, ...patch };
    set({ themeDraft: t });
    pending.current = t;
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => void flush(), 500);
  };
  const choose = async (id: string) => {
    await flush();
    set({ themeDraft: null });
    await saveThemePrefs({ ...currentPrefs(), themeId: id });
  };
  const customize = async () => {
    await flush();
    const copy: Theme = { ...structuredClone(shown), id: newThemeId(), name: `${shown.name} (custom)`.slice(0, 40) };
    const p = currentPrefs();
    await saveThemePrefs({ themeId: copy.id, customThemes: [...p.customThemes, copy] });
  };
  const remove = async () => {
    if (!(await ask(`Delete the theme “${shown.name}”? Export it first if you want to keep a copy.`, 'Delete'))) return;
    await flush();
    const p = currentPrefs();
    set({ themeDraft: null });
    await saveThemePrefs({ themeId: PRESETS[0].id, customThemes: p.customThemes.filter((t) => t.id !== shown.id) });
  };
  const doImport = async () => {
    const path = await openDialog({ multiple: false, title: 'Import a theme', filters: [{ name: 'TF2 HUD Studio theme', extensions: ['json'] }] });
    if (typeof path !== 'string') return;
    try {
      const t = importTheme(await api.themeRead(path));
      await flush();
      const p = currentPrefs();
      await saveThemePrefs({ themeId: t.id, customThemes: [...p.customThemes, t] });
      toast(`Imported “${t.name}”`, 'ok');
    } catch (e) {
      toast(e instanceof Error ? e.message : errorText(e), 'error');
    }
  };
  const doExport = async () => {
    const slug = shown.name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'theme';
    const path = await saveDialog({ title: 'Export theme', defaultPath: `${slug}.hudstudio-theme.json`, filters: [{ name: 'TF2 HUD Studio theme', extensions: ['json'] }] });
    if (!path) return;
    try {
      await api.themeWrite(path, exportTheme(shown));
      toast(`Saved ${path}`, 'ok');
    } catch (e) {
      toast(errorText(e), 'error');
    }
  };

  const all = [...PRESETS, ...prefs.customThemes];
  const missingTf2 = tf2FontsUsed(shown).filter((f) => uiFonts !== null && !uiFonts.includes(f));
  const setColor = (k: keyof ThemeColors, v: string) => edit({ colors: { ...shown.colors, [k]: v } });
  const setCode = (k: keyof CodeColors, v: string) => edit({ code: { ...shown.code, [k]: v } });

  return (
    <>
      <p className="hint">Changes HUD Studio’s own look. Your HUD’s preview is never affected.</p>
      <div className="theme-grid">
        {all.map((t) => <ThemeCard key={t.id} t={t.id === shown.id ? shown : t} chosen={t.id === findTheme(prefs).id} onPick={() => void choose(t.id)} />)}
      </div>
      <div className="row-actions">
        <button className="btn" onClick={() => void customize()}><Copy size={13} /> {editing ? 'Duplicate' : 'Customize a copy'}</button>
        <button className="btn" onClick={() => void doImport()}><Upload size={13} /> Import…</button>
        <button className="btn" onClick={() => void doExport()}><Download size={13} /> Export…</button>
        {editing && <button className="btn" onClick={() => void remove()}><Trash2 size={13} /> Delete</button>}
      </div>
      {missingTf2.length > 0 && (
        <p className="hint">{tfDir ? 'TF2’s fonts weren’t found in your TF2 folder' : 'Set your TF2 folder in Settings to use TF2’s own fonts'}; a similar font is used for now.</p>
      )}
      {!editing && <p className="hint">Built-in themes can’t be changed. Use “Customize a copy” to make your own version and edit it here.</p>}
      {editing && (
        <div className="theme-editor">
          <div className="details-grid">
            <label className="field"><span>Name</span><input value={shown.name} maxLength={40} onChange={(e) => edit({ name: e.target.value })} /></label>
            <label className="field"><span>Author (shown when shared)</span><input value={shown.author ?? ''} maxLength={40} onChange={(e) => edit({ author: e.target.value || undefined })} /></label>
          </div>
          <label className="check"><input type="checkbox" checked={!shown.dark} onChange={(e) => edit({ dark: !e.target.checked })} /> Light theme (light scrollbars and controls)</label>
          {COLOR_GROUPS.map((g) => (
            <section key={g.title}>
              <h3>{g.title}</h3>
              <div className="theme-colors">{g.keys.map(([k, label]) => <ColorField key={k} label={label} value={shown.colors[k]} onChange={(v) => setColor(k, v)} />)}</div>
            </section>
          ))}
          <h3>Fonts</h3>
          <div className="details-grid">
            {FONT_ROLES.map(([k, label]) => <FontField key={k} role={k} label={label} value={shown.fonts[k]} onChange={(v) => edit({ fonts: { ...shown.fonts, [k]: v } })} />)}
          </div>
          <h3>Shape</h3>
          <label className="field"><span>Corner rounding: {shown.shape.radius}px</span>
            <input type="range" min={0} max={12} value={shown.shape.radius} onChange={(e) => edit({ shape: { ...shown.shape, radius: Number(e.target.value) } })} /></label>
          <label className="check"><input type="checkbox" checked={shown.shape.bevel} onChange={(e) => edit({ shape: { ...shown.shape, bevel: e.target.checked } })} /> Beveled buttons and panels (TF2 style)</label>
          <label className="check"><input type="checkbox" checked={shown.shape.caps} onChange={(e) => edit({ shape: { ...shown.shape, caps: e.target.checked } })} /> Uppercase headings and main buttons (TF2 style)</label>
          <h3>Source view colors</h3>
          <div className="theme-colors">{CODE_KEYS.map(([k, label]) => <ColorField key={k} label={label} value={shown.code[k]} onChange={(v) => setCode(k, v)} />)}</div>
        </div>
      )}
      <footer><button className="btn primary" onClick={onClose}>Done</button></footer>
    </>
  );
}

/** Compact picker for the Settings dialog. */
export function ThemePicker() {
  const ui = useStore((s) => s.app?.settings.ui);
  const prefs = themePrefs(ui);
  const current = findTheme(prefs);
  return (
    <div className="field"><span>Theme</span><span className="field-row">
      <Select title="Theme" value={current.id} onChange={(e) => void saveThemePrefs({ ...currentPrefs(), themeId: e.target.value })}>
        {PRESETS.map((t) => <option key={t.id} value={t.id}>{t.name}</option>)}
        {prefs.customThemes.map((t) => <option key={t.id} value={t.id}>{t.name}</option>)}
      </Select>
      <button className="btn" onClick={() => set({ dialog: { kind: 'theme' } })}>Customize…</button>
    </span></div>
  );
}
