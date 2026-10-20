import React, { useEffect, useMemo, useRef, useState } from 'react';
import { Pause, Play, Plus, Search, Square, Copy } from 'lucide-react';
import { api, type AnimEvent } from '../api.ts';
import { getState, guard, openFile, refreshSemantics, set, setSource, toast, useStore } from '../store.ts';
import { expand, timelineLength, INTERPOLATORS } from '../vgui/anim.ts';
import { Canvas } from './Canvas.tsx';

const INTERP_COLORS: Record<string, string> = { linear: '#f0a43a', accel: '#e0703a', deaccel: '#d9c25b', spline: '#7fb069', pulse: '#bd3b3b', flicker: '#b06fc4', gain: '#5b8db3', bias: '#4fb3a9', bounce: '#e3cfa6' };

function Timeline({ ev }: { ev: AnimEvent }) {
  const animations = useStore((s) => s.animations);
  const playback = useStore((s) => s.playback);
  const cmds = useMemo(() => (animations ? expand(animations.events, ev.name) : []), [animations, ev.name]);
  const length = timelineLength(cmds);
  const rows = useMemo(() => {
    const m = new Map<string, typeof cmds>();
    for (const c of cmds) {
      const key = c.kind === 'Animate' ? `${c.panel} · ${c.property}` : c.kind === 'SetVisible' ? `${c.panel} · visible` : c.panel ? `${c.panel} · ${c.kind}` : c.kind;
      m.set(key, [...(m.get(key) ?? []), c]);
    }
    return [...m];
  }, [cmds]);
  const scale = (t: number) => `${(t / length) * 100}%`;
  const track = useRef<HTMLDivElement>(null);
  const scrub = (clientX: number) => {
    const r = track.current!.getBoundingClientRect();
    const t = Math.max(0, Math.min(length, ((clientX - r.left) / r.width) * length));
    set({ playback: { event: ev.name, t, playing: false } });
  };
  const ticks = [];
  const step = length > 4 ? 1 : length > 1.5 ? 0.25 : 0.1;
  for (let t = 0; t <= length + 1e-6; t += step) ticks.push(t);
  return (
    <div className="timeline">
      <div className="tl-ruler" ref={track} onPointerDown={(e) => { scrub(e.clientX); const mv = (m: PointerEvent) => scrub(m.clientX); window.addEventListener('pointermove', mv); window.addEventListener('pointerup', () => window.removeEventListener('pointermove', mv), { once: true }); }}>
        {ticks.map((t) => <span key={t} className="tick" style={{ left: scale(t) }}>{Math.round(t * 100) / 100}s</span>)}
        {playback?.event === ev.name && <span className="playhead" style={{ left: scale(playback.t) }} />}
      </div>
      <div className="tl-rows">
        {rows.map(([label, list]) => (
          <div key={label} className="tl-row">
            <span className="tl-label" title={label}>{label}</span>
            <span className="tl-track">
              {list.map((c, i) => {
                const interp = (c.interpolator ?? 'linear').toLowerCase();
                const instant = !c.duration;
                return (
                  <button key={i} className={'tl-bar' + (instant ? ' instant' : '') + (c.depth > 0 ? ' nested' : '')}
                    style={{ left: scale(c.at), width: instant ? undefined : scale(c.duration), background: INTERP_COLORS[interp] ?? '#999' }}
                    title={`${c.kind}${c.value !== undefined ? ` → ${c.value}` : ''}${c.interpolator ? ` (${c.interpolator}${c.param ? ' ' + c.param : ''})` : ''} at ${c.at.toFixed(2)}s${c.duration ? ` for ${c.duration}s` : ''}${c.depth ? ` · from ${c.from}` : ''} — click to edit`}
                    onClick={() => {
                      const owner = animations?.events.find((e) => e.name === c.from && !e.shadowedBy);
                      if (owner && owner.editable) void openFile(owner.file, { line: c.line });
                      else toast(`${c.from} comes from ${owner?.file ?? 'TF2'} — use “Override in my HUD” to edit it.`, 'info');
                    }} />
                );
              })}
              {playback?.event === ev.name && <span className="playhead" style={{ left: scale(playback.t) }} />}
            </span>
          </div>
        ))}
        {rows.length === 0 && <p className="hint pad">This event has no commands.</p>}
      </div>
      <div className="tl-legend">{INTERPOLATORS.map((i) => <span key={i}><i style={{ background: INTERP_COLORS[i.toLowerCase()] }} />{i}</span>)}</div>
    </div>
  );
}

export function AnimationPanel() {
  const animations = useStore((s) => s.animations);
  const playback = useStore((s) => s.playback);
  const files = useStore((s) => s.files);
  const [filter, setFilter] = useState('');
  const [selected, setSelected] = useState<string | null>(null);
  const events = useMemo(() => (animations?.events ?? []).filter((e) => !filter || e.name.toLowerCase().includes(filter.toLowerCase()) || e.commands.some((c) => c.panel?.toLowerCase().includes(filter.toLowerCase()))), [animations, filter]);
  const ev = animations?.events.find((e) => e.name === selected && !e.shadowedBy) ?? animations?.events.find((e) => e.name === selected);
  const length = useMemo(() => (ev && animations ? timelineLength(expand(animations.events, ev.name)) : 1), [ev, animations]);

  useEffect(() => {
    if (!playback?.playing) return;
    let raf = 0;
    let last = performance.now();
    const tick = (now: number) => {
      const dt = (now - last) / 1000;
      last = now;
      const p = getState().playback;
      if (!p?.playing) return;
      const t = p.t + dt;
      set({ playback: { ...p, t: t > length ? (getState().app?.settings.ui?.loopAnim === false ? length : 0) : t, playing: t > length ? getState().app?.settings.ui?.loopAnim !== false : true } });
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [playback?.playing, length]);

  const ownFile = animations?.files.find((f) => !f.startsWith('stock:') && files.some((x) => x.path.toLowerCase() === f.toLowerCase()));

  const appendToOwnFile = async (text: string) => {
    if (!ownFile) return toast('Your HUD has no animation file of its own yet. Create scripts/hudanimations_custom.txt and list it first in scripts/hudanimations_manifest.txt.', 'error');
    const doc = await guard(null, () => api.open(ownFile));
    if (!doc) return;
    set((s) => ({ docs: { ...s.docs, [doc.path]: doc } }));
    const nl = doc.source.includes('\r\n') ? '\r\n' : '\n';
    await setSource(ownFile, doc.source.replace(/\s*$/, '') + nl + nl + text.replace(/\r?\n/g, nl) + nl);
    await refreshSemantics();
  };

  const overrideEvent = async (e: AnimEvent) => {
    const r = await guard(null, () => api.fileText(e.file.replace(/^stock:|^tf\//, '')));
    if (!r) return;
    await appendToOwnFile(r[0].slice(e.start, e.end));
    const first = animations?.files[0];
    toast(first === ownFile ? `Copied ${e.name} into ${ownFile}` : `Copied ${e.name} into ${ownFile}. Note: ${first} is listed earlier in the manifest, so its copy still wins.`, first === ownFile ? 'ok' : 'info');
  };

  const newEvent = async () => {
    const name = prompt('Event name', 'MyEvent');
    if (!name) return;
    await appendToOwnFile(`event ${name}\n{\n\tAnimate\tPanelName\tAlpha\t"255"\tLinear\t0.0\t0.2\n}`);
    setSelected(name);
  };

  return (
    <div className="anim-layout">
      <div className="anim-preview"><Canvas /></div>
      <div className="anim-bottom">
        <div className="anim-events">
          <div className="panel-head">
            <h2>Events</h2>
            <button className="icon-btn" title="New event in your HUD's animation file" onClick={() => void newEvent()}><Plus size={14} /></button>
          </div>
          <label className="search-box"><Search size={13} /><input placeholder="Event or panel name" value={filter} onChange={(e) => setFilter(e.target.value)} /></label>
          <div className="tree">
            {events.map((e, i) => (
              <button key={e.name + e.file + i} className={'tree-row file' + (selected === e.name ? ' active' : '') + (e.shadowedBy ? ' ghost' : '')} onClick={() => { setSelected(e.name); set({ playback: { event: e.name, t: 0, playing: false } }); }} title={`${e.file}:${e.line}${e.shadowedBy ? ` — ignored, ${e.shadowedBy} defines it first` : ''}`}>
                <span className={'asset-dot' + (e.editable ? '' : ' stock')} />
                <span>{e.name}</span>
                <span className="muted">{e.commands.length}</span>
              </button>
            ))}
          </div>
          {animations && <p className="hint pad">Load order: {animations.files.map((f) => f.replace(/^stock:/, 'TF2 ')).join(' → ')}. The first file to define an event wins.</p>}
        </div>
        <div className="anim-main">
          {ev ? (
            <>
              <div className="anim-toolbar">
                <strong>{ev.name}</strong>
                <span className="muted">{ev.file}:{ev.line}</span>
                <span className="spacer" />
                <button className="btn" onClick={() => set({ playback: { event: ev.name, t: playback?.event === ev.name && playback.t < length ? playback.t : 0, playing: !(playback?.playing && playback.event === ev.name) } })}>
                  {playback?.playing && playback.event === ev.name ? <><Pause size={13} /> Pause</> : <><Play size={13} /> Play</>}
                </button>
                <button className="btn" onClick={() => set({ playback: null })}><Square size={13} /> Stop</button>
                {ev.editable ? <button className="btn" onClick={() => void openFile(ev.file, { line: ev.line })}>Edit text</button> : <button className="btn" onClick={() => void overrideEvent(ev)}><Copy size={13} /> Override in my HUD</button>}
              </div>
              <Timeline ev={ev} />
            </>
          ) : (
            <div className="empty-state"><p>Pick an event to see its timeline and play it on the HUD above.</p></div>
          )}
        </div>
      </div>
    </div>
  );
}
