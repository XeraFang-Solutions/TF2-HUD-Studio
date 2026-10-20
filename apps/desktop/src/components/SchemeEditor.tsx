import React, { useMemo, useState } from 'react';
import { Plus, Search } from 'lucide-react';
import { api, type ColorDef, type FontDef } from '../api.ts';
import { docEdit, openFile, set, toast, useStore } from '../store.ts';
import { fontStyle, hexOf, parseRGBA } from '../vgui/scheme.ts';

function ColorRow({ c, color }: { c: ColorDef; color: (raw: string) => [number, number, number, number] | null }) {
  const [draft, setDraft] = useState(c.raw);
  const rgba = color(c.raw);
  const write = (value: string) => {
    if (!c.editable) return toast(`${c.name} comes from ${c.file}; copy that file into your HUD to change it.`, 'error');
    void docEdit(c.file, () => api.setProperty(c.file, c.id, c.name, value, c.offset));
  };
  React.useEffect(() => setDraft(c.raw), [c.raw]);
  return (
    <div className="scheme-row">
      <label className="swatch big" style={{ ['--c' as string]: rgba ? `rgba(${rgba[0]},${rgba[1]},${rgba[2]},${rgba[3] / 255})` : 'transparent' }}>
        <input type="color" disabled={!c.editable} value={rgba ? hexOf(rgba) : '#000000'} onChange={(e) => {
          const h = e.target.value;
          write(`${[1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16)).join(' ')} ${rgba?.[3] ?? 255}`);
        }} />
      </label>
      <span className="scheme-name" title={c.name}>{c.name}</span>
      <input className="prop-input" value={draft} disabled={!c.editable} spellCheck={false} onChange={(e) => setDraft(e.target.value)} onBlur={() => draft !== c.raw && write(draft)} onKeyDown={(e) => e.key === 'Enter' && (e.target as HTMLInputElement).blur()} />
      {rgba && (
        <input className="alpha" type="range" min={0} max={255} value={rgba[3]} disabled={!c.editable || !parseRGBA(c.raw)} title={`Alpha ${rgba[3]}`}
          onChange={(e) => setDraft(`${rgba[0]} ${rgba[1]} ${rgba[2]} ${e.target.value}`)}
          onPointerUp={(e) => write(`${rgba[0]} ${rgba[1]} ${rgba[2]} ${(e.target as HTMLInputElement).value}`)} />
      )}
      <button className="prov own" onClick={() => void openFile(c.file, { offset: c.offset ?? undefined })} title={c.file}>{c.file.split('/').pop()}</button>
    </div>
  );
}

function FontRow({ f, screenPx, sample }: { f: FontDef; screenPx: number; sample: string }) {
  const scheme = useStore((s) => s.schemeIndex);
  useStore((s) => s.fontsVersion);
  const props = scheme.font(f.name, screenPx);
  const style = fontStyle(props);
  const pxPerUnit = screenPx / 480;
  const variant = f.variants.find((v) => v.props === props) ?? f.variants[0];
  const edit = (key: string, value: string) => {
    if (!f.editable || !variant) return toast(`${f.name} comes from ${f.file}; copy that file into your HUD to change it.`, 'error');
    void docEdit(f.file, () => api.setProperty(f.file, `${f.id}/${variant.key}`, key, value));
  };
  return (
    <div className="font-row">
      <div className="font-head">
        <span className="scheme-name">{f.name}</span>
        <span className="muted">{props?.name ?? '—'}</span>
        <label className="mini">tall <input className="prop-input narrow" defaultValue={props?.tall ?? ''} key={props?.tall} onBlur={(e) => e.target.value !== props?.tall && edit('tall', e.target.value)} disabled={!f.editable} /></label>
        <label className="mini"><input type="checkbox" checked={props?.outline === '1'} onChange={(e) => edit('outline', e.target.checked ? '1' : '0')} disabled={!f.editable} /> outline</label>
        <label className="mini"><input type="checkbox" checked={props?.dropshadow === '1'} onChange={(e) => edit('dropshadow', e.target.checked ? '1' : '0')} disabled={!f.editable} /> shadow</label>
        <button className="prov own" onClick={() => void openFile(f.file)} title={f.file}>{f.file.split('/').pop()}</button>
      </div>
      <div className="font-sample" style={{ fontFamily: style.family, fontSize: style.sizeUnits * pxPerUnit, fontWeight: style.weight, fontStyle: style.italic ? 'italic' : undefined, textShadow: style.outline ? '-1px 0 #000,1px 0 #000,0 -1px #000,0 1px #000' : style.shadow ? '1px 1px 0 #000' : undefined }}>
        {sample}
      </div>
    </div>
  );
}

export function SchemeEditor() {
  const scheme = useStore((s) => s.scheme);
  const index = useStore((s) => s.schemeIndex);
  const resolution = useStore((s) => s.resolution);
  const [tab, setTab] = useState<'colors' | 'fonts' | 'borders' | 'files'>('colors');
  const [filter, setFilter] = useState('');
  const [sample, setSample] = useState('150  32 / 6  BLU wins!');
  const [newName, setNewName] = useState('');
  const colors = useMemo(() => (scheme?.colors ?? []).filter((c) => !filter || c.name.toLowerCase().includes(filter.toLowerCase()) || c.raw.includes(filter)), [scheme, filter]);
  const fonts = useMemo(() => (scheme?.fonts ?? []).filter((f) => !filter || f.name.toLowerCase().includes(filter.toLowerCase())), [scheme, filter]);
  if (!scheme) return <div className="empty-state"><p>Loading scheme…</p></div>;
  const editableColorsFile = scheme.colors.find((c) => c.editable)?.file;
  const addColor = async () => {
    const name = newName.trim();
    if (!name || !editableColorsFile) return;
    const holder = scheme.colors.find((c) => c.file === editableColorsFile)!;
    await docEdit(holder.file, () => api.setProperty(holder.file, holder.id, name, '255 255 255 255'));
    setNewName('');
  };
  return (
    <div className="scheme-editor">
      <div className="subtabs">
        {(['colors', 'fonts', 'borders', 'files'] as const).map((t) => (
          <button key={t} className={tab === t ? 'chosen' : ''} onClick={() => setTab(t)}>
            {t === 'colors' ? `Colors ${scheme.colors.length}` : t === 'fonts' ? `Fonts ${scheme.fonts.length}` : t === 'borders' ? `Borders ${scheme.borders.length}` : 'Scheme files'}
          </button>
        ))}
        <label className="search-box inline"><Search size={13} /><input placeholder="Filter" value={filter} onChange={(e) => setFilter(e.target.value)} /></label>
      </div>
      <div className="scheme-body">
        {tab === 'colors' && (
          <>
            <div className="scheme-add">
              <input placeholder="New color name" value={newName} onChange={(e) => setNewName(e.target.value)} onKeyDown={(e) => e.key === 'Enter' && void addColor()} />
              <button className="btn" disabled={!newName.trim() || !editableColorsFile} onClick={() => void addColor()}><Plus size={13} /> Add color{editableColorsFile ? ` to ${editableColorsFile.split('/').pop()}` : ''}</button>
            </div>
            <div className="scheme-grid">{colors.map((c) => <ColorRow key={c.name} c={c} color={(raw) => index.color(raw)} />)}</div>
          </>
        )}
        {tab === 'fonts' && (
          <>
            <div className="scheme-add"><input value={sample} onChange={(e) => setSample(e.target.value)} placeholder="Sample text" /><span className="muted">Shown at {resolution.w}×{resolution.h}</span></div>
            {scheme.fontFiles.length > 0 && (
              <p className="hint">Custom font files: {scheme.fontFiles.map((f) => `${f.declaredName ?? f.path}${f.found ? '' : ' (missing)'}`).join(', ')}</p>
            )}
            <div>{fonts.slice(0, 300).map((f) => <FontRow key={f.name} f={f} screenPx={resolution.h} sample={sample} />)}</div>
          </>
        )}
        {tab === 'borders' && (
          <div className="scheme-grid">
            {scheme.borders.filter((b) => !filter || b.key.toLowerCase().includes(filter.toLowerCase())).map((b) => (
              <div key={b.key} className="scheme-row">
                <span className="border-chip" style={{ borderColor: index.css(b.children?.find((c) => c.key.toLowerCase() === 'left')?.children?.[0]?.children?.find((c) => c.key.toLowerCase() === 'color')?.value) ?? 'transparent' }} />
                <span className="scheme-name">{b.key}</span>
                <span className="muted">{b.children?.find((c) => c.key.toLowerCase() === 'image')?.value ?? (b.children?.find((c) => c.key.toLowerCase() === 'backgroundtype') ? `backgroundtype ${b.children.find((c) => c.key.toLowerCase() === 'backgroundtype')!.value}` : 'line border')}</span>
                <button className="prov own" onClick={() => void openFile(b.file.replace(/^stock:/, ''), { offset: b.children?.[0]?.offset })}>{b.file.split('/').pop()}</button>
              </div>
            ))}
          </div>
        )}
        {tab === 'files' && (
          <div className="scheme-files">
            <p className="hint">TF2 loads resource/clientscheme.res, then every #base file it lists. Earlier files win when two define the same name.</p>
            {scheme.chain.map((f, i) => (
              <button key={f} className="tree-row file" onClick={() => f.startsWith('stock:') ? undefined : void openFile(f, { view: 'source' })}>
                <span className="muted">{i + 1}</span><span>{f}</span>
              </button>
            ))}
            <button className="btn" onClick={() => set({ view: 'source' })}>Edit this file as text</button>
          </div>
        )}
      </div>
    </div>
  );
}
