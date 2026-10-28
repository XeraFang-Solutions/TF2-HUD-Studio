import React, { useRef, useEffect, useMemo, useState } from 'react';
import { AlignCenterHorizontal, AlignCenterVertical, AlignEndHorizontal, AlignEndVertical, AlignStartHorizontal, AlignStartVertical, Copy, FileCode2, ImageIcon, Plus, RotateCcw, Trash2 } from 'lucide-react';
import { api } from '../api.ts';
import { ask, docEdit, editElements, getState, openFile, rebuildModel, set, toast, useStore } from '../store.ts';
import { findEl, walk, type El, type PropVal } from '../vgui/model.ts';
import { ALIGNMENTS, hexOf, type SchemeIndex } from '../vgui/scheme.ts';
import { PIN_CORNERS, encodeMove, layoutTree, screenUnits, type LayoutBox } from '../vgui/layout.ts';
import { layoutInputs } from '../vgui/model.ts';
import { PROPS, PROP_GROUPS, controlKind, propInfo, type PropType } from '../vgui/catalog.ts';
import { defaultFor, type DefaultSource } from '../vgui/defaults.ts';
import { useImagePreview } from './Assets.tsx';
import { Select, SuggestInput } from './Select.tsx';

const short = (f: string) => f.replace(/^stock:/, 'TF2 default: ').split('/').slice(-2).join('/');

function Badge({ pv, el, fallback }: { pv?: PropVal; el: El; fallback?: DefaultSource }) {
  if (!pv) return <span className="prov none" title={fallback ? `Not set: TF2 uses ${fallback.value} (${fallback.from})` : 'Not set — the game default applies'}>default</span>;
  const own = pv.file === el.home.file;
  const stock = pv.file.startsWith('stock:') || pv.file.startsWith('tf/') || !pv.blockId;
  return (
    <button className={'prov ' + (stock ? 'stock' : own ? 'own' : 'base')} title={`Defined in ${pv.file} — click to open`} onClick={() => openFile(pv.file.replace(/^stock:|^tf\//, ''), { offset: pv.offset })} disabled={stock}>
      {own ? 'here' : short(pv.file)}
    </button>
  );
}

function ColorEditor({ value, onCommit, scheme, placeholder }: { value: string; onCommit: (v: string) => void; scheme: SchemeIndex; placeholder?: string }) {
  const c = scheme.color(value || placeholder);
  return (
    <span className="color-edit">
      <label className={'swatch' + (!value && placeholder ? ' is-default' : '')} style={{ ['--c' as string]: c ? `rgba(${c[0]},${c[1]},${c[2]},${c[3] / 255})` : 'transparent' }} title={c ? c.join(' ') : 'Unresolved color'}>
        <input
          type="color"
          value={c ? hexOf(c) : '#ffffff'}
          onChange={(e) => {
            const h = e.target.value;
            const rgb = [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
            onCommit(`${rgb.join(' ')} ${c ? c[3] : 255}`);
          }}
        />
      </label>
      <TextEditor value={value} onCommit={onCommit} list="scheme-colors" placeholder={placeholder} />
    </span>
  );
}

function TextEditor({ value, onCommit, list, placeholder }: { value: string; onCommit: (v: string) => void; list?: string; placeholder?: string }) {
  const [draft, setDraft] = useState(value);
  const cancelled = useRef(false);
  useEffect(() => setDraft(value), [value]);
  return (
    <SuggestInput
      className="prop-input"
      value={draft}
      list={list}
      placeholder={placeholder}
      spellCheck={false}
      onChange={setDraft}
      onPick={(v) => v !== value && onCommit(v)}
      onBlur={() => {
        if (!cancelled.current && draft !== value) onCommit(draft);
        cancelled.current = false;
      }}
      onKeyDown={(e) => {
        if (e.key === 'Enter') (e.target as HTMLInputElement).blur();
        if (e.key === 'Escape') {
          cancelled.current = true;
          setDraft(value);
          (e.target as HTMLInputElement).blur();
        }
      }}
    />
  );
}

function ImageEditor({ value, onCommit }: { value: string; onCommit: (v: string) => void }) {
  const img = useImagePreview(value);
  return (
    <span className="image-edit">
      <span className="thumb" title={img ? `${img.texture} · ${img.width}×${img.height} ${img.format}` : 'Not found'}>{img ? <img src={img.dataUrl} alt="" /> : <ImageIcon size={14} />}</span>
      <TextEditor value={value} onCommit={onCommit} />
      <button className="icon-btn" title="Pick from assets" onClick={() => set({ sidebar: 'assets' })}>…</button>
    </span>
  );
}

function Editor({ type, value, onCommit, scheme, siblings, placeholder }: { type: PropType; value: string; onCommit: (v: string) => void; scheme: SchemeIndex; siblings: string[]; placeholder?: string }) {
  switch (type) {
    case 'color':
      return <ColorEditor value={value} onCommit={onCommit} scheme={scheme} placeholder={placeholder} />;
    case 'bool':
      return (
        <label className="toggle">
          <input type="checkbox" checked={(value || placeholder || '').trim() !== '0' && (value || placeholder || '').trim() !== ''} onChange={(e) => onCommit(e.target.checked ? '1' : '0')} />
          <span />
        </label>
      );
    case 'font':
      return <TextEditor value={value} onCommit={onCommit} list="scheme-fonts" placeholder={placeholder} />;
    case 'border':
      return <TextEditor value={value} onCommit={onCommit} list="scheme-borders" placeholder={placeholder} />;
    case 'alignment':
      return (
        <Select className="prop-input" value={ALIGNMENTS.includes(value.toLowerCase()) ? value.toLowerCase() : value} onChange={(e) => onCommit(e.target.value)}>
          {!ALIGNMENTS.includes(value.toLowerCase()) && <option value={value}>{value || '(default west)'}</option>}
          {ALIGNMENTS.map((a) => <option key={a}>{a}</option>)}
        </Select>
      );
    case 'corner':
      return (
        <Select className="prop-input" value={/^\d$/.test(value) ? PIN_CORNERS[+value] : value.toUpperCase()} onChange={(e) => onCommit(e.target.value)}>
          {PIN_CORNERS.map((c) => <option key={c} value={c}>{c.replace('PIN_', '').replace('_', ' ').toLowerCase()}</option>)}
        </Select>
      );
    case 'pin':
      return (
        <>
          <TextEditor value={value} onCommit={onCommit} list="pin-siblings" />
          <datalist id="pin-siblings">{siblings.map((s) => <option key={s} value={s} />)}</datalist>
        </>
      );
    case 'image':
      return <ImageEditor value={value} onCommit={onCommit} />;
    default:
      return <TextEditor value={value} onCommit={onCommit} placeholder={placeholder} />;
  }
}

function blockIdFor(el: El): { file: string; id: string } | null {
  const doc = getState().docs[el.home.file];
  if (!doc) return null;
  const root = doc.panels.find((p) => !p.id.includes('/'));
  if (!root) return null;
  const id = [root.id, ...el.home.path].join('/');
  return doc.panels.some((p) => p.id === id) ? { file: el.home.file, id } : null;
}

export async function deleteSelected() {
  const s = getState();
  const els = s.selection.map((u) => findEl(s.model, u)).filter((e): e is El => !!e);
  if (!els.length) return;
  if (!(await ask(`Delete ${els.length === 1 ? els[0].name : els.length + ' elements'} from ${[...new Set(els.map((e) => e.home.file))].join(', ')}? Undo restores it.`, 'Delete'))) return;
  for (const el of els) {
    const d = await api.open(el.home.file).catch(() => null);
    if (d) set((st) => ({ docs: { ...st.docs, [d.path]: d } }));
    const b = blockIdFor(el);
    if (!b) {
      toast(`${el.name} isn't defined in ${el.home.file} itself (it's inherited), so there is nothing to delete there.`, 'error');
      continue;
    }
    await docEdit(b.file, () => api.removeBlock(b.file, b.id));
  }
  set({ selection: [] });
}

export async function duplicateSelected() {
  const s = getState();
  const el = s.selection.length === 1 ? findEl(s.model, s.selection[0]) : undefined;
  if (!el) return;
  const doc = await api.open(el.home.file).catch(() => null);
  if (doc) set((st) => ({ docs: { ...st.docs, [doc.path]: doc } }));
  const b = blockIdFor(el);
  if (!b) return toast(`${el.name} is inherited from another file; edit it once to create it here, then duplicate.`, 'error');
  let name = el.name + 'Copy';
  let i = 2;
  const taken = new Set<string>();
  walk(s.model, (e) => taken.add(e.name.toLowerCase()));
  while (taken.has(name.toLowerCase())) name = `${el.name}Copy${i++}`;
  const r = await api.duplicateBlock(b.file, b.id, name).catch((e) => (toast(String(e), 'error'), null));
  if (!r) return;
  set((st) => ({ docs: { ...st.docs, [r.document.path]: r.document }, history: [...st.history, r.document.path], future: [] }));
  await rebuildModel();
  const prefix = el.uid.slice(0, el.uid.length - el.name.length);
  set({ selection: [prefix + name] });
}

function AlignTools({ els }: { els: El[] }) {
  const resolution = useStore((s) => s.resolution);
  const model = useStore((s) => s.model);
  const boxes = useMemo(() => {
    const out = new Map<string, LayoutBox>();
    layoutTree(layoutInputs(model, (el, k) => el.props.get(k)?.value), screenUnits(resolution), out);
    return out;
  }, [model, resolution]);
  const align = (mode: string) => {
    const bs = els.map((e) => [e, boxes.get(e.uid)!] as const).filter(([, b]) => b);
    if (bs.length < 2) return;
    const minX = Math.min(...bs.map(([, b]) => b.x)), maxX = Math.max(...bs.map(([, b]) => b.x + b.w));
    const minY = Math.min(...bs.map(([, b]) => b.y)), maxY = Math.max(...bs.map(([, b]) => b.y + b.h));
    const edits: { el: El; key: string; value: string }[] = [];
    bs.forEach(([el, b]) => {
      let x = b.x, y = b.y;
      if (mode === 'left') x = minX;
      if (mode === 'right') x = maxX - b.w;
      if (mode === 'hcenter') x = (minX + maxX) / 2 - b.w / 2;
      if (mode === 'top') y = minY;
      if (mode === 'bottom') y = maxY - b.h;
      if (mode === 'vcenter') y = (minY + maxY) / 2 - b.h / 2;
      const enc = encodeMove(b, el.props.get('xpos')?.value, el.props.get('ypos')?.value, x, y);
      if (x !== b.x) edits.push({ el, key: 'xpos', value: enc.xpos });
      if (y !== b.y) edits.push({ el, key: 'ypos', value: enc.ypos });
    });
    void editElements(edits);
  };
  return (
    <div className="align-tools">
      <button title="Align left edges" onClick={() => align('left')}><AlignStartVertical size={15} /></button>
      <button title="Align horizontal centers" onClick={() => align('hcenter')}><AlignCenterVertical size={15} /></button>
      <button title="Align right edges" onClick={() => align('right')}><AlignEndVertical size={15} /></button>
      <button title="Align top edges" onClick={() => align('top')}><AlignStartHorizontal size={15} /></button>
      <button title="Align vertical centers" onClick={() => align('vcenter')}><AlignCenterHorizontal size={15} /></button>
      <button title="Align bottom edges" onClick={() => align('bottom')}><AlignEndHorizontal size={15} /></button>
    </div>
  );
}

export function Inspector() {
  const selection = useStore((s) => s.selection);
  const model = useStore((s) => s.model);
  const scheme = useStore((s) => s.schemeIndex);
  const active = useStore((s) => s.active);
  const label = useStore((s) => s.modelLabel);
  const [newKey, setNewKey] = useState('');
  const [newVal, setNewVal] = useState('');
  const [filter, setFilter] = useState('');
  const els = selection.map((u) => findEl(model, u)).filter((e): e is El => !!e);
  const el = els.length === 1 ? els[0] : null;

  const siblings = useMemo(() => {
    if (!el) return [];
    let parentChildren: El[] = model;
    walk(model, (e) => {
      if (e.children.some((c) => c.uid === el.uid)) parentChildren = e.children;
    });
    return parentChildren.filter((c) => c.uid !== el.uid).map((c) => c.props.get('fieldname')?.value ?? c.name);
  }, [el, model]);

  const lists = (
    <>
      <datalist id="scheme-colors">{scheme.scheme?.colors.map((c) => <option key={c.name} value={c.name}>{c.raw}</option>)}</datalist>
      <datalist id="scheme-fonts">{scheme.scheme?.fonts.map((f) => <option key={f.name} value={f.name} />)}</datalist>
      <datalist id="scheme-borders">{scheme.scheme?.borders.map((b) => <option key={b.key} value={b.key} />)}</datalist>
      <datalist id="known-props">{Object.keys(PROPS).map((p) => <option key={p} value={p} />)}</datalist>
    </>
  );

  if (!el) {
    return (
      <aside className="inspector">
        {lists}
        <div className="panel-head"><h2>{els.length > 1 ? `${els.length} elements` : 'Inspector'}</h2></div>
        {els.length > 1 ? (
          <>
            <p className="hint">Align the selection. Arrow keys nudge all of them (Shift for 10 units).</p>
            <AlignTools els={els} />
          </>
        ) : (
          <div className="empty-state">
            <p>{active ? 'Click an element on the canvas, or pick one in the Elements list.' : 'Open a .res file to inspect its elements.'}</p>
            {label && <p className="hint">Preview: {label}</p>}
            {active && /\.res$/i.test(active) && <button className="btn" onClick={() => set({ dialog: { kind: 'add-element' } })}><Plus size={14} /> Add element</button>}
          </div>
        )}
      </aside>
    );
  }

  const keys = new Set<string>([...el.props.keys()]);
  const groups = new Map<string, string[]>();
  for (const k of keys) {
    if (filter && !k.includes(filter.toLowerCase())) continue;
    const g = propInfo(k).group;
    groups.set(g, [...(groups.get(g) ?? []), k]);
  }
  // always offer the layout basics, even when unset
  for (const k of ['xpos', 'ypos', 'zpos', 'wide', 'tall', 'visible']) if (!keys.has(k) && (!filter || k.includes(filter))) groups.set('Layout', [...(groups.get('Layout') ?? []), k]);
  // buttons: offer their state colors with the values TF2 falls back to
  const offered = controlKind(el.control) === 'button'
    ? ['defaultfgcolor_override', 'defaultbgcolor_override', 'armedfgcolor_override', 'armedbgcolor_override', ...(el.control.includes('imagebutton') ? ['image_drawcolor', 'image_armedcolor'] : [])]
    : [];
  for (const k of offered) if (!keys.has(k) && (!filter || k.includes(filter.toLowerCase()))) groups.set('Colors', [...(groups.get('Colors') ?? []), k]);
  const fallbackOf = (k: string) => (el.props.has(k) ? undefined : defaultFor(el.control, k, (x) => el.props.get(x)?.value, (n) => scheme.base(n)));
  const commit = (key: string, value: string | null) => void editElements([{ el, key, value }]);
  const homeStock = el.home.file.startsWith('stock:');

  return (
    <aside className="inspector">
      {lists}
      <div className="panel-head">
        <h2 title={el.uid}>{el.name}</h2>
        <div className="head-actions">
          <button className="icon-btn" title="Duplicate (Ctrl+D)" onClick={() => void duplicateSelected()}><Copy size={14} /></button>
          <button className="icon-btn" title="Show in source" onClick={() => {
            const pv = el.props.get('fieldname') ?? el.props.get('controlname') ?? [...el.props.values()][0];
            void openFile(el.home.file.replace(/^stock:/, ''), { offset: pv?.offset });
          }}><FileCode2 size={14} /></button>
          <button className="icon-btn danger" title="Delete (Del)" onClick={() => void deleteSelected()}><Trash2 size={14} /></button>
        </div>
      </div>
      <div className="el-meta">
        <span className="control-tag">{el.props.get('controlname')?.value ?? 'Panel (created by the game)'}</span>
        <span className="muted">{el.synthetic ? `Not in your ${el.home.file.split('/').pop()} — shown with TF2’s built-in defaults. The first edit writes the full block.` : homeStock ? 'From TF2’s default HUD — editing copies the file into your HUD' : `Edits go to ${el.home.file}`}</span>
        {el.alsoFrom?.length ? <span className="muted">Also configured in {el.alsoFrom.map((h) => h.file).join(', ')}</span> : null}
      </div>
      <input className="prop-filter" placeholder="Filter properties" value={filter} onChange={(e) => setFilter(e.target.value)} />
      <div className="props">
        {PROP_GROUPS.filter((g) => groups.has(g)).map((g) => (
          <section key={g} className="prop-group">
            <h3>{g}</h3>
            {groups.get(g)!.sort((a, b) => {
              const order = Object.keys(PROPS);
              const ia = order.indexOf(a), ib = order.indexOf(b);
              return (ia < 0 ? 999 : ia) - (ib < 0 ? 999 : ib) || a.localeCompare(b);
            }).map((k) => {
              const pv = el.props.get(k);
              const info = propInfo(k);
              const canReset = pv && pv.file === el.home.file && pv.editable;
              return (
                <div className="prop-row" key={k} title={info.doc}>
                  <span className="prop-name">{pv?.key ?? k}</span>
                  <span className="prop-editor">
                    <Editor type={info.type} value={pv?.value ?? ''} placeholder={fallbackOf(k)?.value} onCommit={(v) => commit(pv?.key ?? k, v)} scheme={scheme} siblings={siblings} />
                  </span>
                  <span className="prop-meta">
                    <Badge pv={pv} el={el} fallback={fallbackOf(k)} />
                    {canReset && <button className="icon-btn tiny" title="Remove from this file (falls back to inherited/default)" onClick={() => commit(pv!.key, null)}><RotateCcw size={11} /></button>}
                  </span>
                </div>
              );
            })}
          </section>
        ))}
        {el.sub.length > 0 && (
          <section className="prop-group">
            <h3>Nested settings</h3>
            {el.sub.map((b) => (
              <button key={b.key + b.id} className="sub-block" onClick={() => void openFile(b.file.replace(/^stock:/, ''), { offset: b.children?.[0]?.offset })}>
                <span>{b.key}</span>
                <span className="muted">{(b.children ?? []).length} entries · edit in source</span>
              </button>
            ))}
          </section>
        )}
      </div>
      <div className="add-prop">
        <SuggestInput placeholder="property" list="known-props" value={newKey} onChange={setNewKey} spellCheck={false} />
        <input placeholder="value" value={newVal} onChange={(e) => setNewVal(e.target.value)} spellCheck={false} onKeyDown={(e) => {
          if (e.key === 'Enter' && newKey.trim()) {
            commit(newKey.trim(), newVal);
            setNewKey('');
            setNewVal('');
          }
        }} />
        <button className="btn" disabled={!newKey.trim()} onClick={() => {
          commit(newKey.trim(), newVal);
          setNewKey('');
          setNewVal('');
        }}><Plus size={13} /> Add</button>
      </div>
    </aside>
  );
}
