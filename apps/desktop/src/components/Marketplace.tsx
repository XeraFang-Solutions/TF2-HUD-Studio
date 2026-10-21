import React, { useEffect, useRef, useState } from 'react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { Download, ExternalLink, RefreshCw, Search } from 'lucide-react';
import { api, errorText, type ExternalTool, type LibraryHud, type MarketplaceCatalogue, type MarketplaceEntry } from '../api.ts';
import { toast } from '../store.ts';

function open(url: string) { void api.marketplaceOpen(url).catch((e) => toast(errorText(e), 'error')); }

// Share in-flight requests across cards and keep at most three album lookups running at once.
const previews = new Map<string, Promise<string | null>>();
const previewQueue: (() => void)[] = [];
let activePreviews = 0;
function previewFor(h: MarketplaceEntry): Promise<string | null> {
  const cached = previews.get(h.id);
  if (cached) return cached;
  const result = new Promise<string | null>((resolve) => {
    const run = () => {
      activePreviews++;
      void api.marketplacePreview(h.id).then(resolve, () => resolve(null)).finally(() => {
        activePreviews--;
        previewQueue.shift()?.();
      });
    };
    if (activePreviews < 3) run(); else previewQueue.push(run);
  });
  if (previews.size >= 60) previews.delete(previews.keys().next().value!);
  previews.set(h.id, result);
  return result;
}

function AlbumPreview({ hud }: { hud: MarketplaceEntry }) {
  const target = useRef<HTMLButtonElement>(null);
  const [src, setSrc] = useState<string | null>(null);
  const [done, setDone] = useState(false);
  useEffect(() => {
    let live = true;
    const observer = new IntersectionObserver((entries) => {
      if (!entries.some((e) => e.isIntersecting)) return;
      observer.disconnect();
      void previewFor(hud).then((url) => { if (live) { setSrc(url); setDone(true); } });
    }, { rootMargin: '150px' });
    if (target.current) observer.observe(target.current);
    return () => { live = false; observer.disconnect(); };
  }, [hud.id]);
  return <button ref={target} className="market-preview" onClick={() => hud.album && open(hud.album)} title={`Open ${hud.name} album`} aria-label={`Open ${hud.name} album`}>
    {src ? <img src={src} alt={`${hud.name} album preview`} loading="lazy" onError={() => setSrc(null)} /> : <span className="muted">{done ? 'Preview unavailable' : 'Loading album preview…'}</span>}
  </button>;
}

export function Marketplace({ library, huds, onImported }: { library: string | null; huds: LibraryHud[]; onImported: () => Promise<void> }) {
  const [catalogue, setCatalogue] = useState<MarketplaceCatalogue | null>(null);
  const [tools, setTools] = useState<ExternalTool[]>([]);
  const [query, setQuery] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [importing, setImporting] = useState<string | null>(null);
  const [page, setPage] = useState(1);
  const live = useRef(true);
  const refreshTools = async () => {
    try { const t = await api.toolsList(); if (live.current) setTools(t.filter((t) => t.id === '7zip' || t.id === 'peazip-7z')); }
    catch (e) { if (live.current) toast(errorText(e), 'error'); }
  };
  const refresh = async () => {
    setLoading(true); setError('');
    try { const c = await api.marketplaceList(); if (live.current) setCatalogue(c); }
    catch (e) { if (live.current) setError(errorText(e)); }
    finally { if (live.current) setLoading(false); }
  };
  useEffect(() => {
    live.current = true;
    void refresh(); void refreshTools();
    return () => { live.current = false; };
  }, []);
  const chooseTool = async (id: string) => {
    const p = await openDialog({ title: 'Locate the 7-Zip command-line executable (7z.exe, 7zz or 7z)', multiple: false });
    if (typeof p !== 'string') return;
    try { await api.toolsRegister(id, p); await refreshTools(); }
    catch (e) { toast(errorText(e), 'error'); }
  };
  const download = async (h: MarketplaceEntry) => {
    if (importing) return;
    setImporting(h.id);
    try {
      await api.marketplaceImport(h.id);
      await onImported();
      toast(`${h.name} added to your library. Use My HUDs to open or install it.`, 'ok');
    } catch (e) { toast(errorText(e), 'error'); }
    finally { if (live.current) setImporting(null); }
  };
  const filtered = (catalogue?.entries ?? []).filter((h) => `${h.name} ${h.author}`.toLowerCase().includes(query.trim().toLowerCase()));
  const extractor = tools.find((t) => t.available);
  const pages = Math.max(1, Math.ceil(filtered.length / 30));
  const currentPage = Math.min(page, pages);
  return (
    <section className="marketplace" aria-label="HUD marketplace">
      <p className="hint">Browse Hypnootize's updated HUD list. Downloads are added to your library with a manifest when missing. Choose “Use this HUD” in My HUDs to install one. Listing here reflects the archive's classification; compatibility can vary.</p>
      <div className="market-tools">
        <div className="tool-info"><strong>Archive support</strong><span className="muted">ZIP: built in. 7z / RAR: {extractor ? `ready via ${extractor.name}` : "requires 7-Zip or PeaZip's 7-Zip backend"}.</span>
          {extractor?.path && <span className="muted small">{extractor.path}</span>}
        </div>
        <div className="row-actions">
          <button className="btn" onClick={() => void refreshTools()} disabled={!!importing}>Detect again</button>
          <button className="btn" onClick={() => void chooseTool('7zip')} disabled={!!importing}>Locate 7-Zip…</button>
          <button className="btn" onClick={() => void chooseTool('peazip-7z')} disabled={!!importing}>Locate PeaZip backend…</button>
          {!extractor && <><button className="btn" onClick={() => open('https://www.7-zip.org/')}>Get 7-Zip (recommended) <ExternalLink size={12} /></button>
            <button className="btn" onClick={() => open('https://peazip.github.io/')}>Get PeaZip <ExternalLink size={12} /></button></>}
        </div>
      </div>
      <div className="market-toolbar">
        <label className="field-row grow"><Search size={16} /><input aria-label="Search marketplace HUDs" placeholder="Search HUDs or creators…" value={query} onChange={(e) => { setQuery(e.target.value); setPage(1); }} /></label>
        <span className="muted">{filtered.length} HUDs{catalogue?.cached ? ' · saved list' : ''}</span>
        <button className="btn" disabled={loading || !!importing} onClick={() => void refresh()}><RefreshCw size={13} /> {loading ? 'Loading…' : 'Refresh list'}</button>
        <button className="btn" onClick={() => open('https://github.com/Hypnootize/TF2-HUDs-Archive/blob/master/Updated%20HUDs%20List.md')}>Source <ExternalLink size={12} /></button>
      </div>
      {!library && <p className="hint">Choose a library folder above to enable downloads.</p>}
      {catalogue?.warning && <p className="hint" role="status">{catalogue.warning}</p>}
      {error && <p className="market-error" role="alert">{error}</p>}
      {importing && <p className="hint" role="status">Downloading and importing {catalogue?.entries.find((h) => h.id === importing)?.name}… You can keep working while it finishes.</p>}
      {!loading && catalogue && filtered.length === 0 && <p className="hint">No HUDs match your search.</p>}
      <div className="lib-grid">
        {filtered.slice((currentPage - 1) * 30, currentPage * 30).map((h) => {
          const present = huds.some((installed) => h.repository ? installed.details.manifest.source === h.repository : installed.name === h.name);
          const needsTool = /\.(7z|rar)(?:[?#]|$)/i.test(h.download ?? '');
          return <article className="lib-card market-card" key={h.id}>
            {h.album && <AlbumPreview hud={h} />}
            <div className="lib-main"><span className="hud-icon blank">{h.name.slice(0, 2).toUpperCase()}</span><div className="lib-text"><strong>{h.name}</strong><span className="muted">{h.author || 'Unknown creator'}</span></div></div>
            <div className="lib-state">{present && <span className="pill ok">In library</span>}{needsTool && !extractor && <span className="pill">Archive tool needed</span>}</div>
            <div className="lib-actions">
              {h.album && <button className="btn" onClick={() => open(h.album!)}>Album <ExternalLink size={12} /></button>}
              {h.repository && <button className="btn" onClick={() => open(h.repository!)}>{h.repository.startsWith('https://github.com/') ? 'GitHub' : 'Repository'} <ExternalLink size={12} /></button>}
              {h.download && <><button className="btn primary" disabled={!library || !!importing || (needsTool && !extractor) || present} onClick={() => void download(h)}><Download size={13} /> {importing === h.id ? 'Importing…' : present ? 'Added' : 'Add to library'}</button><button className="btn" title="Open the original download link" onClick={() => open(h.download!)}>Download <ExternalLink size={12} /></button></>}
            </div>
          </article>;
        })}
      </div>
      {pages > 1 && <div className="market-pagination"><button className="btn" disabled={currentPage === 1} onClick={() => setPage(currentPage - 1)}>Previous</button><span>Page {currentPage} of {pages}</span><button className="btn" disabled={currentPage === pages} onClick={() => setPage(currentPage + 1)}>Next</button></div>}
    </section>
  );
}
