// The HUD library (a folder of HUDs, one of which is installed into tf/custom) and TFHUD.manifest editing.
import React, { useEffect, useState } from 'react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { FolderOpen, Link2, Pencil, Plus, Repeat, Trash2, Upload } from 'lucide-react';
import { api, errorText, type HudDetails, type InstallMode, type InstallResult, type LibraryHud, type LibraryState, type Manifest } from '../api.ts';
import { ask, guard, openProject, set, toast, updateSettings, useStore } from '../store.ts';
import { Select } from './Select.tsx';
import { Marketplace } from './Marketplace.tsx';

export function installSummary(r: InstallResult): string {
  const parts = [r.mode === 'link' ? `Linked ${r.target}` : `Copied ${r.files} files to ${r.target}`];
  if (r.unlinked.length) parts.push(`unlinked ${r.unlinked.join(', ')}`);
  if (r.backedUp.length) parts.push(`${r.backedUp.length} folder${r.backedUp.length > 1 ? 's' : ''} moved to backups`);
  return parts.join(' · ');
}

function HudIcon({ d, size = 56 }: { d: HudDetails; size?: number }) {
  return d.iconUrl ? <img className="hud-icon" src={d.iconUrl} width={size} height={size} alt="" /> : <span className="hud-icon blank" style={{ width: size, height: size }}>{d.manifest.name.slice(0, 2).toUpperCase()}</span>;
}

export function LibraryDialog({ onClose }: { onClose: () => void }) {
  const app = useStore((s) => s.app);
  const project = useStore((s) => s.project);
  const [lib, setLib] = useState<LibraryState | null>(null);
  const [tab, setTab] = useState<'local' | 'marketplace'>('local');
  const [marketVisited, setMarketVisited] = useState(false);
  const mode = app?.settings.installMode ?? 'link';
  const refresh = async () => setLib((await guard(null, () => api.libraryGet())) ?? null);
  useEffect(() => void refresh(), []);
  if (!app) return null;

  const chooseFolder = async () => {
    const p = await openDialog({ directory: true, title: 'Choose (or create) a folder for your HUDs' });
    if (typeof p !== 'string') return;
    const s = await guard('Opening library…', () => api.librarySet(p));
    if (s) {
      setLib(s);
      set({ app: { ...app, settings: { ...app.settings, hudLibrary: s.library } } });
    }
  };
  const swap = async (h: LibraryHud) => {
    const plan = await guard(null, () => api.librarySwapPlan(h.folder));
    if (!plan) return;
    if (plan.length) {
      const list = plan.map((c) => `${c.name} (${c.kind === 'link' ? 'link, just removed' : 'folder, moved to backups'})`).join(', ');
      if (!(await ask(`Make ${h.name} your installed HUD? This takes out ${list}. Configs and other custom content stay.`, 'Swap HUD'))) return;
    }
    const r = await guard('Swapping HUD…', () => api.librarySwap(h.folder, mode));
    if (r) toast(`${h.name} is installed. ${installSummary(r)}. Restart TF2 or run hud_reloadscheme.`, 'ok');
    void refresh();
  };
  const install = async (h: LibraryHud) => {
    const r = await guard('Installing…', () => api.libraryInstall({ folder: h.folder, mode }));
    if (r) toast(installSummary(r), 'ok');
    void refresh();
  };
  const uninstall = async (folder: string, name: string, kind: string) => {
    if (!(await ask(kind === 'link' ? `Remove the ${name} link from tf/custom? The HUD itself stays where it is.` : `Take ${name} out of tf/custom? The folder is moved to HUD Studio's backups, not deleted.`, 'Remove'))) return;
    const r = await guard(null, () => api.libraryUninstall(folder));
    if (r) toast(installSummary({ ...r, mode: 'copy', files: 0, target: name }).replace(/^Copied 0 files to /, 'Removed '), 'ok');
    void refresh();
  };
  const adopt = async (folder: string, name: string) => {
    if (project && project.root.replace(/\\/g, '/').toLowerCase().endsWith(`/custom/${folder.toLowerCase()}`)) {
      toast('That HUD is open right now. Open another HUD first, then move it.', 'error');
      return;
    }
    if (!(await ask(`Move ${name} into your library and ${mode === 'link' ? 'link it back' : 'copy it back'} into tf/custom?`, 'Move to library'))) return;
    const s = await guard('Moving…', () => api.libraryAdopt(folder, mode));
    if (s) { setLib(s); toast(`${name} is now in your library`, 'ok'); }
  };
  const add = async () => {
    const p = await openDialog({ directory: true, title: 'Choose a HUD folder to copy into the library' });
    if (typeof p !== 'string') return;
    const dest = await guard('Copying…', () => api.libraryAdd(p));
    if (dest) toast(`Added to the library: ${dest}`, 'ok');
    void refresh();
  };

  const loose = lib?.custom.filter((c) => c.isHud && !c.libraryFolder) ?? [];
  return (
    <>
      <div className="lib-head">
        <label className="field grow"><span>Library folder</span>
          <span className="field-row"><input readOnly value={lib?.library ?? ''} placeholder="Choose a folder to keep your HUDs in (outside tf/custom)" />
            <button className="btn" onClick={() => void chooseFolder()}><FolderOpen size={14} /> Choose…</button></span></label>
        <label className="field"><span>Install as</span>
          <Select value={mode} onChange={(e) => void updateSettings({ installMode: e.target.value as InstallMode })}>
            <option value="link">Link (no copy; saves space)</option>
            <option value="copy">Copy</option>
          </Select></label>
      </div>
      <div className="market-tabs" role="tablist" aria-label="HUD library views">
        <button className={'btn' + (tab === 'local' ? ' primary' : '')} role="tab" aria-selected={tab === 'local'} onClick={() => setTab('local')}>My HUDs{lib ? ` (${lib.huds.length})` : ''}</button>
        <button className={'btn' + (tab === 'marketplace' ? ' primary' : '')} role="tab" aria-selected={tab === 'marketplace'} onClick={() => { setMarketVisited(true); setTab('marketplace'); }}>Marketplace</button>
      </div>
      <div hidden={tab !== 'marketplace'}>{marketVisited && <Marketplace library={lib?.library ?? null} huds={lib?.huds ?? []} onImported={refresh} />}</div>
      <div hidden={tab !== 'local'}>
      {!lib?.library && <p className="hint">Pick a folder (for example Documents/TF2 HUDs). Every HUD in it shows up here, and one click swaps which one TF2 loads. Linking means TF2 reads the library copy directly: nothing is duplicated, and every save shows up in game.</p>}
      {lib?.library && (
        <>
          <div className="lib-grid">
            {lib.huds.map((h) => {
              const installed = h.installs.length > 0;
              const d = h.details;
              return (
                <article key={h.folder} className={'lib-card' + (installed ? ' on' : '')}>
                  {d.bannerUrl && <img className="lib-banner" src={d.bannerUrl} alt="" />}
                  <div className="lib-main">
                    <HudIcon d={d} />
                    <div className="lib-text">
                      <strong>{d.manifest.name}</strong>
                      <span className="muted">{[d.manifest.version && `v${d.manifest.version}`, d.manifest.author].filter(Boolean).join(' · ') || h.folder}</span>
                      {d.manifest.description && <p>{d.manifest.description}</p>}
                    </div>
                  </div>
                  <div className="lib-state">
                    {installed ? h.installs.map((c) => <span key={c.folder} className="pill ok" title={c.path}>{c.kind === 'link' ? <Link2 size={11} /> : null} Installed as {c.kind}</span>) : <span className="pill">Not installed</span>}
                    {!d.hasManifest && <span className="pill" title="No TFHUD.manifest yet">No manifest</span>}
                  </div>
                  <div className="lib-actions">
                    {!installed && <button className="btn primary" onClick={() => void swap(h)} title="Install this HUD and take the others out"><Repeat size={13} /> Use this HUD</button>}
                    {!installed && <button className="btn" onClick={() => void install(h)} title="Install alongside the others"><Upload size={13} /> Install</button>}
                    {installed && h.installs.map((c) => <button key={c.folder} className="btn" onClick={() => void uninstall(c.folder, h.name, c.kind)}><Trash2 size={13} /> Uninstall</button>)}
                    <button className="btn" onClick={() => { onClose(); void openProject(h.path); }}>Open</button>
                    <button className="btn icon-only" title="Edit details (TFHUD.manifest)" onClick={() => set({ dialog: { kind: 'hud-details', path: h.path } })}><Pencil size={13} /></button>
                    <button className="btn icon-only" title="Show folder" onClick={() => api.openPath(h.path).catch((e) => toast(errorText(e), 'error'))}><FolderOpen size={13} /></button>
                  </div>
                </article>
              );
            })}
            <button className="lib-card add" onClick={() => void add()}><Plus size={22} /><span>Add a HUD folder…</span><span className="muted">Copies a downloaded or extracted HUD into the library</span></button>
          </div>
          {loose.length > 0 && (
            <>
              <h3>In tf/custom, not in the library</h3>
              <div className="tools-list">
                {loose.map((c) => (
                  <div className="tool-row" key={c.folder}>
                    <div className="tool-info"><strong>{c.name}</strong><span className="muted">custom/{c.folder}{c.kind === 'link' ? ` → ${c.target}` : ''}</span></div>
                    {c.kind === 'copy' && <button className="btn" onClick={() => void adopt(c.folder, c.name)}>Move to library</button>}
                    <button className="btn" onClick={() => void uninstall(c.folder, c.name, c.kind)}>Take out</button>
                  </div>
                ))}
              </div>
            </>
          )}
        </>
      )}
      </div>
    </>
  );
}

const FIELDS: { key: keyof Manifest; label: string; placeholder?: string; long?: boolean }[] = [
  { key: 'name', label: 'Name' },
  { key: 'version', label: 'Version', placeholder: '1.1.0' },
  { key: 'author', label: 'Author' },
  { key: 'description', label: 'Description', long: true },
  { key: 'homepage', label: 'Homepage', placeholder: 'https://…' },
  { key: 'source', label: 'Source', placeholder: 'https://github.com/…' },
  { key: 'license', label: 'License', placeholder: 'e.g. CC BY-NC-SA 4.0' },
  { key: 'tags', label: 'Tags', placeholder: 'minimal, competitive, 4:3' },
];

/** Edits TFHUD.manifest for `path` (a library HUD) or the open project. */
export function HudDetailsForm({ path, onDone }: { path?: string; onDone: () => void }) {
  const [d, setD] = useState<HudDetails | null>(null);
  const [m, setM] = useState<Manifest | null>(null);
  useEffect(() => {
    void guard(null, () => api.manifestGet(path)).then((r) => { if (r) { setD(r); setM(r.manifest); } });
  }, [path]);
  if (!d || !m) return <p className="hint">Loading…</p>;
  const pick = async (slot: 'icon' | 'banner') => {
    const p = await openDialog({ multiple: false, title: slot === 'icon' ? 'Choose an icon (square works best)' : 'Choose a banner (wide)', filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'svg'] }] });
    if (typeof p !== 'string') return;
    const r = await guard(null, () => api.manifestSetImage(slot, p, path));
    if (r) { setD(r); setM({ ...m, [slot]: r.manifest[slot] }); }
  };
  const save = async () => {
    const r = await guard('Saving…', () => api.manifestSave(m, path));
    if (r) { toast(d.hasManifest ? 'Saved TFHUD.manifest' : 'Created TFHUD.manifest', 'ok'); onDone(); }
  };
  return (
    <>
      <p className="hint">Stored in <code>TFHUD.manifest</code> at the top of the HUD. It’s a KeyValues file like <code>info.vdf</code>; TF2 ignores it, HUD Studio’s library shows it.</p>
      <div className="details-images">
        <button className="details-icon" onClick={() => void pick('icon')} title="Choose an icon">{d.iconUrl ? <img src={d.iconUrl} alt="" /> : <span>Icon</span>}</button>
        <button className="details-banner" onClick={() => void pick('banner')} title="Choose a banner">{d.bannerUrl ? <img src={d.bannerUrl} alt="" /> : <span>Banner (optional)</span>}</button>
      </div>
      <div className="details-grid">
        {FIELDS.map((f) => (
          <label key={f.key} className={'field' + (f.long ? ' span2' : '')}><span>{f.label}</span>
            {f.long ? <textarea rows={3} value={m[f.key]} onChange={(e) => setM({ ...m, [f.key]: e.target.value })} /> : <input value={m[f.key]} placeholder={f.placeholder} onChange={(e) => setM({ ...m, [f.key]: e.target.value })} />}
          </label>
        ))}
      </div>
      <footer><button className="btn" onClick={onDone}>Cancel</button><button className="btn primary" disabled={!m.name.trim()} onClick={() => void save()}>{d.hasManifest ? 'Save' : 'Create manifest'}</button></footer>
    </>
  );
}

export function openLibrary() {
  set({ dialog: { kind: 'library' } });
}
