import React, { useEffect } from 'react';
import { AlertTriangle, Code2, ExternalLink, Eye, Files, FolderOpen, Gamepad2, Grid3x3, ImageIcon, Layers, LayoutPanelTop, Library, Magnet, MonitorPlay, Plus, Redo2, Save, Search, Settings2, Undo2, Upload, Wrench, X } from 'lucide-react';
import { api, inTauri } from './api.ts';
import { CLASSES, type Scenario, closeTab, getState, init, isAnimFile, isRes, isSchemeFile, redo, refreshFiles, refreshSemantics, rebuildModel, saveAll, set, toast, undo, useStore, type Sidebar as SidebarKind, type View } from './store.ts';
import { Canvas } from './components/Canvas.tsx';
import { Inspector, deleteSelected, duplicateSelected } from './components/Inspector.tsx';
import { Sidebar } from './components/Sidebar.tsx';
import { CodeEditor } from './components/CodeEditor.tsx';
import { SchemeEditor } from './components/SchemeEditor.tsx';
import { AnimationPanel } from './components/AnimationPanel.tsx';
import { Dialogs, Welcome, pickAndOpenHud } from './components/Dialogs.tsx';
import { ContextMenu, fileMenu, openInTextEditor, showMenu, useAppName } from './components/External.tsx';
import { cursorLines } from './components/CodeEditor.tsx';
import { ASPECTS } from './vgui/layout.ts';
import { Select } from './components/Select.tsx';
import { PlayButton } from './components/Game.tsx';

function ActivityBar() {
  const sidebar = useStore((s) => s.sidebar);
  const problems = useStore((s) => s.problems.filter((p) => p.severity !== 'info').length);
  const item = (id: SidebarKind, icon: React.ReactNode, label: string, badge?: number) => (
    <button className={sidebar === id ? 'on' : ''} title={label} aria-label={label} onClick={() => set({ sidebar: id })}>
      {icon}
      {badge ? <span className="activity-badge">{badge > 99 ? '99+' : badge}</span> : null}
    </button>
  );
  return (
    <nav className="activity">
      {item('files', <Files size={19} />, 'Files')}
      {item('elements', <Layers size={19} />, 'Elements')}
      {item('search', <Search size={19} />, 'Search (Ctrl+Shift+F)')}
      {item('problems', <AlertTriangle size={19} />, 'Problems', problems)}
      {item('assets', <ImageIcon size={19} />, 'Images')}
      {item('game', <Gamepad2 size={19} />, 'Game console and test commands')}
      <span className="spacer" />
      <button title="HUD library" onClick={() => set({ dialog: { kind: 'library' } })}><Library size={18} /></button>
      <button title="External tools" onClick={() => set({ dialog: { kind: 'tools' } })}><Wrench size={18} /></button>
      <button title="Settings" onClick={() => set({ dialog: { kind: 'settings' } })}><Settings2 size={18} /></button>
    </nav>
  );
}

function Toolbar() {
  const project = useStore((s) => s.project);
  const dirty = useStore((s) => Object.values(s.docs).filter((d) => d.dirty).length);
  const history = useStore((s) => s.history.length);
  const future = useStore((s) => s.future.length);
  const resolution = useStore((s) => s.resolution);
  const zoom = useStore((s) => s.zoom);
  const showHidden = useStore((s) => s.showHidden);
  const showGrid = useStore((s) => s.showGrid);
  const snap = useStore((s) => s.snap);
  const composite = useStore((s) => s.composite);
  const view = useStore((s) => s.view);
  const busy = useStore((s) => s.busy);
  const canvasy = view === 'design' || view === 'animations';
  return (
    <header className="toolbar">
      <div className="brand"><img src="/icon.svg" alt="" /><span>{project?.name ?? 'TF2 HUD Studio'}</span></div>
      <div className="tool-group">
        <button className="tb" onClick={() => void pickAndOpenHud()} title="Open HUD folder"><FolderOpen size={15} /><span>Open</span></button>
        <button className="tb" onClick={() => set({ dialog: { kind: 'new-hud' } })} title="New HUD"><Plus size={15} /><span>New</span></button>
      </div>
      {project && (
        <>
          <div className="tool-group">
            <button className="tb" disabled={!dirty} onClick={() => void saveAll()} title="Save every edited file (Ctrl+S)"><Save size={15} /><span>Save{dirty ? ` ${dirty}` : ''}</span></button>
            <button className="tb icon" disabled={!history} onClick={() => void undo()} title="Undo (Ctrl+Z)"><Undo2 size={15} /></button>
            <button className="tb icon" disabled={!future} onClick={() => void redo()} title="Redo (Ctrl+Y)"><Redo2 size={15} /></button>
          </div>
          {canvasy && (
            <div className="tool-group">
              <Select className="tb-select" value={`${resolution.w}x${resolution.h}`} onChange={(e) => { const [w, h] = e.target.value.split('x').map(Number); set({ resolution: { w, h } }); }} title="Preview resolution">
                {ASPECTS.map((a) => <option key={a.label} value={`${a.w}x${a.h}`}>{a.label}</option>)}
                {!ASPECTS.some((a) => a.w === resolution.w && a.h === resolution.h) && <option value={`${resolution.w}x${resolution.h}`}>{resolution.w} × {resolution.h}</option>}
              </Select>
              <Select className="tb-select narrow" value={String(zoom)} onChange={(e) => set({ zoom: e.target.value === 'fit' ? 'fit' : Number(e.target.value) })} title="Zoom">
                <option value="fit">Fit</option><option value="0.5">50%</option><option value="0.75">75%</option><option value="1">100%</option><option value="1.5">150%</option><option value="2">200%</option>
              </Select>
              <button className={'tb icon' + (showHidden ? ' on' : '')} onClick={() => set({ showHidden: !showHidden })} title="Show elements with visible 0 (the game shows many of them at runtime)"><Eye size={15} /></button>
              <button className={'tb icon' + (showGrid ? ' on' : '')} onClick={() => set({ showGrid: !showGrid })} title="Grid"><Grid3x3 size={15} /></button>
              <Select className={'tb-select narrow' + (snap > 1 ? ' on' : '')} value={snap} onChange={(e) => set({ snap: Number(e.target.value) })} title="Snap moves to a grid (edges and centers of neighbours always snap; hold Alt to drag freely)">
                <option value={1}>Snap 1</option><option value={2}>Snap 2</option><option value={5}>Snap 5</option><option value={10}>Snap 10</option>
              </Select>
              <button className={'tb icon' + (composite ? ' on' : '')} onClick={() => { set({ composite: !composite }); void rebuildModel(); }} title="Show .res files inside their hudlayout container (as in game)"><LayoutPanelTop size={15} /></button>
            </div>
          )}
          <span className="spacer" />
          {busy && <span className="busy">{busy}</span>}
          <PlayButton />
          <button className="tb accent" onClick={() => set({ dialog: { kind: 'deploy' } })} title="Install into TF2 or export a zip"><Upload size={15} /><span>Install</span></button>
        </>
      )}
      {!project && <><span className="spacer" /><PlayButton /><button className="tb" onClick={() => set({ dialog: { kind: 'library' } })} title="HUD library"><Library size={15} /><span>Library</span></button><button className="tb" onClick={() => set({ dialog: { kind: 'settings' } })} title="Settings"><Settings2 size={15} /><span>Settings</span></button></>}
    </header>
  );
}

function viewsFor(path: string): { id: View; label: string; icon: React.ReactNode }[] {
  if (isAnimFile(path)) return [{ id: 'animations', label: 'Timeline', icon: <MonitorPlay size={14} /> }, { id: 'source', label: 'Source', icon: <Code2 size={14} /> }];
  if (isSchemeFile(path)) return [{ id: 'scheme', label: 'Scheme', icon: <Layers size={14} /> }, { id: 'source', label: 'Source', icon: <Code2 size={14} /> }];
  if (isRes(path)) return [{ id: 'design', label: 'Design', icon: <LayoutPanelTop size={14} /> }, { id: 'source', label: 'Source', icon: <Code2 size={14} /> }];
  return [{ id: 'source', label: 'Source', icon: <Code2 size={14} /> }];
}

function Tabs() {
  const tabs = useStore((s) => s.tabs);
  const active = useStore((s) => s.active);
  const docs = useStore((s) => s.docs);
  const view = useStore((s) => s.view);
  const label = useStore((s) => s.modelLabel);
  const scenario = useStore((s) => s.scenario);
  const textName = useAppName('text');
  const imageName = useAppName('image');
  return (
    <div className="tabbar">
      <div className="tabs" role="tablist">
        {tabs.map((t) => (
          <div key={t} className={'tab' + (t === active ? ' on' : '')} role="tab" aria-selected={t === active} title={t}
            onClick={() => { set({ active: t, view: viewsFor(t)[0].id, selection: [] }); void rebuildModel(); }}
            onAuxClick={(e) => e.button === 1 && void closeTab(t)}
            onContextMenu={(e) => showMenu(e, [{ label: 'Close tab', run: () => void closeTab(t) }, ...fileMenu(t, textName, imageName)])}>
            <span>{t.split('/').pop()}</span>
            {docs[t]?.dirty ? <b className="dot-dirty" /> : null}
            <button className="tab-x" onClick={(e) => { e.stopPropagation(); void closeTab(t); }} aria-label={`Close ${t}`}><X size={12} /></button>
          </div>
        ))}
      </div>
      {active && (
        <div className="view-switch">
          {(view === 'design' || view === 'animations') && /hudlayout\.res$|hudanimations/i.test(active) && (
            <Select className="tb-select" value={scenario} onChange={(e) => { set({ scenario: e.target.value as Scenario }); void rebuildModel(); }} title="Which HUD elements the whole-HUD preview shows">
              {CLASSES.map((c) => <option key={c} value={c}>Playing {c[0].toUpperCase() + c.slice(1)}</option>)}
              <option value="all">Every element</option>
            </Select>
          )}
          {view === 'design' && label && <span className="muted preview-label">{label}</span>}
          {view === 'design' && <button className="btn small" onClick={() => set({ dialog: { kind: 'add-element' } })}><Plus size={13} /> Add element</button>}
          {viewsFor(active).map((v) => <button key={v.id} className={view === v.id ? 'chosen' : ''} onClick={() => set({ view: v.id })}>{v.icon}{v.label}</button>)}
          <button title={`Open in ${textName} (Ctrl+Shift+E)`} onClick={() => void openInTextEditor(active, cursorLines.get(active))}><ExternalLink size={14} /></button>
        </div>
      )}
    </div>
  );
}

function Main() {
  const active = useStore((s) => s.active);
  const view = useStore((s) => s.view);
  const project = useStore((s) => s.project);
  const modelError = useStore((s) => s.modelError);
  if (!project) return <main className="main"><Welcome /></main>;
  if (!active) return <main className="main"><div className="empty-state"><p>Pick a file in the explorer. <kbd>Ctrl</kbd>+<kbd>P</kbd> jumps to any file or element.</p></div></main>;
  return (
    <main className="main">
      <Tabs />
      <div className="view">
        {view === 'design' && (modelError ? <div className="empty-state"><p>{modelError}</p></div> : <Canvas />)}
        {view === 'source' && <CodeEditor key={active} path={active} />}
        {view === 'scheme' && <SchemeEditor />}
        {view === 'animations' && <AnimationPanel />}
      </div>
    </main>
  );
}

function StatusBar() {
  const project = useStore((s) => s.project);
  const active = useStore((s) => s.active);
  const doc = useStore((s) => (s.active ? s.docs[s.active] : undefined));
  const problems = useStore((s) => s.problems);
  const selection = useStore((s) => s.selection.length);
  const errors = problems.filter((p) => p.severity === 'error').length;
  const warnings = problems.filter((p) => p.severity === 'warning').length;
  return (
    <footer className="statusbar">
      {project ? (
        <>
          <button className="status-btn" onClick={() => set({ sidebar: 'problems' })}>{errors} errors · {warnings} warnings</button>
          <span>{project.game ? `TF2 files loaded` : 'TF2 folder not set — stock files unavailable'}</span>
          {selection > 0 && <span>{selection} selected</span>}
          <span className="spacer" />
          {doc && <span>{doc.encoding === 'utf8' ? 'UTF-8' : 'UTF-16'}{doc.diagnostics.length ? ` · ${doc.diagnostics.length} syntax issue(s)` : ''}</span>}
          {active && <span className="muted">{active}</span>}
        </>
      ) : <span>Open a HUD to start</span>}
    </footer>
  );
}

function Toasts() {
  const toasts = useStore((s) => s.toasts);
  return <div className="toasts" aria-live="polite">{toasts.map((t) => <div key={t.id} className={'toast ' + t.kind}>{t.text}</div>)}</div>;
}

export function App() {
  const project = useStore((s) => s.project);
  const view = useStore((s) => s.view);
  useEffect(() => {
    if (inTauri) void init();
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = e.ctrlKey || e.metaKey;
      const inEditor = !!(e.target as HTMLElement).closest('.cm-editor');
      const inField = !!(e.target as HTMLElement).closest('input,textarea,select,.select');
      if (mod && e.key.toLowerCase() === 's') { e.preventDefault(); void saveAll(); return; } // designer edits can touch several files
      if (mod && e.key.toLowerCase() === 'p') { e.preventDefault(); if (getState().project) set({ dialog: { kind: 'quick-open' } }); return; }
      if (mod && e.shiftKey && e.key.toLowerCase() === 'f') { e.preventDefault(); set({ sidebar: 'search' }); return; }
      if (mod && e.shiftKey && e.key.toLowerCase() === 'e') { e.preventDefault(); const a = getState().active; if (a) void openInTextEditor(a, cursorLines.get(a)); return; }
      if (inEditor || inField) return;
      if (mod && e.key.toLowerCase() === 'z' && !e.shiftKey) { e.preventDefault(); void undo(); }
      else if (mod && (e.key.toLowerCase() === 'y' || (e.shiftKey && e.key.toLowerCase() === 'z'))) { e.preventDefault(); void redo(); }
      else if (mod && e.key.toLowerCase() === 'd') { e.preventDefault(); void duplicateSelected(); }
      else if (mod && e.key === '0') { e.preventDefault(); set({ zoom: 'fit' }); }
      else if ((e.key === 'Delete' || e.key === 'Backspace') && getState().view === 'design' && getState().selection.length) { e.preventDefault(); void deleteSelected(); }
      else if (e.key === 'Escape') set({ selection: [] });
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  // pick up edits made in other programs when the window regains focus
  useEffect(() => {
    const onFocus = async () => {
      if (!getState().project) return;
      try {
        const status = await api.status();
        const changed = status.filter((s) => s.changedOnDisk);
        for (const c of changed.filter((c) => !c.dirty)) {
          const doc = await api.reload(c.path);
          set((s) => ({ docs: { ...s.docs, [doc.path]: doc } }));
        }
        const conflicts = changed.filter((c) => c.dirty);
        if (conflicts.length) toast(`${conflicts.map((c) => c.path).join(', ')} changed in another program while you have unsaved edits here. Saving is blocked until you reload it.`, 'error');
        if (changed.length) {
          await refreshFiles();
          await refreshSemantics();
          await rebuildModel();
        }
      } catch {
        /* backend not ready */
      }
    };
    window.addEventListener('focus', onFocus);
    return () => window.removeEventListener('focus', onFocus);
  }, []);

  useEffect(() => {
    const before = (e: BeforeUnloadEvent) => {
      if (Object.values(getState().docs).some((d) => d.dirty)) {
        e.preventDefault();
        e.returnValue = '';
      }
    };
    window.addEventListener('beforeunload', before);
    return () => window.removeEventListener('beforeunload', before);
  }, []);

  if (!inTauri) {
    return <div className="app"><div className="welcome"><h1>TF2 HUD Studio</h1><p className="lede">This is the desktop app’s interface. Run it with <code>npm run tauri:dev</code> to work with HUD files.</p></div></div>;
  }

  return (
    <div className="app">
      <Toolbar />
      <div className="workspace">
        {project && <ActivityBar />}
        {project && <Sidebar />}
        <Main />
        {project && view === 'design' && <Inspector />}
      </div>
      <StatusBar />
      <Dialogs />
      <ContextMenu />
      <Toasts />
    </div>
  );
}
