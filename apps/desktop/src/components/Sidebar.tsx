import React, { useEffect, useMemo, useState } from 'react';
import { AlertTriangle, ChevronDown, ChevronRight, Eye, EyeOff, File, FileCode2, FileImage, Folder, Info, Layers, Search, XCircle } from 'lucide-react';
import { api, type SearchHit } from '../api.ts';
import { guard, openFile, refreshProblems, refreshSemantics, set, toast, useStore } from '../store.ts';
import { walk, type El } from '../vgui/model.ts';
import { Assets } from './Assets.tsx';
import { GamePanel } from './Game.tsx';
import { fileMenu, openInTextEditor, showMenu, useAppName } from './External.tsx';

// ── files ──────────────────────────────────────────────────────────────────
type Tree = { name: string; path: string; dirs: Map<string, Tree>; files: { name: string; path: string }[] };

function buildTree(paths: string[]): Tree {
  const root: Tree = { name: '', path: '', dirs: new Map(), files: [] };
  for (const p of paths) {
    const parts = p.split('/');
    let node = root;
    parts.slice(0, -1).forEach((seg, i) => {
      if (!node.dirs.has(seg.toLowerCase())) node.dirs.set(seg.toLowerCase(), { name: seg, path: parts.slice(0, i + 1).join('/'), dirs: new Map(), files: [] });
      node = node.dirs.get(seg.toLowerCase())!;
    });
    node.files.push({ name: parts[parts.length - 1], path: p });
  }
  return root;
}

function fileIcon(name: string) {
  if (/\.(vtf|png|tga|jpg)$/i.test(name)) return <FileImage size={14} />;
  if (/\.(res|txt|vdf|vmt|cfg|rc)$/i.test(name)) return <FileCode2 size={14} />;
  return <File size={14} />;
}

function TreeView({ tree, depth, open, toggle }: { tree: Tree; depth: number; open: Set<string>; toggle: (p: string) => void }) {
  const textName = useAppName('text');
  const imageName = useAppName('image');
  const active = useStore((s) => s.active);
  const docs = useStore((s) => s.docs);
  const problems = useStore((s) => s.problems);
  const counts = useMemo(() => {
    const m = new Map<string, number>();
    for (const p of problems) if (p.severity !== 'info') m.set(p.file, (m.get(p.file) ?? 0) + 1);
    return m;
  }, [problems]);
  const dirs = [...tree.dirs.values()].sort((a, b) => a.name.localeCompare(b.name));
  const files = [...tree.files].sort((a, b) => a.name.localeCompare(b.name));
  return (
    <>
      {dirs.map((d) => (
        <React.Fragment key={d.path}>
          <button className="tree-row dir" style={{ paddingLeft: 8 + depth * 12 }} onClick={() => toggle(d.path)}>
            {open.has(d.path) ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
            <Folder size={14} />
            <span>{d.name}</span>
          </button>
          {open.has(d.path) && <TreeView tree={d} depth={depth + 1} open={open} toggle={toggle} />}
        </React.Fragment>
      ))}
      {files.map((f) => {
        const text = /\.(res|txt|vdf|vmt|cfg|rc|md)$/i.test(f.name);
        const img = /\.vtf$/i.test(f.name);
        return (
          <button
            key={f.path}
            className={'tree-row file' + (active === f.path ? ' active' : '') + (text || img ? '' : ' inert')}
            style={{ paddingLeft: 21 + depth * 12 }}
            title={f.path}
            onClick={() => (text ? void openFile(f.path) : img ? set({ sidebar: 'assets' }) : undefined)}
            onContextMenu={(e) => showMenu(e, fileMenu(f.path, textName, imageName))}
          >
            {fileIcon(f.name)}
            <span>{f.name}</span>
            {docs[f.path]?.dirty && <b className="dot-dirty" title="Unsaved changes" />}
            {counts.get(f.path) ? <span className="count-badge">{counts.get(f.path)}</span> : null}
          </button>
        );
      })}
    </>
  );
}

function CustomizeStock() {
  const project = useStore((s) => s.project);
  const [list, setList] = useState<string[] | null>(null);
  const [q, setQ] = useState('');
  if (!project?.game) return <p className="hint pad">Set your TF2 folder in Settings to customize TF2’s default files.</p>;
  if (!list) return <button className="btn stock-btn" onClick={() => api.stockFiles().then(setList, (e) => toast(String(e), 'error'))}>Customize a TF2 file…</button>;
  const shown = list.filter((f) => !q || f.includes(q.toLowerCase())).slice(0, 80);
  return (
    <div className="stock-picker">
      <label className="search-box"><Search size={13} /><input autoFocus placeholder={`${list.length} TF2 files — filter`} value={q} onChange={(e) => setQ(e.target.value)} /></label>
      <div className="stock-list">
        {shown.map((f) => (
          <button key={f} className="tree-row file" title={`Copy ${f} into your HUD and open it`} onClick={async () => {
            const files = await guard('Copying…', () => api.copyStock(f));
            if (!files) return;
            set({ files });
            await refreshSemantics();
            setList(null);
            await openFile(f);
            toast(`${f} is now part of your HUD`, 'ok');
          }}><FileCode2 size={14} /><span>{f}</span></button>
        ))}
      </div>
      <button className="btn" onClick={() => setList(null)}>Close</button>
    </div>
  );
}

function Files() {
  const files = useStore((s) => s.files);
  const project = useStore((s) => s.project);
  const [filter, setFilter] = useState('');
  const [open, setOpen] = useState<Set<string>>(new Set(['resource', 'resource/ui', 'scripts']));
  const filtered = files.map((f) => f.path).filter((p) => !filter || p.toLowerCase().includes(filter.toLowerCase()));
  const tree = useMemo(() => buildTree(filtered), [filtered.join('\n')]);
  const effectiveOpen = filter ? new Set(filtered.flatMap((p) => p.split('/').slice(0, -1).map((_, i, a) => a.slice(0, i + 1).join('/')))) : open;
  return (
    <>
      <div className="panel-head"><h2>{project?.name ?? 'Files'}</h2></div>
      <label className="search-box"><Search size={13} /><input placeholder="Filter files" value={filter} onChange={(e) => setFilter(e.target.value)} /></label>
      <div className="tree">
        <TreeView tree={tree} depth={0} open={effectiveOpen} toggle={(p) => setOpen((o) => { const n = new Set(o); n.has(p) ? n.delete(p) : n.add(p); return n; })} />
      </div>
      <div className="stock-area"><CustomizeStock /></div>
    </>
  );
}

// ── elements outline ───────────────────────────────────────────────────────
function Outline({ els, depth, filter }: { els: El[]; depth: number; filter: string }) {
  const selection = useStore((s) => s.selection);
  const hidden = useStore((s) => s.hidden);
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const matches = (el: El): boolean => !filter || el.name.toLowerCase().includes(filter) || el.children.some(matches);
  return (
    <>
      {els.filter(matches).map((el) => {
        const sel = selection.includes(el.uid);
        const engineHidden = el.props.get('visible')?.value === '0';
        const isOpen = !collapsed.has(el.uid) || !!filter;
        return (
          <React.Fragment key={el.uid}>
            <div className={'tree-row el' + (sel ? ' active' : '') + (engineHidden ? ' ghost' : '')} style={{ paddingLeft: 6 + depth * 12 }}
              onClick={(e) => set((s) => ({ selection: e.shiftKey || e.ctrlKey ? (sel ? s.selection.filter((u) => u !== el.uid) : [...s.selection, el.uid]) : [el.uid] }))}>
              {el.children.length ? (
                <button className="caret" onClick={(e) => { e.stopPropagation(); setCollapsed((c) => { const n = new Set(c); n.has(el.uid) ? n.delete(el.uid) : n.add(el.uid); return n; }); }}>
                  {isOpen ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                </button>
              ) : <span className="caret" />}
              <span className="el-name">{el.name}</span>
              <span className="el-kind">{el.props.get('controlname')?.value ?? ''}</span>
              <button className="eye" title={hidden.has(el.uid) ? 'Show in preview' : 'Hide in preview (does not change the file)'} onClick={(e) => {
                e.stopPropagation();
                set((s) => { const h = new Set(s.hidden); h.has(el.uid) ? h.delete(el.uid) : h.add(el.uid); return { hidden: h }; });
              }}>{hidden.has(el.uid) ? <EyeOff size={12} /> : <Eye size={12} />}</button>
            </div>
            {isOpen && el.children.length > 0 && <Outline els={el.children} depth={depth + 1} filter={filter} />}
          </React.Fragment>
        );
      })}
    </>
  );
}

function Elements() {
  const model = useStore((s) => s.model);
  const label = useStore((s) => s.modelLabel);
  const [filter, setFilter] = useState('');
  let count = 0;
  walk(model, () => count++);
  return (
    <>
      <div className="panel-head"><h2>Elements</h2><span className="muted">{count}</span></div>
      {label && <p className="hint pad">{label}</p>}
      <label className="search-box"><Search size={13} /><input placeholder="Find element" value={filter} onChange={(e) => setFilter(e.target.value)} /></label>
      <div className="tree">{model.length ? <Outline els={model} depth={0} filter={filter.toLowerCase()} /> : <p className="hint pad">Open a .res file to see its elements.</p>}</div>
    </>
  );
}

// ── search ─────────────────────────────────────────────────────────────────
function SearchPanel() {
  const [q, setQ] = useState('');
  const [regex, setRegex] = useState(false);
  const [cs, setCs] = useState(false);
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [err, setErr] = useState('');
  useEffect(() => {
    const t = setTimeout(async () => {
      if (!q) return setHits([]);
      try {
        setHits(await api.search(q, regex, cs));
        setErr('');
      } catch (e) {
        setErr(String(e));
      }
    }, 220);
    return () => clearTimeout(t);
  }, [q, regex, cs]);
  const byFile = new Map<string, SearchHit[]>();
  for (const h of hits) byFile.set(h.file, [...(byFile.get(h.file) ?? []), h]);
  return (
    <>
      <div className="panel-head"><h2>Search</h2>{hits.length > 0 && <span className="muted">{hits.length >= 2000 ? '2000+' : hits.length}</span>}</div>
      <label className="search-box"><Search size={13} /><input autoFocus placeholder="Search all HUD files" value={q} onChange={(e) => setQ(e.target.value)} /></label>
      <div className="search-opts">
        <label><input type="checkbox" checked={cs} onChange={(e) => setCs(e.target.checked)} /> Match case</label>
        <label><input type="checkbox" checked={regex} onChange={(e) => setRegex(e.target.checked)} /> Regex</label>
      </div>
      {err && <p className="hint pad error-text">{err}</p>}
      <div className="tree">
        {[...byFile].map(([file, hs]) => (
          <div key={file} className="hit-group">
            <div className="hit-file">{file} <span className="muted">{hs.length}</span></div>
            {hs.slice(0, 200).map((h) => (
              <button key={h.offset} className="hit" onClick={() => void openFile(file, { offset: h.offset })}>
                <span className="muted">{h.line}</span>
                <span>{h.preview.slice(0, Math.max(0, h.column - 1 - 30)).length ? '…' : ''}{h.preview.slice(Math.max(0, h.column - 1 - 30), h.column - 1)}<mark>{h.preview.slice(h.column - 1, h.column - 1 + h.length)}</mark>{h.preview.slice(h.column - 1 + h.length)}</span>
              </button>
            ))}
          </div>
        ))}
      </div>
    </>
  );
}

// ── problems ───────────────────────────────────────────────────────────────
function Problems() {
  const problems = useStore((s) => s.problems);
  const textName = useAppName('text');
  const [showInfo, setShowInfo] = useState(false);
  const list = problems.filter((p) => showInfo || p.severity !== 'info');
  const byFile = new Map<string, typeof problems>();
  for (const p of list) byFile.set(p.file, [...(byFile.get(p.file) ?? []), p]);
  return (
    <>
      <div className="panel-head">
        <h2>Problems</h2>
        <button className="icon-btn" title="Check again" onClick={() => void guard('Checking…', refreshProblems)}>↻</button>
      </div>
      <div className="search-opts"><label><input type="checkbox" checked={showInfo} onChange={(e) => setShowInfo(e.target.checked)} /> Show notes ({problems.filter((p) => p.severity === 'info').length})</label></div>
      <div className="tree">
        {list.length === 0 && <p className="hint pad">No problems found. Errors in .res files, undefined colors and fonts, broken #base files, missing images and animation mistakes show up here.</p>}
        {[...byFile].map(([file, ps]) => (
          <div key={file} className="hit-group">
            <div className="hit-file">{file}</div>
            {ps.map((p, i) => (
              <button key={i} className={'problem ' + p.severity} onClick={() => void openFile(file, p.offset !== null ? { offset: p.offset } : { line: p.line })}
                onContextMenu={(e) => showMenu(e, [{ label: `Open in ${textName} at line ${p.line || 1}`, run: () => void openInTextEditor(file, p.line || 1) }])}>
                {p.severity === 'error' ? <XCircle size={13} /> : p.severity === 'warning' ? <AlertTriangle size={13} /> : <Info size={13} />}
                <span>{p.message}</span>
                {p.line > 0 && <span className="muted">:{p.line}</span>}
              </button>
            ))}
          </div>
        ))}
      </div>
    </>
  );
}

export function Sidebar() {
  const which = useStore((s) => s.sidebar);
  return (
    <aside className="sidebar">
      {which === 'files' && <Files />}
      {which === 'elements' && <Elements />}
      {which === 'search' && <SearchPanel />}
      {which === 'problems' && <Problems />}
      {which === 'assets' && <Assets />}
      {which === 'game' && <GamePanel />}
    </aside>
  );
}

export { Layers };
