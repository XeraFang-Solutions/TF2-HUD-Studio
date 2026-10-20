import React, { useEffect, useMemo, useState } from 'react';
import { ExternalLink, Search } from 'lucide-react';
import { api, type ImageResult } from '../api.ts';
import { openInImageEditor, useAppName } from './External.tsx';
import { editElements, getState, toast, useStore } from '../store.ts';
import { findEl } from '../vgui/model.ts';

const cache = new Map<string, ImageResult | null>();

export function useImagePreview(name: string | undefined): ImageResult | null {
  const [, force] = useState(0);
  const key = (name ?? '').toLowerCase();
  useEffect(() => {
    if (!key || cache.has(key)) return;
    let live = true;
    api.image(name!).then((r) => cache.set(key, r), () => cache.set(key, null)).finally(() => live && force((n) => n + 1));
    return () => { live = false; };
  }, [key]);
  return key ? cache.get(key) ?? null : null;
}

/** materials/vgui/replay/x.vmt → replay/x ; materials/hud/x.vmt → ../hud/x */
export function imageNameFor(path: string): string {
  const p = path.replace(/\\/g, '/').replace(/\.(vmt|vtf)$/i, '');
  const m = /^materials\/vgui\/(.*)$/i.exec(p);
  if (m) return m[1];
  return '../' + p.replace(/^materials\//i, '');
}

export function Assets() {
  const [stock, setStock] = useState(false);
  const [list, setList] = useState<{ path: string; stock: boolean }[]>([]);
  const [filter, setFilter] = useState('');
  const [picked, setPicked] = useState<string | null>(null);
  const imageName = useAppName('image');
  const project = useStore((s) => s.project);
  const selection = useStore((s) => s.selection);
  useEffect(() => {
    api.assets(stock).then(setList, (e) => toast(String(e), 'error'));
  }, [stock, project?.root]);
  const shown = useMemo(() => list.filter((a) => !filter || a.path.toLowerCase().includes(filter.toLowerCase())).slice(0, 400), [list, filter]);
  const name = picked ? imageNameFor(picked) : undefined;
  const img = useImagePreview(name);
  const pickedEntry = list.find((a) => a.path === picked);
  const useOnSelection = async () => {
    const s = getState();
    const els = s.selection.map((u) => findEl(s.model, u)).filter((e) => !!e);
    if (!els.length || !name) return;
    await editElements(els.map((el) => ({ el: el!, key: 'image', value: name })));
    toast(`Set image on ${els.length} element${els.length > 1 ? 's' : ''}`, 'ok');
  };
  return (
    <>
      <div className="panel-head"><h2>Images</h2><span className="muted">{list.length}</span></div>
      <label className="search-box"><Search size={13} /><input placeholder="Filter materials" value={filter} onChange={(e) => setFilter(e.target.value)} /></label>
      <div className="search-opts"><label><input type="checkbox" checked={stock} onChange={(e) => setStock(e.target.checked)} /> Include TF2’s own vgui materials</label></div>
      <div className="tree assets-list">
        {shown.map((a) => (
          <button key={a.path} className={'tree-row file' + (picked === a.path ? ' active' : '')} onClick={() => setPicked(a.path)} title={a.path}>
            <span className={'asset-dot' + (a.stock ? ' stock' : '')} />
            <span>{imageNameFor(a.path)}</span>
          </button>
        ))}
        {list.length === 0 && <p className="hint pad">No .vmt materials in this HUD{stock ? ' or TF2' : ''}. Turn on TF2’s materials to browse the stock icons.</p>}
      </div>
      {picked && (
        <div className="asset-preview">
          <div className="checker">{img ? <img src={img.dataUrl} alt="" /> : <span className="muted">{img === null ? 'Loading or not decodable…' : ''}</span>}</div>
          <code>{name}</code>
          {img && <span className="muted">{img.width}×{img.height} · {img.format} · {img.origin}</span>}
          <div className="row-actions">
            <button className="btn primary" disabled={!selection.length} onClick={() => void useOnSelection()}>Use on selected</button>
            <button className="btn" onClick={() => { void navigator.clipboard.writeText(name ?? ''); toast('Image name copied', 'ok'); }}>Copy name</button>
          </div>
          {img && (
            <div className="row-actions">
              {!pickedEntry?.stock && <button className="btn" onClick={() => void openInImageEditor(img.texture, false)}><ExternalLink size={13} /> Edit in {imageName}</button>}
              <button className="btn" onClick={() => void openInImageEditor(img.texture, true)}><ExternalLink size={13} /> Edit a PNG copy</button>
            </div>
          )}
        </div>
      )}
    </>
  );
}
