// Opening files in the user's own text and image editors, plus the Settings pickers for them.
import React, { useEffect, useState } from 'react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { FolderOpen } from 'lucide-react';
import { api, errorText, type AppCategory, type AppChoice, type EditorInfo } from '../api.ts';
import { getState, set, toast, useStore } from '../store.ts';
import { Select } from './Select.tsx';

const cache: Partial<Record<AppCategory, Promise<EditorInfo[]>>> = {};
export function editorsFor(category: AppCategory, refresh = false): Promise<EditorInfo[]> {
  if (refresh || !cache[category]) cache[category] = api.editorsList(category).catch(() => []);
  return cache[category]!;
}

/** Display name of the chosen app ("VS Code", "your default app", …). */
export function useAppName(category: AppCategory): string {
  const choice = useStore((s) => (category === 'text' ? s.app?.settings.editor : s.app?.settings.imageEditor));
  const [name, setName] = useState('');
  useEffect(() => {
    let live = true;
    if (!choice || choice.id === 'system') setName(category === 'text' ? 'default editor' : 'default image app');
    else if (choice.id === 'custom') setName(choice.path?.split(/[\\/]/).pop()?.replace(/\.(exe|app)$/i, '') ?? 'custom app');
    else void editorsFor(category).then((l) => live && setName(l.find((e) => e.id === choice.id)?.name ?? choice.id));
    return () => { live = false; };
  }, [choice?.id, choice?.path, category]);
  return name;
}

export async function openInTextEditor(path: string, line?: number) {
  try {
    const name = await api.editorOpen(path, line);
    toast(`Opened ${path.split('/').pop()} in ${name}`, 'ok');
  } catch (e) {
    toast(errorText(e), 'error');
  }
}

export async function openInImageEditor(path: string, asPng: boolean) {
  try {
    const name = await api.imageEditorOpen(path, asPng);
    toast(asPng ? `Opened a PNG copy in ${name}. Convert it back to VTF (e.g. with VTFEdit) when you're done.` : `Opened ${path.split('/').pop()} in ${name}`, 'ok');
  } catch (e) {
    toast(errorText(e), 'error');
  }
}

/** Dropdown of every known app for a category (found ones first), plus system default and a custom program. */
export function AppPicker({ category, value, onChange }: { category: AppCategory; value: AppChoice; onChange: (v: AppChoice) => void }) {
  const [list, setList] = useState<EditorInfo[] | null>(null);
  useEffect(() => void editorsFor(category, true).then(setList), [category]);
  const found = (list ?? []).filter((e) => e.detected);
  const other = (list ?? []).filter((e) => !e.detected);
  const current = list?.find((e) => e.id === value.id);
  const needsPath = value.id === 'custom' || (current && !current.detected);
  const browse = async () => {
    const p = await openDialog({ multiple: false, directory: false, title: 'Choose the program' });
    if (typeof p === 'string') onChange({ ...value, path: p });
  };
  return (
    <div className="app-picker">
      <Select value={value.id} onChange={(e) => onChange({ id: e.target.value, path: null, args: value.id === 'custom' ? value.args : null })}>
        <option value="system">System default app</option>
        {found.length > 0 && (
          <optgroup label="Found on this computer">
            {found.map((e) => <option key={e.id} value={e.id}>{e.name}</option>)}
          </optgroup>
        )}
        {other.length > 0 && (
          <optgroup label="Other known apps (choose their program)">
            {other.map((e) => <option key={e.id} value={e.id}>{e.name}</option>)}
          </optgroup>
        )}
        <option value="custom">Custom program…</option>
      </Select>
      {list === null && <span className="hint">Looking for installed apps…</span>}
      {current?.detected && <span className="hint">{current.path}{category === 'text' ? (current.opensAtLine ? ' · opens at the right line' : ' · opens the file (no line jump)') : ''}{current.terminal ? ' · runs in a terminal' : ''}</span>}
      {needsPath && (
        <span className="field-row">
          <input value={value.path ?? ''} placeholder={value.id === 'custom' ? 'Program to run' : `${current?.name ?? 'App'} wasn't found — choose its program`} onChange={(e) => onChange({ ...value, path: e.target.value || null })} />
          <button className="btn" onClick={() => void browse()}><FolderOpen size={14} /></button>
        </span>
      )}
      {value.id === 'custom' && category === 'text' && (
        <label className="field"><span>Arguments — <code>{'{file}'}</code> and <code>{'{line}'}</code> are filled in</span>
          <input value={value.args ?? ''} placeholder='{file}   (e.g. -g "{file}:{line}")' onChange={(e) => onChange({ ...value, args: e.target.value || null })} /></label>
      )}
    </div>
  );
}

/** Right-click menu for files and other rows. */
export type MenuItem = { label: string; run: () => void; disabled?: boolean };
export function showMenu(e: React.MouseEvent, items: MenuItem[]) {
  e.preventDefault();
  set({ menu: { x: e.clientX, y: e.clientY, items } });
}

export function ContextMenu() {
  const menu = useStore((s) => s.menu);
  useEffect(() => {
    if (!menu) return;
    const close = () => set({ menu: null });
    window.addEventListener('pointerdown', close);
    window.addEventListener('keydown', close);
    window.addEventListener('blur', close);
    return () => {
      window.removeEventListener('pointerdown', close);
      window.removeEventListener('keydown', close);
      window.removeEventListener('blur', close);
    };
  }, [menu]);
  if (!menu) return null;
  const x = Math.min(menu.x, window.innerWidth - 240), y = Math.min(menu.y, window.innerHeight - menu.items.length * 30 - 12);
  return (
    <div className="context-menu" style={{ left: x, top: y }} role="menu" onPointerDown={(e) => e.stopPropagation()}>
      {menu.items.map((it) => (
        <button key={it.label} role="menuitem" disabled={it.disabled} onClick={() => { set({ menu: null }); it.run(); }}>{it.label}</button>
      ))}
    </div>
  );
}

export function fileMenu(path: string, textName: string, imageName: string): MenuItem[] {
  const text = /\.(res|txt|vdf|vmt|cfg|rc|md)$/i.test(path);
  const vtf = /\.vtf$/i.test(path);
  const img = /\.(png|tga|jpg|jpeg|psd|bmp)$/i.test(path);
  const items: MenuItem[] = [];
  if (text) items.push({ label: `Open in ${textName}`, run: () => void openInTextEditor(path) });
  if (vtf) items.push({ label: `Edit in ${imageName}`, run: () => void openInImageEditor(path, false) }, { label: `Edit a PNG copy in ${imageName}`, run: () => void openInImageEditor(path, true) });
  if (img) items.push({ label: `Edit in ${imageName}`, run: () => void openInImageEditor(path, false) });
  items.push({ label: 'Show in folder', run: () => void api.openPath(path).catch((e) => toast(errorText(e), 'error')) });
  return items;
}

export function currentSettings() {
  return getState().app?.settings;
}
