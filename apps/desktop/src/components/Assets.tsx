import React, { useEffect, useMemo, useState } from 'react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { Download, ExternalLink, Search, Upload } from 'lucide-react';
import { api, errorText, type ImageResult } from '../api.ts';
import { openInImageEditor, useAppName } from './External.tsx';
import { ask, editElements, getState, guard, refreshFiles, toast, useStore } from '../store.ts';
import { findEl } from '../vgui/model.ts';
import { clearImageCache, useCachedImage } from '../vgui/imageCache.ts';

export function useImagePreview(name: string | undefined): ImageResult | null {
  return useCachedImage(name);
}

/** materials/vgui/replay/x.vmt → replay/x ; materials/hud/x.vmt → ../hud/x */
export function imageNameFor(path: string): string {
  const p = path.replace(/\\/g, '/').replace(/\.(vmt|vtf)$/i, '');
  const m = /^materials\/vgui\/(.*)$/i.exec(p);
  if (m) return m[1];
  return '../' + p.replace(/^materials\//i, '');
}

/** Textures that have a PNG copy open in an image editor this session. */
const pngCopies = new Set<string>();

function ImportForm({ onDone }: { onDone: (vmtPath: string | null) => void }) {
  const [src, setSrc] = useState<string | null>(null);
  const [name, setName] = useState('');
  const [compress, setCompress] = useState(false);
  const [pot, setPot] = useState(false);
  const pick = async () => {
    const p = await openDialog({ multiple: false, title: 'Choose an image', filters: [{ name: 'Images', extensions: ['png', 'tga', 'jpg', 'jpeg', 'bmp', 'psd', 'webp', 'gif', 'vtf'] }] });
    if (typeof p !== 'string') return;
    setSrc(p);
    const stem = p.split(/[\\/]/).pop()!.replace(/\.[^.]+$/, '').toLowerCase().replace(/[^a-z0-9_-]+/g, '_');
    if (!name) setName(`replay/thumbnails/${stem}`);
  };
  const run = async (overwrite = false) => {
    if (!src) return;
    try {
      const r = await api.imageImport(src, { name, compress, powerOfTwo: pot, overwrite });
      toast(`Imported ${r.texture} (${r.width}×${r.height})${r.material ? ' with a new .vmt' : ''}`, 'ok');
      clearImageCache();
      await refreshFiles();
      onDone(`materials/vgui/${r.image}.vmt`);
    } catch (e) {
      const msg = errorText(e);
      if (/already exists/.test(msg)) {
        if (await ask(`${msg}. Replace it?`, 'Replace')) return run(true);
        return;
      }
      toast(msg, 'error');
    }
  };
  return (
    <div className="import-form">
      <button className="btn" onClick={() => void pick()}><Upload size={13} /> {src ? src.split(/[\\/]/).pop() : 'Choose image…'}</button>
      <label className="field"><span>Name (under materials/vgui)</span><input value={name} onChange={(e) => setName(e.target.value)} placeholder="replay/thumbnails/myicon" spellCheck={false} /></label>
      <label className="check"><input type="checkbox" checked={pot} onChange={(e) => setPot(e.target.checked)} /> Scale to a power of two</label>
      <label className="check"><input type="checkbox" checked={compress} onChange={(e) => setCompress(e.target.checked)} /> Compress (DXT5, needs VTFCmd)</label>
      <div className="row-actions"><button className="btn" onClick={() => onDone(null)}>Cancel</button><button className="btn primary" disabled={!src || !name.trim()} onClick={() => void run()}>Import</button></div>
    </div>
  );
}

export function Assets() {
  const [stock, setStock] = useState(false);
  const [list, setList] = useState<{ path: string; stock: boolean }[]>([]);
  const [filter, setFilter] = useState('');
  const [picked, setPicked] = useState<string | null>(null);
  const [importing, setImporting] = useState(false);
  const [, bump] = useState(0);
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
      <div className="panel-head"><h2>Images</h2><span className="muted">{list.length}</span><button className="btn small" disabled={!project} onClick={() => setImporting((v) => !v)} title="Import a PNG, TGA, JPG… as a HUD image"><Upload size={12} /> Import</button></div>
      {importing && <ImportForm onDone={(vmt) => { setImporting(false); if (vmt) { setStock(false); setPicked(vmt); api.assets(false).then(setList, () => undefined); } }} />}
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
              <button className="btn" onClick={() => { pngCopies.add(img.texture); bump((n) => n + 1); void openInImageEditor(img.texture, true); }}><ExternalLink size={13} /> Edit a PNG copy</button>
            </div>
          )}
          {img && pngCopies.has(img.texture) && (
            <div className="row-actions">
              <button className="btn primary" title="Converts your edited PNG copy back into this VTF (an override in your HUD for TF2's own textures)"
                onClick={async () => {
                  const rel = await guard('Converting…', () => api.imagePngBack(img.texture, false));
                  if (rel) {
                    clearImageCache();
                    toast(`Saved the PNG copy into ${rel}`, 'ok');
                    await refreshFiles();
                    const p = picked;
                    setPicked(null);
                    setTimeout(() => setPicked(p), 0);
                  }
                }}><Download size={13} /> Save PNG copy into HUD</button>
            </div>
          )}
        </div>
      )}
    </>
  );
}
