import React, { useEffect, useMemo, useState } from 'react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { ExternalLink, FolderOpen, Library, Plus, X } from 'lucide-react';
import { api, type ExternalTool, type InstallMode, type LaunchSettings } from '../api.ts';
import { docEdit, getState, guard, openFile, openProject, rebuildModel, refreshSemantics, set, toast, updateSettings, useStore } from '../store.ts';
import { HudDetailsForm, LibraryDialog, installSummary } from './Library.tsx';
import { CONTROL_TEMPLATES } from '../vgui/catalog.ts';
import { AppPicker } from './External.tsx';
import { walk } from '../vgui/model.ts';
import { Select } from './Select.tsx';
import { ThemeDialogBody, ThemePicker } from './ThemeEditor.tsx';

function Modal({ title, children, onClose, wide }: { title: string; children: React.ReactNode; onClose: () => void; wide?: boolean }) {
  useEffect(() => {
    const k = (e: KeyboardEvent) => e.key === 'Escape' && onClose();
    window.addEventListener('keydown', k);
    return () => window.removeEventListener('keydown', k);
  }, []);
  return (
    <div className="modal-backdrop" onPointerDown={onClose}>
      <section className={'modal' + (wide ? ' wide' : '')} onPointerDown={(e) => e.stopPropagation()} role="dialog" aria-label={title}>
        <header><h2>{title}</h2><button className="icon-btn" onClick={onClose} aria-label="Close"><X size={16} /></button></header>
        <div className="modal-body">{children}</div>
      </section>
    </div>
  );
}

const close = () => set({ dialog: null });

export async function pickAndOpenHud() {
  const path = await openDialog({ directory: true, multiple: false, title: 'Choose a TF2 HUD folder' });
  if (typeof path === 'string') await openProject(path);
}

// ── welcome ────────────────────────────────────────────────────────────────
export function Welcome() {
  const app = useStore((s) => s.app);
  const installs = app?.installs ?? [];
  const recent = app?.settings.recentProjects ?? [];
  return (
    <div className="welcome">
      <div className="welcome-mark"><img src="/icon.svg" alt="" /></div>
      <h1>TF2 HUD Studio</h1>
      <p className="lede">Open a HUD folder to edit it visually, or start a new one.</p>
      <div className="welcome-actions">
        <button className="btn primary big" onClick={() => void pickAndOpenHud()}><FolderOpen size={17} /> Open HUD folder</button>
        <button className="btn big" onClick={() => set({ dialog: { kind: 'new-hud' } })}><Plus size={17} /> New HUD</button>
        <button className="btn big" onClick={() => set({ dialog: { kind: 'library' } })}><Library size={17} /> HUD library</button>
      </div>
      <div className="welcome-cols">
        <section>
          <h3>Recent</h3>
          {recent.length === 0 && <p className="hint">HUDs you open show up here.</p>}
          {recent.map((r) => <button key={r} className="link-row" onClick={() => void openProject(r)} title={r}><span>{r.split(/[\\/]/).pop()}</span><span className="muted">{r}</span></button>)}
        </section>
        <section>
          <h3>Installed in TF2</h3>
          {installs.length === 0 && <p className="hint">TF2 wasn’t found automatically. Set its folder in Settings to load stock files, fonts and icons.</p>}
          {installs.flatMap((i) => i.huds).map((h) => <button key={h.path} className="link-row" onClick={() => void openProject(h.path)} title={h.path}><span>{h.name}</span><span className="muted">custom/{h.folder}</span></button>)}
        </section>
      </div>
      <p className="hint">Tip: work on a copy, or keep backups on (Settings) — every save keeps the previous version.</p>
    </div>
  );
}

// ── new HUD ────────────────────────────────────────────────────────────────
function NewHud() {
  const app = useStore((s) => s.app);
  const [name, setName] = useState('My HUD');
  const [parent, setParent] = useState(app?.settings.hudLibrary ?? (app?.settings.tfDir ? `${app.settings.tfDir}${app.settings.tfDir.includes('\\') ? '\\' : '/'}custom` : ''));
  const [author, setAuthor] = useState('');
  const [template, setTemplate] = useState<'stock' | 'empty'>(app?.settings.tfDir ? 'stock' : 'empty');
  const create = async () => {
    const path = await guard('Creating HUD…', () => api.projectCreate(parent, name, template));
    if (path) {
      close();
      await guard(null, () => api.manifestSave({ name, version: '1.1.0', author, description: '', icon: '', banner: '', homepage: '', source: '', license: '', tags: '' }, path));
      await openProject(path);
      toast(`Created ${name}`, 'ok');
    }
  };
  return (
    <Modal title="New HUD" onClose={close}>
      <label className="field"><span>Name</span><input value={name} onChange={(e) => setName(e.target.value)} autoFocus /></label>
      <label className="field"><span>Author (optional)</span><input value={author} onChange={(e) => setAuthor(e.target.value)} placeholder="Shown in the HUD library" /></label>
      <label className="field"><span>Create in{app?.settings.hudLibrary ? ' (your HUD library)' : ''}</span>
        <span className="field-row"><input value={parent} onChange={(e) => setParent(e.target.value)} placeholder="Folder" />
          <button className="btn" onClick={async () => { const p = await openDialog({ directory: true }); if (typeof p === 'string') setParent(p); }}><FolderOpen size={14} /></button></span>
      </label>
      <div className="choice-list">
        <label className={'choice' + (template === 'stock' ? ' on' : '')}><input type="radio" checked={template === 'stock'} onChange={() => setTemplate('stock')} />
          <span><strong>Start from TF2’s default HUD</strong><span className="muted">Copies the default scheme, hudlayout and the health, ammo, class, über, damage and target ID files so you can restyle them. Needs your TF2 folder.</span></span></label>
        <label className={'choice' + (template === 'empty' ? ' on' : '')}><input type="radio" checked={template === 'empty'} onChange={() => setTemplate('empty')} />
          <span><strong>Empty HUD</strong><span className="muted">Only info.vdf and your own animation file. Everything looks like TF2’s default HUD; the first time you edit an element, its file is copied in from TF2.</span></span></label>
      </div>
      <footer><button className="btn" onClick={close}>Cancel</button><button className="btn primary" disabled={!name.trim() || !parent.trim()} onClick={() => void create()}>Create HUD</button></footer>
    </Modal>
  );
}

// ── settings ───────────────────────────────────────────────────────────────
function Settings() {
  const app = useStore((s) => s.app);
  const project = useStore((s) => s.project);
  const [tf, setTf] = useState(app?.settings.tfDir ?? '');
  if (!app) return null;
  const saveTf = async () => {
    const info = await guard('Loading TF2 files…', () => api.gameSetDir(tf));
    if (!info) return;
    toast(`TF2 found · ${info.vpkFiles.toLocaleString()} stock files indexed`, 'ok');
    const a = await api.appInit();
    set({ app: a });
    if (project) await openProject(project.root);
  };
  const saveSettings = async (patch: Partial<typeof app.settings>) => {
    const settings = await guard(null, () => api.settingsSave({ ...app.settings, ...patch }));
    if (settings) set({ app: { ...app, settings } });
  };
  return (
    <Modal title="Settings" onClose={close} wide>
      <h3>Appearance</h3>
      <p className="hint">HUD Studio’s own colors and fonts. TF2 themes use the fonts from your TF2 install.</p>
      <ThemePicker />
      <h3>Team Fortress 2</h3>
      <p className="hint">The “tf” folder inside your TF2 install. Used for the default HUD files, TF2’s fonts and icons, and deploying.</p>
      <label className="field"><span>TF2 folder</span>
        <span className="field-row"><input value={tf} onChange={(e) => setTf(e.target.value)} placeholder="…/steamapps/common/Team Fortress 2/tf" />
          <button className="btn" onClick={async () => { const p = await openDialog({ directory: true }); if (typeof p === 'string') setTf(p); }}><FolderOpen size={14} /></button>
          <button className="btn primary" onClick={() => void saveTf()}>Use this folder</button></span>
      </label>
      {app.installs.length > 0 && <p className="hint">Found: {app.installs.map((i) => i.tfDir).join(', ')}</p>}
      <h3>HUD library</h3>
      <p className="hint">A folder you keep your HUDs in. From the library you can swap which HUD TF2 loads in one click. A link (a junction on Windows) installs a HUD without copying it, so saves show up in game straight away.</p>
      <span className="field-row"><input readOnly value={app.settings.hudLibrary ?? ''} placeholder="Not set" />
        <button className="btn" onClick={() => set({ dialog: { kind: 'library' } })}><Library size={14} /> Open library</button></span>
      <label className="field"><span>Install HUDs as</span>
        <Select value={app.settings.installMode ?? 'link'} onChange={(e) => void saveSettings({ installMode: e.target.value as InstallMode })}>
          <option value="link">Link (no copy; saves space; edits are live)</option><option value="copy">Copy</option>
        </Select></label>
      <LaunchSection />
      <h3>Text editor</h3>
      <p className="hint">Used by “Open in editor” (source view, right-click on files, Problems). HUD Studio’s own editor is always available too.</p>
      <AppPicker category="text" value={app.settings.editor ?? { id: 'system', path: null, args: null }} onChange={(v) => void saveSettings({ editor: v })} />
      <h3>Image editor</h3>
      <p className="hint">Used by “Edit image”. Editors without VTF support get a PNG copy to work on.</p>
      <AppPicker category="image" value={app.settings.imageEditor ?? { id: 'system', path: null, args: null }} onChange={(v) => void saveSettings({ imageEditor: v })} />
      <h3>Saving</h3>
      <label className="check"><input type="checkbox" checked={app.settings.backups} onChange={(e) => void saveSettings({ backups: e.target.checked })} /> Back up files before overwriting them</label>
      <p className="hint">Backups go to <code>{app.backupDir}</code>, one folder per save; the newest 60 are kept per HUD.</p>
      <h3>Preview</h3>
      <label className="field"><span>Preview target operating system</span>
        <Select value={app.settings.platform} onChange={async (e) => { await saveSettings({ platform: e.target.value as 'windows' | 'linux' | 'mac' }); await refreshSemantics(); await rebuildModel(); }}>
          <option value="windows">Windows</option><option value="linux">Linux / Steam Deck</option><option value="mac">macOS</option>
        </Select></label>
      <p className="hint">Use the platform TF2 runs on. The legacy [$WIN32] desktop flag is active on Linux too; [$WINDOWS], [$LINUX] and [$OSX] select OS-specific settings. Older saved settings keep their selected target.</p>
      <label className="field"><span>Backdrop</span>
        <Select value={(app.settings.ui?.backdrop as string) ?? 'dusk'} onChange={(e) => void saveSettings({ ui: { ...app.settings.ui, backdrop: e.target.value } })}>
          <option value="dusk">Dusk (warm gradient)</option><option value="day">Daylight map</option><option value="dark">Dark</option><option value="checker">Transparency checker</option>
        </Select></label>
      <label className="check"><input type="checkbox" checked={app.settings.ui?.loopAnim !== false} onChange={(e) => void saveSettings({ ui: { ...app.settings.ui, loopAnim: e.target.checked } })} /> Loop animation playback</label>
      <footer><span className="muted">TF2 HUD Studio {app.version}</span><button className="btn" onClick={close}>Done</button></footer>
    </Modal>
  );
}

// ── launching TF2 ─────────────────────────────────────────────────────────
const isWindows = typeof navigator !== 'undefined' && /windows/i.test(navigator.userAgent);
function LaunchSection() {
  const app = useStore((s) => s.app);
  const [extra, setExtra] = useState(app?.settings.launch.extraArgs ?? '');
  if (!app) return null;
  const l = app.settings.launch;
  const save = (patch: Partial<LaunchSettings>) => void updateSettings({ launch: { ...l, ...patch } });
  const num = (v: string) => (v.trim() === '' ? null : Math.max(320, Math.min(16384, Math.round(Number(v)) || 0)) || null);
  const detect = async () => {
    const r = await guard(null, () => api.displaySize());
    if (r) { save({ width: r[0], height: r[1] }); toast(`Display: ${r[0]} × ${r[1]}`, 'ok'); }
  };
  return (
    <>
      <h3>Launching TF2</h3>
      <p className="hint">The Play button starts TF2 to test your HUD. Steam needs to be running.{!isWindows ? ' On Linux TF2 is 64-bit only and is started through Steam.' : ''}</p>
      {isWindows && (
        <div className="field-grid">
          <label className="field"><span>Build</span>
            <Select value={l.bits} onChange={(e) => save({ bits: e.target.value as '64' | '32' })}><option value="64">64-bit (standard)</option><option value="32">32-bit (legacy)</option></Select></label>
          <label className="field"><span>Start it</span>
            <Select value={l.method} onChange={(e) => save({ method: e.target.value as 'direct' | 'steam' })}><option value="direct">Directly (lets HUD Studio pick 64/32-bit)</option><option value="steam">Through Steam</option></Select></label>
        </div>
      )}
      <label className="check"><input type="checkbox" checked={l.itemtest} onChange={(e) => save({ itemtest: e.target.checked })} /> <span>Load straight into <code>itemtest</code> (local game: <code>sv_lan 1</code>, borderless window, quick-start switches)</span></label>
      <div className="field-grid">
        <label className="field"><span>Window width</span><input inputMode="numeric" defaultValue={l.width ?? ''} key={`w${l.width}`} placeholder="detected on first launch" onBlur={(e) => save({ width: num(e.target.value) })} /></label>
        <label className="field"><span>Window height</span><input inputMode="numeric" defaultValue={l.height ?? ''} key={`h${l.height}`} placeholder="detected on first launch" onBlur={(e) => save({ height: num(e.target.value) })} /></label>
        <button className="btn align-end" onClick={() => void detect()}>Use my display size</button>
      </div>
      <label className="check"><input type="checkbox" checked={l.rcon} onChange={(e) => save({ rcon: e.target.checked })} /> <span>Remote console (<code>-usercon</code> on 127.0.0.1, with a new password each launch): send commands and test buttons from HUD Studio</span></label>
      <label className="check"><input type="checkbox" checked={l.consoleLog} onChange={(e) => save({ consoleLog: e.target.checked })} /> <span>Show the game’s console in HUD Studio (<code>-condebug</code>)</span></label>
      <label className="check"><input type="checkbox" checked={l.reloadOnSave} onChange={(e) => save({ reloadOnSave: e.target.checked })} /> <span>Reload the HUD in TF2 after saving (<code>hud_reloadscheme</code>)</span></label>
      <label className="field"><span>Extra launch options</span><input value={extra} placeholder="-novid +exec myconfig" onChange={(e) => setExtra(e.target.value)} onBlur={() => extra !== l.extraArgs && save({ extraArgs: extra })} spellCheck={false} /></label>
    </>
  );
}

// ── deploy ─────────────────────────────────────────────────────────────────
function Deploy() {
  const project = useStore((s) => s.project);
  const app = useStore((s) => s.app);
  const [folder, setFolder] = useState(project?.root.split(/[\\/]/).pop() ?? 'my-hud');
  const [mode, setMode] = useState<InstallMode>(app?.settings.installMode ?? 'link');
  if (!project) return null;
  const run = async () => {
    const r = await guard('Installing…', () => api.deploy(folder, mode));
    if (!r) return;
    close();
    toast(`${installSummary(r)}. In game, run hud_reloadscheme.`, 'ok');
  };
  const exportZip = async () => {
    const { save } = await import('@tauri-apps/plugin-dialog');
    const dest = await save({ defaultPath: `${folder}.zip`, filters: [{ name: 'Zip', extensions: ['zip'] }] });
    if (!dest) return;
    const n = await guard('Zipping…', () => api.exportZip(dest));
    if (n !== undefined) toast(`Exported ${n} files to ${dest}`, 'ok');
  };
  return (
    <Modal title="Install & share" onClose={close}>
      {project.insideTfCustom ? (
        <p>This HUD already lives in <code>tf/custom</code>, so TF2 loads it directly. Save, then type <code>hud_reloadscheme</code> in the game console to see changes (some files need a map change or restart).</p>
      ) : project.game ? (
        <>
          <p>Install this HUD as <code>{project.game.tfDir}/custom/{folder}</code>. Whatever is there now is unlinked, or moved to backups if it’s a real folder.</p>
          <label className="field"><span>Folder name</span><input value={folder} onChange={(e) => setFolder(e.target.value)} /></label>
          <label className="field"><span>Install as</span>
            <Select value={mode} onChange={(e) => setMode(e.target.value as InstallMode)}>
              <option value="link">Link: TF2 reads this folder directly, so every save is live</option>
              <option value="copy">Copy: a snapshot; install again to update it</option>
            </Select></label>
        </>
      ) : <p>Set your TF2 folder in Settings to install HUDs directly.</p>}
      <footer>
        <button className="btn" onClick={() => set({ dialog: { kind: 'hud-details' } })}>HUD details…</button>
        <button className="btn" onClick={() => set({ dialog: { kind: 'library' } })}><Library size={14} /> HUD library</button>
        <button className="btn" onClick={() => void exportZip()}>Export .zip…</button>
        {!project.insideTfCustom && project.game && <button className="btn primary" onClick={() => void run()}>Install to TF2</button>}
      </footer>
    </Modal>
  );
}

// ── external tools ─────────────────────────────────────────────────────────
function Tools() {
  const [tools, setTools] = useState<ExternalTool[]>([]);
  const refresh = () => api.toolsList().then(setTools, (e) => toast(String(e), 'error'));
  useEffect(() => void refresh(), []);
  return (
    <Modal title="External tools" onClose={close} wide>
      <p className="hint">Tools you already have. HUD Studio finds them in the usual install folders, or you can point it at the program. It never downloads or bundles any of them. Your text and image editors are chosen in Settings.</p>
      {(['gui', 'cli'] as const).map((kind) => (
      <React.Fragment key={kind}>
        <h3>{kind === 'gui' ? 'Apps' : 'Command-line tools'}</h3>
      {kind === 'cli' && <p className="hint">Used by Images → Import: ImageMagick turns TGA, JPG, PSD and WebP into PNG first, and VTFCmd (it comes with VTFEdit and VTFEdit Reloaded) makes compressed DXT5 textures. PNG → VTF needs neither.</p>}
      <div className="tools-list">
        {tools.filter((t) => (t.kind ?? 'gui') === kind).map((t) => (
          <div className="tool-row" key={t.id}>
            <div className="tool-info"><strong>{t.name}</strong><span className="muted">{t.about}</span><span className="muted small">{t.path ?? 'Not found. Choose its program to use it.'}</span></div>
            <span className={t.available ? 'pill ok' : 'pill'}>{t.available ? (t.detected ? 'Detected' : 'Registered') : 'Missing'}</span>
            <button className="btn" onClick={async () => { const p = await openDialog({ multiple: false, title: `Locate ${t.name}` }); if (typeof p === 'string') { await guard(null, () => api.toolsRegister(t.id, p)); void refresh(); } }}>Choose…</button>
            {t.kind !== 'cli' && <button className="btn" disabled={!t.available} onClick={() => api.toolsLaunch(t.id).catch((e) => toast(String(e), 'error'))}><ExternalLink size={13} /> Launch</button>}
          </div>
        ))}
      </div>
      </React.Fragment>
      ))}
    </Modal>
  );
}

// ── quick open ─────────────────────────────────────────────────────────────
function QuickOpen() {
  const files = useStore((s) => s.files);
  const model = useStore((s) => s.model);
  const [q, setQ] = useState('');
  const [i, setI] = useState(0);
  const items = useMemo(() => {
    const out: { label: string; detail: string; run: () => void }[] = [];
    const ql = q.toLowerCase();
    walk(model, (el) => {
      if (ql && el.name.toLowerCase().includes(ql)) out.push({ label: el.name, detail: `element · ${el.home.file}`, run: () => set({ selection: [el.uid], view: 'design' }) });
    });
    for (const f of files) if (/\.(res|txt|vdf|vmt|cfg|rc)$/i.test(f.path) && (!ql || f.path.toLowerCase().includes(ql))) out.push({ label: f.path.split('/').pop()!, detail: f.path, run: () => void openFile(f.path) });
    return out.slice(0, 60);
  }, [q, files, model]);
  const go = (n: number) => { items[n]?.run(); close(); };
  return (
    <div className="modal-backdrop top" onPointerDown={close}>
      <section className="palette" onPointerDown={(e) => e.stopPropagation()}>
        <input autoFocus placeholder="Go to file or element…" value={q} onChange={(e) => { setQ(e.target.value); setI(0); }}
          onKeyDown={(e) => { if (e.key === 'ArrowDown') { e.preventDefault(); setI((x) => Math.min(items.length - 1, x + 1)); } if (e.key === 'ArrowUp') { e.preventDefault(); setI((x) => Math.max(0, x - 1)); } if (e.key === 'Enter') go(i); if (e.key === 'Escape') close(); }} />
        <div className="palette-list">{items.map((it, n) => <button key={n} className={n === i ? 'on' : ''} onMouseEnter={() => setI(n)} onClick={() => go(n)}><span>{it.label}</span><span className="muted">{it.detail}</span></button>)}</div>
      </section>
    </div>
  );
}

// ── add element ────────────────────────────────────────────────────────────
function AddElement() {
  const active = useStore((s) => s.active);
  const model = useStore((s) => s.model);
  const selection = useStore((s) => s.selection);
  const [tpl, setTpl] = useState(0);
  const [name, setName] = useState('MyLabel');
  const sel = selection.length === 1 ? selection[0] : null;
  const parentEl = useMemo(() => {
    let found = null as null | ReturnType<typeof Object>;
    walk(model, (e) => { if (e.uid === sel) found = e; });
    return found as unknown as import('../vgui/model.ts').El | null;
  }, [model, sel]);
  const [inside, setInside] = useState(false);
  const canNest = !!parentEl && parentEl.control.includes('editablepanel') && parentEl.home.file === active;
  const add = async () => {
    if (!active) return;
    const doc = await guard(null, () => api.open(active));
    if (!doc) return;
    const root = doc.panels.find((p) => !p.id.includes('/'));
    if (!root) return toast('This file has no root block to add to.', 'error');
    const parentId = inside && canNest && parentEl ? [root.id, ...parentEl.home.path].join('/') : root.id;
    const t = CONTROL_TEMPLATES[tpl];
    const props: [string, string][] = [['ControlName', t.control], ['fieldName', name], ...t.props];
    let newId = '';
    await docEdit(active, async () => {
      const r = await api.insertBlock(active, parentId, name, props);
      newId = r.id;
      return r.document;
    });
    await rebuildModel();
    let uid = '';
    walk(getState().model, (e) => { if (e.home.file === active && [root.id, ...e.home.path].join('/') === newId) uid = e.uid; });
    set({ selection: uid ? [uid] : [], dialog: null });
    toast(`Added ${name}`, 'ok');
  };
  return (
    <Modal title="Add element" onClose={close}>
      <div className="template-grid">
        {CONTROL_TEMPLATES.map((t, n) => (
          <button key={t.label} className={'template' + (n === tpl ? ' on' : '')} onClick={() => { setTpl(n); setName(`My${t.label.replace(/\s/g, '')}`); }}>
            <strong>{t.label}</strong><span className="muted">{t.control}</span>
          </button>
        ))}
      </div>
      <label className="field"><span>Name</span><input value={name} onChange={(e) => setName(e.target.value.replace(/[\s"{}]/g, ''))} autoFocus /></label>
      {canNest && <label className="check"><input type="checkbox" checked={inside} onChange={(e) => setInside(e.target.checked)} /> Put it inside {parentEl!.name}</label>}
      <p className="hint">Added to {active}. Game-created panels only show elements their .res file lists, so add to the file of the HUD element you want it to appear in.</p>
      <footer><button className="btn" onClick={close}>Cancel</button><button className="btn primary" disabled={!name} onClick={() => void add()}>Add</button></footer>
    </Modal>
  );
}

function Ask() {
  const a = useStore((s) => s.ask);
  if (!a) return null;
  return (
    <Modal title="Please confirm" onClose={() => a.resolve(false)}>
      <p>{a.message}</p>
      <footer><button className="btn" onClick={() => a.resolve(false)}>Cancel</button><button className="btn primary" autoFocus onClick={() => a.resolve(true)}>{a.ok}</button></footer>
    </Modal>
  );
}

export function Dialogs() {
  const d = useStore((s) => s.dialog);
  const asking = useStore((s) => !!s.ask);
  if (asking) return <Ask />;
  if (!d) return null;
  switch (d.kind) {
    case 'new-hud': return <NewHud />;
    case 'settings': return <Settings />;
    case 'deploy': return <Deploy />;
    case 'tools': return <Tools />;
    case 'quick-open': return <QuickOpen />;
    case 'add-element': return <AddElement />;
    case 'theme': return <Modal title="Appearance" onClose={close} wide><ThemeDialogBody onClose={close} /></Modal>;
    case 'library': return <Modal title="HUD library" onClose={close} wide><LibraryDialog onClose={close} /></Modal>;
    case 'hud-details': {
      const back = d.path ? () => set({ dialog: { kind: 'library' } }) : close;
      return <Modal title="HUD details" onClose={back}><HudDetailsForm path={d.path} onDone={back} /></Modal>;
    }
    default: return null;
  }
}
