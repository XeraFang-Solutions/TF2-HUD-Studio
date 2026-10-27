import React, { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { editElements, getState, set, useStore } from '../store.ts';
import { clientToUnits, computePos, encodeMove, encodeSize, layoutTree, screenUnits, type LayoutBox, type Rect } from '../vgui/layout.ts';
import { findEl, layoutInputs, toEl, walk, type El } from '../vgui/model.ts';
import { alignment, borderSides, fontStyle, imageBorder, sampleText, type RGBA, type SchemeIndex } from '../vgui/scheme.ts';
import { controlKind } from '../vgui/catalog.ts';
import { evaluate, expand } from '../vgui/anim.ts';
import { useCachedImage as useImage } from '../vgui/imageCache.ts';

// ── image cache ────────────────────────────────────────────────────────────

// ── value lookup with drag + animation overrides ───────────────────────────
type Overrides = Map<string, Record<string, string>>;

/** Panels the game switches on at runtime even though their files say visible 0. */
const RUNTIME_SHOWN = new Set(['objectivestatustimepanel']);
/** Conditional blocks TF2 applies on an ordinary (casual/competitive) server. */
const ACTIVE_CONDITIONS = new Set(['if_match']);

function makeValue(drag: Overrides, anim: Map<string, Map<string, string>> | null) {
  return (el: El, key: string): string | undefined => {
    const k = key.toLowerCase();
    const d = drag.get(el.uid)?.[k];
    if (d !== undefined) return d;
    if (anim) {
      const a = anim.get((el.props.get('fieldname')?.value ?? el.name).toLowerCase()) ?? anim.get(el.name.toLowerCase());
      const v = a?.get(k);
      if (v !== undefined) return v;
    }
    if (k === 'visible' && RUNTIME_SHOWN.has(el.name.toLowerCase())) return '1';
    for (const sub of el.sub) {
      if (!ACTIVE_CONDITIONS.has(sub.key.toLowerCase())) continue;
      const hit = sub.children?.find((c) => c.value !== undefined && c.key.toLowerCase() === k);
      if (hit) return hit.value;
    }
    return el.props.get(k)?.value;
  };
}

type Ctx = {
  k: number; // px per unit
  value: (el: El, key: string) => string | undefined;
  boxes: Map<string, LayoutBox>;
  scheme: SchemeIndex;
  screenPx: number;
  showHidden: boolean;
  hidden: Set<string>;
  localized: Record<string, string>;
  selection: Set<string>;
  onDown: (e: React.PointerEvent, el: El) => void;
};

const truthy = (v: string | undefined) => v !== undefined && v.trim() !== '' && v.trim() !== '0';

function paints(el: El, ctx: Ctx): boolean {
  const v = (k: string) => ctx.value(el, k);
  if (['huddeathnotice', 'hudchathistory'].includes(el.name.toLowerCase()) || el.control === 'ctfteamstatus') return true;
  const kind = controlKind(el.control);
  if (kind === 'label' || kind === 'button' || kind === 'richtext') return !!v('labeltext') || !!v('bgcolor_override');
  if (kind === 'image') return !!(v('image') || v('fillcolor') || v('teambg_2'));
  if (kind === 'progress' || kind === 'model' || kind === 'avatar') return true;
  return !!(v('bgcolor_override') && v('paintbackground') !== '0') || !!v('border');
}

function ImageLayer({ name, tint, scale, tile, k, w, h }: { name: string; tint: RGBA | null; scale: boolean; tile: boolean; k: number; w: number; h: number }) {
  const img = useImage(name);
  if (!img) return null;
  const natural = { w: (img.width * 480) / 1080, h: (img.height * 480) / 1080 };
  const style: React.CSSProperties = scale || tile ? { position: 'absolute', inset: 0 } : { position: 'absolute', left: 0, top: 0, width: natural.w * k, height: natural.h * k };
  const white = !tint || (tint[0] === 255 && tint[1] === 255 && tint[2] === 255);
  const opacity = tint ? tint[3] / 255 : 1;
  const base: React.CSSProperties = tile ? { ...style, backgroundImage: `url(${img.dataUrl})`, backgroundRepeat: 'repeat', backgroundSize: `${natural.w * k}px ${natural.h * k}px`, opacity } : { ...style, opacity };
  return (
    <>
      {tile ? <div style={base} /> : <img src={img.dataUrl} style={{ ...base, width: style.width ?? '100%', height: style.height ?? '100%' }} draggable={false} alt="" />}
      {!white && (
        <div
          style={{
            ...style,
            background: `rgb(${tint![0]},${tint![1]},${tint![2]})`,
            mixBlendMode: 'multiply',
            WebkitMaskImage: `url(${img.dataUrl})`,
            WebkitMaskSize: tile ? `${natural.w * k}px ${natural.h * k}px` : '100% 100%',
            WebkitMaskRepeat: tile ? 'repeat' : 'no-repeat',
            opacity,
          }}
        />
      )}
    </>
  );
}

/**
 * A 9-slice image (ScalableImagePanel / CTFImagePanel corners, or a `scalable_image` scheme border): the source corners
 * (texture pixels) are drawn at a fixed size (units) and the edges and center stretch to fill the panel.
 */
function NineSlice({ name, src, draw, tint, k }: { name: string; src: [number, number]; draw: [number, number]; tint: RGBA | null; k: number }) {
  const img = useImage(name);
  if (!img) return null;
  const sw = Math.min(src[0], img.width / 2);
  const sh = Math.min(src[1], img.height / 2);
  const widths = `${draw[1] * k}px ${draw[0] * k}px`;
  const url = `url(${img.dataUrl})`;
  const box: React.CSSProperties = { position: 'absolute', inset: 0, boxSizing: 'border-box', borderStyle: 'solid', borderColor: 'transparent', borderWidth: widths, pointerEvents: 'none' };
  const white = !tint || (tint[0] === 255 && tint[1] === 255 && tint[2] === 255);
  const opacity = tint ? tint[3] / 255 : 1;
  return (
    <>
      <div style={{ ...box, borderImageSource: url, borderImageSlice: `${sh} ${sw} fill`, borderImageRepeat: 'stretch', opacity }} />
      {!white && (
        <div
          style={{
            position: 'absolute', inset: 0, pointerEvents: 'none', opacity, mixBlendMode: 'multiply',
            background: `rgb(${tint![0]},${tint![1]},${tint![2]})`,
            WebkitMaskBoxImage: `${url} ${sh} ${sw} / ${widths} stretch`,
          } as React.CSSProperties}
        />
      )}
    </>
  );
}

const num = (v: string | undefined, d: number) => { const n = parseFloat(v ?? ''); return Number.isFinite(n) ? n : d; };

const SAMPLE_KILLS = [
  { killer: 'Pyrotechnic', kTeam: 'red', victim: 'Spy', vTeam: 'blu', local: false, crit: false },
  { killer: 'You', kTeam: 'blu', victim: 'Scout', vTeam: 'red', local: true, crit: true },
  { killer: 'Sniper', kTeam: 'red', victim: 'Medic', vTeam: 'blu', local: false, crit: false },
  { killer: 'Heavy', kTeam: 'blu', victim: 'Demoman', vTeam: 'red', local: false, crit: false },
  { killer: 'Engineer', kTeam: 'red', victim: 'Soldier', vTeam: 'blu', local: false, crit: false },
];

/** Generic kill icon (TF2 draws weapon-specific icons from its own textures). */
function KillIcon({ w, h, color }: { w: number; h: number; color: string }) {
  return (
    <svg width={w} height={h} viewBox="0 0 40 16" style={{ flexShrink: 0 }} aria-hidden>
      <path d="M2 6h22l3-3h6v3h5v4h-5v3h-6l-3-3H12l-2 4H5l1-4H2z" fill={color} />
    </svg>
  );
}

/** HudDeathNotice is drawn by game code from its hudlayout settings; this mirrors that layout with sample kills. */
function DeathNotices({ el, ctx }: { el: El; ctx: Ctx }) {
  const v = (k: string) => ctx.value(el, k);
  const k = ctx.k;
  const lh = num(v('lineheight'), 16), sp = num(v('linespacing'), 4), r = num(v('cornerradius'), 3), max = Math.max(0, Math.round(num(v('maxdeathnotices'), 4)));
  const right = v('rightjustify') !== '0';
  const f = fontStyle(ctx.scheme.font(v('textfont'), ctx.screenPx));
  const team = (t: string) => ctx.scheme.css(t === 'red' ? v('teamred') : v('teamblue')) ?? (t === 'red' ? 'rgb(189,59,61)' : 'rgb(115,150,170)');
  const icon = ctx.scheme.css(v('iconcolor')) ?? '#fff';
  const local = ctx.scheme.css(v('localplayercolor')) ?? '#000';
  const bg = ctx.scheme.css(v('basebackgroundcolor')) ?? 'rgba(46,43,42,.86)';
  const lbg = ctx.scheme.css(v('localbackgroundcolor')) ?? 'rgba(245,229,196,.78)';
  return (
    <div style={{ position: 'absolute', inset: 0, display: 'flex', flexDirection: 'column', alignItems: right ? 'flex-end' : 'flex-start', gap: sp * k }}>
      {SAMPLE_KILLS.slice(0, max).map((row, i) => (
        <div key={i} style={{ height: lh * k, display: 'flex', alignItems: 'center', gap: 3 * k, padding: `0 ${4 * k}px`, borderRadius: r * k, background: row.local ? lbg : bg, fontFamily: f.family, fontSize: f.sizeUnits * k, fontWeight: f.weight, whiteSpace: 'nowrap', textShadow: textShadow(f.outline, f.shadow, k) }}>
          <span style={{ color: row.local ? local : team(row.kTeam) }}>{row.killer}</span>
          <KillIcon w={lh * 1.9 * k} h={lh * 0.75 * k} color={row.crit ? team(row.kTeam) : row.local ? local : icon} />
          <span style={{ color: team(row.vTeam) }}>{row.victim}</span>
        </div>
      ))}
    </div>
  );
}

const SAMPLE_CLASSES = ['scout', 'soldier', 'pyro', 'demo', 'heavy', 'engi', 'medic', 'sniper', 'spy', 'soldier', 'scout', 'medic'];

/** CTFTeamStatus builds one player panel per player from its playerpanels_kv template. */
function TeamStatusView({ el, ctx, box, parentBox }: { el: El; ctx: Ctx; box: Rect; parentBox: Rect }) {
  const v = (k: string) => ctx.value(el, k);
  // the template has layout keys, so the model may hold it as a child element or as a nested block
  const tplBlock = el.sub.find((b) => b.key.toLowerCase() === 'playerpanels_kv');
  const tplChild = el.children.find((c) => c.name.toLowerCase() === 'playerpanels_kv');
  const k = ctx.k;
  const n = Math.max(1, Math.round(num(v('max_size'), 12)));
  const gap = num(v('12v12_gap') ?? v('6v6_gap'), 1);
  const templ = useMemo(() => tplChild ?? (tplBlock ? toEl(tplBlock, el.home, el.uid + '/panel') : null), [tplChild, tplBlock, el]);
  if (!templ) return null;
  const tv = (key: string) => templ.props.get(key)?.value;
  const teams = [
    { side: 'team1', team: 'blue', dir: (v('team1_grow_dir') ?? 'west').toLowerCase() },
    { side: 'team2', team: 'red', dir: (v('team2_grow_dir') ?? 'east').toLowerCase() },
  ];
  const panels: React.ReactNode[] = [];
  for (const t of teams) {
    const expand = num(v(`${t.side}_max_expand`), 125);
    const w = (expand - (n - 1) * gap) / n;
    const base = parentBox.x + computePos(v(`${t.side}_base_x`) ?? 'c0', 0, parentBox.w);
    const panelBox = (i: number): Rect => ({ x: t.dir === 'west' ? base - (i + 1) * w - i * gap : base + i * (w + gap), y: box.y, w, h: box.h });
    for (let i = 0; i < n; i++) {
      const pb = panelBox(i);
      const dead = i === 3 || i === 8;
      const cls = SAMPLE_CLASSES[i % SAMPLE_CLASSES.length];
      const team = t.team === 'blue' ? 'blue' : 'red';
      const pv = (cel: El, key: string) => {
        const raw = ctx.value(cel, key);
        if (key === 'image' && raw) return raw.replace(/class_[a-z]+?(red|blue)/i, `class_${cls}${team}`);
        if (key === 'labeltext' && raw?.includes('%respawntime%')) return dead ? String(4 + i) : '';
        if (key === 'progress') return dead ? '0' : String(1 - ((i * 37) % 70) / 100);
        return raw;
      };
      const boxes = new Map<string, LayoutBox>();
      layoutTree(layoutInputs(templ.children, pv), pb, boxes);
      const bg = tv(`color_portrait_bg_${team}${dead ? '_dead' : ''}`);
      panels.push(
        <div key={t.side + i} className="vg-el" style={{ left: (pb.x - box.x) * k, top: 0, width: pb.w * k, height: pb.h * k, background: ctx.scheme.css(bg) }}>
          {templ.children.map((c) => <ElementView key={c.uid} el={c} ctx={{ ...ctx, boxes, value: pv, onDown: () => undefined }} parentBox={pb} />)}
        </div>,
      );
    }
  }
  return <>{panels}</>;
}

const SAMPLE_CHAT: [string, string, string][] = [
  ['Pyrotechnic', 'red', 'gg'],
  ['Medic', 'blu', 'uber ready, pop on me'],
  ['*DEAD* Scout', 'red', 'nice shot sniper'],
  ['Engineer', 'blu', 'teleporter up at spawn'],
];

function ChatSample({ font, ctx }: { font: string | undefined; ctx: Ctx }) {
  const f = fontStyle(ctx.scheme.font(font, ctx.screenPx));
  const k = ctx.k;
  return (
    <div style={{ position: 'absolute', inset: 0, display: 'flex', flexDirection: 'column', justifyContent: 'flex-end', fontFamily: f.family, fontSize: f.sizeUnits * k, fontWeight: f.weight, lineHeight: 1.25, color: 'rgb(251,235,202)', textShadow: textShadow(f.outline, true, k), overflow: 'hidden' }}>
      {SAMPLE_CHAT.map(([name, t, text], i) => (
        <div key={i} style={{ whiteSpace: 'pre-wrap' }}><span style={{ color: t === 'red' ? 'rgb(255,63,63)' : 'rgb(153,204,255)' }}>{name}</span> :  {text}</div>
      ))}
    </div>
  );
}

function textShadow(outline: boolean, shadow: boolean, k: number) {
  const s: string[] = [];
  const o = Math.max(0.6, k * 0.5);
  if (outline) for (const [x, y] of [[-o, 0], [o, 0], [0, -o], [0, o], [-o, -o], [o, o], [-o, o], [o, -o]]) s.push(`${x}px ${y}px 0 #000`);
  if (shadow) s.push(`${Math.max(1, k * 0.7)}px ${Math.max(1, k * 0.7)}px 0 rgba(0,0,0,.9)`);
  return s.join(',') || undefined;
}

function ElementView({ el, ctx, parentBox }: { el: El; ctx: Ctx; parentBox: Rect }) {
  const box = ctx.boxes.get(el.uid);
  if (!box) return null;
  const v = (k: string) => ctx.value(el, k);
  const userHidden = ctx.hidden.has(el.uid);
  const engineHidden = v('visible') === '0';
  if (userHidden || (engineHidden && !ctx.showHidden)) return null;
  const k = ctx.k;
  const kind = controlKind(el.control);
  const alpha = v('alpha');
  const style: React.CSSProperties = {
    left: (box.x - parentBox.x) * k,
    top: (box.y - parentBox.y) * k,
    width: box.w * k,
    height: box.h * k,
    zIndex: box.z + 1000,
    opacity: (alpha !== undefined ? Math.max(0, Math.min(255, parseFloat(alpha) || 0)) / 255 : 1) * (engineHidden ? 0.35 : 1),
    outline: engineHidden ? '1px dashed rgba(240,164,58,.6)' : undefined,
  };
  const bgRaw = v('bgcolor_override') ?? (kind === 'panel' ? v('bgcolor') : undefined);
  if (bgRaw && v('paintbackground') !== '0') style.background = ctx.scheme.css(bgRaw);
  if (v('paintbackgroundtype') === '2') style.borderRadius = 6 * k;
  const borderName = v('border') ?? (kind === 'button' ? v('border_default') : undefined);
  const imgBorder = v('paintborder') !== '0' ? imageBorder(ctx.scheme, borderName) : null;
  const border = imgBorder ? null : borderSides(ctx.scheme, borderName);
  if (border && v('paintborder') !== '0') {
    const w = Math.max(1, k * 0.6);
    if (border.left) style.borderLeft = `${w}px solid ${border.left}`;
    if (border.right) style.borderRight = `${w}px solid ${border.right}`;
    if (border.top) style.borderTop = `${w}px solid ${border.top}`;
    if (border.bottom) style.borderBottom = `${w}px solid ${border.bottom}`;
    if (border.rounded) style.borderRadius = 6 * k;
  }
  // Empty full-screen containers would swallow every click; they stay selectable from the Elements list.
  const covers = box.w * box.h > parentBox.w * parentBox.h * 0.5;
  const interactive = paints(el, ctx) || (el.children.length === 0 && !covers);
  let content: React.ReactNode = null;

  const lname = el.name.toLowerCase();
  if (el.control === 'ctfteamstatus') {
    style.overflow = 'visible';
    content = <TeamStatusView el={el} ctx={ctx} box={box} parentBox={parentBox} />;
  } else if (lname === 'huddeathnotice') {
    content = <DeathNotices el={el} ctx={ctx} />;
  } else if (lname === 'hudchathistory' && !(v('labeltext') ?? '').trim()) {
    content = <ChatSample font={v('font')} ctx={ctx} />;
  } else if (kind === 'image') {
    const fill = v('fillcolor');
    if (fill) style.background = ctx.scheme.css(fill);
    const image = v('image') || v('teambg_2') || v('image_default') || v('teambg_1');
    const tint = ctx.scheme.color(v('drawcolor_override') ?? v('drawcolor')) ?? null;
    const scalable = el.control.includes('scalable') || el.control.includes('ctfimagepanel');
    const src: [number, number] = [num(v('src_corner_width'), 0), num(v('src_corner_height'), 0)];
    if (image && scalable && (src[0] > 0 || src[1] > 0)) {
      content = <NineSlice name={image} src={src} draw={[num(v('draw_corner_width'), 0), num(v('draw_corner_height'), 0)]} tint={tint} k={k} />;
    } else if (image) content = <ImageLayer name={image} tint={tint} scale={truthy(v('scaleimage')) || scalable} tile={truthy(v('tileimage'))} k={k} w={box.w} h={box.h} />;
  } else if (kind === 'label' || kind === 'button' || kind === 'richtext') {
    const text = sampleText(v('labeltext') ?? v('text'), ctx.localized);
    const f = fontStyle(ctx.scheme.font(v('font'), ctx.screenPx));
    const fg = ctx.scheme.css(kind === 'button' ? v('defaultfgcolor_override') ?? v('fgcolor_override') ?? v('fgcolor') : v('fgcolor_override') ?? v('fgcolor')) ?? 'rgba(235,226,202,1)';
    if (kind === 'button') {
      const bb = v('defaultbgcolor_override');
      if (bb && v('paintbackground') !== '0') style.background = ctx.scheme.css(bb);
    }
    const a = alignment(v('textalignment'));
    const wrap = truthy(v('wrap')) || truthy(v('centerwrap')) || kind === 'richtext';
    const inset = (parseFloat(v('textinsetx') ?? '0') || 0) * k;
    content = (
      <div
        className="vg-text"
        style={{
          justifyContent: a.justify, alignItems: kind === 'richtext' ? 'flex-start' : a.items, textAlign: truthy(v('centerwrap')) ? 'center' : a.text,
          padding: `${(parseFloat(v('textinsety') ?? '0') || 0) * k}px ${inset}px`,
          fontFamily: f.family, fontSize: f.sizeUnits * k, fontWeight: f.weight, fontStyle: f.italic ? 'italic' : undefined, color: fg,
          whiteSpace: wrap ? 'pre-wrap' : 'pre', textShadow: textShadow(f.outline, f.shadow, k), textTransform: truthy(v('allcaps')) ? 'uppercase' : undefined,
          filter: f.blur ? `blur(${f.blur * k * 0.5}px)` : undefined, mixBlendMode: f.additive ? 'screen' : undefined,
        }}
      >
        {text}
      </div>
    );
  } else if (kind === 'progress') {
    const fg = ctx.scheme.css(v('fgcolor_override') ?? v('fgcolor')) ?? 'rgba(255,255,255,.9)';
    if (!style.background) style.background = ctx.scheme.css(v('bgcolor_override')) ?? 'rgba(0,0,0,.35)';
    const vertical = box.h > box.w * 1.5;
    const p = Math.max(0, Math.min(1, parseFloat(v('progress') ?? '0.65') || 0.65));
    content = <div style={{ position: 'absolute', left: 0, bottom: 0, background: fg, width: vertical ? '100%' : `${p * 100}%`, height: vertical ? `${p * 100}%` : '100%' }} />;
  } else if (kind === 'model' || kind === 'avatar') {
    content = <div className="vg-placeholder">{kind === 'avatar' ? 'avatar' : 'model'}</div>;
  }

  const children = [...el.children].sort((a, b) => (ctx.boxes.get(a.uid)?.z ?? 0) - (ctx.boxes.get(b.uid)?.z ?? 0));
  return (
    <div
      className={'vg-el' + (interactive ? '' : ' passthrough')}
      style={style}
      data-uid={el.uid}
      onPointerDown={interactive ? (e) => ctx.onDown(e, el) : undefined}
    >
      {imgBorder && <NineSlice name={imgBorder.image} src={imgBorder.src} draw={imgBorder.draw} tint={null} k={k} />}
      {content}
      {children.map((c) => <ElementView key={c.uid} el={c} ctx={ctx} parentBox={box} />)}
    </div>
  );
}

// ── interaction ────────────────────────────────────────────────────────────

type Drag =
  | { kind: 'move'; startX: number; startY: number; boxes: Map<string, LayoutBox>; els: El[]; moved: boolean }
  | { kind: 'resize'; handle: string; startX: number; startY: number; box: LayoutBox; el: El; moved: boolean }
  | { kind: 'marquee'; startX: number; startY: number; x: number; y: number; additive: boolean };

const HANDLES = ['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w'];

export function Canvas() {
  const model = useStore((s) => s.model);
  const scheme = useStore((s) => s.schemeIndex);
  const resolution = useStore((s) => s.resolution);
  const zoom = useStore((s) => s.zoom);
  const selection = useStore((s) => s.selection);
  const showHidden = useStore((s) => s.showHidden);
  const showGrid = useStore((s) => s.showGrid);
  const hidden = useStore((s) => s.hidden);
  const localized = useStore((s) => s.localized);
  const snap = useStore((s) => s.snap);
  const playback = useStore((s) => s.playback);
  const animations = useStore((s) => s.animations);
  useStore((s) => s.fontsVersion);
  const backdrop = useStore((s) => (s.app?.settings.ui?.backdrop as string | undefined) ?? 'dusk');

  const wrapRef = useRef<HTMLDivElement>(null);
  const [avail, setAvail] = useState({ w: 900, h: 500 });
  const [drag, setDrag] = useState<Drag | null>(null);
  const [overrides, setOverrides] = useState<Overrides>(new Map());
  const pendingOverrides = useRef<Overrides>(new Map());
  const updateOverrides = (next: Overrides) => { pendingOverrides.current = next; setOverrides(next); };
  const [guides, setGuides] = useState<{ x: number[]; y: number[] }>({ x: [], y: [] });
  const [cursor, setCursor] = useState<{ x: number; y: number } | null>(null);

  useLayoutEffect(() => {
    const el = wrapRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setAvail({ w: el.clientWidth, h: el.clientHeight }));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const screen = screenUnits(resolution);
  const k = zoom === 'fit' ? Math.max(0.2, Math.min((avail.w - 48) / screen.w, (avail.h - 48) / screen.h)) : (zoom * resolution.h) / 480;

  const animCmds = useMemo(() => (playback && animations ? expand(animations.events, playback.event) : null), [playback?.event, animations]);
  const animValues = useMemo(() => {
    if (!playback || !animCmds) return null;
    const base = new Map<string, El>();
    walk(model, (el) => {
      base.set((el.props.get('fieldname')?.value ?? el.name).toLowerCase(), el);
      if (!base.has(el.name.toLowerCase())) base.set(el.name.toLowerCase(), el);
    });
    return evaluate(animCmds, playback.t, (panel, prop) => base.get(panel.toLowerCase())?.props.get(prop)?.value, (raw) => scheme.color(raw));
  }, [animCmds, playback?.t, model, scheme]);

  const value = useMemo(() => makeValue(overrides, animValues), [overrides, animValues]);
  const boxes = useMemo(() => {
    const out = new Map<string, LayoutBox>();
    layoutTree(layoutInputs(model, value), screen, out);
    return out;
  }, [model, value, screen.w]);

  const sel = useMemo(() => new Set(selection), [selection]);

  const toUnits = (e: { clientX: number; clientY: number }) => {
    const r = surfaceRef.current!.getBoundingClientRect();
    return clientToUnits(e, r, screen);
  };
  const surfaceRef = useRef<HTMLDivElement>(null);

  const onDown = (e: React.PointerEvent, el: El) => {
    if (e.button !== 0) return;
    e.stopPropagation();
    surfaceRef.current?.setPointerCapture(e.pointerId);
    updateOverrides(new Map());
    const additive = e.shiftKey || e.ctrlKey || e.metaKey;
    let nextSel = selection;
    if (additive) nextSel = sel.has(el.uid) ? selection.filter((u) => u !== el.uid) : [...selection, el.uid];
    else if (!sel.has(el.uid)) nextSel = [el.uid];
    set({ selection: nextSel });
    const p = toUnits(e);
    const els = nextSel.map((u) => findEl(model, u)).filter((x): x is El => !!x);
    // don't move an element together with one of its own ancestors
    const moving = els.filter((x) => !els.some((o) => o !== x && x.uid.startsWith(o.uid + '/')));
    setDrag({ kind: 'move', startX: p.x, startY: p.y, boxes: new Map(moving.map((x) => [x.uid, boxes.get(x.uid)!])), els: moving, moved: false });
  };

  const onHandleDown = (e: React.PointerEvent, handle: string) => {
    if (e.button !== 0) return;
    e.stopPropagation();
    const el = findEl(model, selection[0]);
    const box = el && boxes.get(el.uid);
    if (!el || !box) return;
    surfaceRef.current?.setPointerCapture(e.pointerId);
    updateOverrides(new Map());
    const p = toUnits(e);
    setDrag({ kind: 'resize', handle, startX: p.x, startY: p.y, box, el, moved: false });
  };

  const onBackgroundDown = (e: React.PointerEvent) => {
    if (e.button !== 0) return;
    surfaceRef.current?.setPointerCapture(e.pointerId);
    const p = toUnits(e);
    if (!(e.shiftKey || e.ctrlKey)) set({ selection: [] });
    setDrag({ kind: 'marquee', startX: p.x, startY: p.y, x: p.x, y: p.y, additive: e.shiftKey || e.ctrlKey });
  };

  /** Snap candidates: sibling and parent edges/centers. */
  const snapTargets = (exclude: Set<string>, parent: Rect) => {
    const xs = [parent.x, parent.x + parent.w / 2, parent.x + parent.w];
    const ys = [parent.y, parent.y + parent.h / 2, parent.y + parent.h];
    for (const b of boxes.values()) {
      if (exclude.has(b.uid) || b.parent !== parent) continue;
      xs.push(b.x, b.x + b.w / 2, b.x + b.w);
      ys.push(b.y, b.y + b.h / 2, b.y + b.h);
    }
    return { xs, ys };
  };

  useEffect(() => {
    if (!drag) return;
    const threshold = 5 / k;
    const move = (e: PointerEvent) => {
      const p = toUnits(e);
      const dx = p.x - drag.startX, dy = p.y - drag.startY;
      if (drag.kind === 'marquee') {
        setDrag({ ...drag, x: p.x, y: p.y });
        return;
      }
      if (!drag.moved && Math.abs(dx) * k < 3 && Math.abs(dy) * k < 3) return;
      const next = new Map<string, Record<string, string>>();
      const gx: number[] = [], gy: number[] = [];
      if (drag.kind === 'move') {
        const first = [...drag.boxes.values()][0];
        let sx = dx, sy = dy;
        if (first && !e.altKey) {
          const { xs, ys } = snapTargets(new Set(drag.boxes.keys()), first.parent);
          const cand = (vals: number[], edges: number[]) => {
            let best: { d: number; at: number } | null = null;
            for (const edge of edges) for (const t of vals) { const d = t - edge; if (Math.abs(d) < threshold && (!best || Math.abs(d) < Math.abs(best.d))) best = { d, at: t }; }
            return best;
          };
          const bx = cand(xs, [first.x + dx, first.x + dx + first.w / 2, first.x + dx + first.w]);
          const by = cand(ys, [first.y + dy, first.y + dy + first.h / 2, first.y + dy + first.h]);
          if (bx) { sx = dx + bx.d; gx.push(bx.at); } else if (snap > 1) sx = Math.round((first.x + dx) / snap) * snap - first.x;
          if (by) { sy = dy + by.d; gy.push(by.at); } else if (snap > 1) sy = Math.round((first.y + dy) / snap) * snap - first.y;
        }
        for (const el of drag.els) {
          const b = drag.boxes.get(el.uid)!;
          const enc = encodeMove(b, el.props.get('xpos')?.value, el.props.get('ypos')?.value, Math.round(b.x + sx), Math.round(b.y + sy));
          next.set(el.uid, enc);
        }
      } else {
        const b = drag.box, h = drag.handle;
        let x = b.x, y = b.y, w = b.w, hh = b.h;
        if (h.includes('e')) w = Math.max(1, b.w + dx);
        if (h.includes('s')) hh = Math.max(1, b.h + dy);
        if (h.includes('w')) { x = Math.min(b.x + b.w - 1, b.x + dx); w = b.w - (x - b.x); }
        if (h.includes('n')) { y = Math.min(b.y + b.h - 1, b.y + dy); hh = b.h - (y - b.y); }
        if (e.shiftKey) { const r = b.w / b.h; if (h === 'n' || h === 's') w = hh * r; else hh = w / r; }
        [x, y, w, hh] = [Math.round(x), Math.round(y), Math.round(w), Math.round(hh)];
        const props: Record<string, string> = {
          wide: encodeSize(drag.el.props.get('wide')?.value, w, b.parent.w, hh),
          tall: encodeSize(drag.el.props.get('tall')?.value, hh, b.parent.h, w),
        };
        const nb = { ...b, w, h: hh };
        const pos = encodeMove(nb, drag.el.props.get('xpos')?.value, drag.el.props.get('ypos')?.value, x, y);
        // keep anchoring notation (r/c/cs) correct for the new size even when only the far edge moved
        props.xpos = pos.xpos;
        props.ypos = pos.ypos;
        next.set(drag.el.uid, props);
      }
      setGuides({ x: gx, y: gy });
      updateOverrides(next);
      if (!drag.moved) setDrag({ ...drag, moved: true } as Drag);
    };
    const up = async (e: PointerEvent) => {
      // Commit the release position even if the last pointermove hasn't caused a React render yet.
      if (drag.kind !== 'marquee') move(e);
      setGuides({ x: [], y: [] });
      if (drag.kind === 'marquee') {
        const end = toUnits(e);
        const r = { x: Math.min(drag.startX, end.x), y: Math.min(drag.startY, end.y), w: Math.abs(end.x - drag.startX), h: Math.abs(end.y - drag.startY) };
        if (r.w > 2 || r.h > 2) {
          const hits: string[] = [];
          walk(model, (el) => {
            const b = boxes.get(el.uid);
            if (b && !hidden.has(el.uid) && b.x >= r.x && b.y >= r.y && b.x + b.w <= r.x + r.w && b.y + b.h <= r.y + r.h && b.w < screen.w * 0.95) hits.push(el.uid);
          });
          set((s) => ({ selection: drag.additive ? [...new Set([...s.selection, ...hits])] : hits }));
        }
        setDrag(null);
        return;
      }
      const pending = pendingOverrides.current;
      setDrag(null);
      if (pending.size === 0) return;
      const edits: { el: El; key: string; value: string }[] = [];
      for (const [uid, props] of pending) {
        const el = findEl(model, uid);
        if (!el) continue;
        for (const [key, val] of Object.entries(props)) if (el.props.get(key)?.value !== val) edits.push({ el, key, value: val });
      }
      if (edits.length) await editElements(edits);
      updateOverrides(new Map());
    };
    const cancel = () => { setDrag(null); setGuides({ x: [], y: [] }); updateOverrides(new Map()); };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up, { once: true });
    window.addEventListener('pointercancel', cancel, { once: true });
    return () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      window.removeEventListener('pointercancel', cancel);
    };
  }, [drag, overrides, k, boxes, model, snap]);

  // keyboard nudging: key repeats accumulate and are written in one batch, so no step is lost while a write is in flight
  const nudge = useRef({ dx: 0, dy: 0, timer: 0 as ReturnType<typeof setTimeout> | 0, busy: false });
  const latest = useRef({ model, boxes });
  latest.current = { model, boxes };
  useEffect(() => {
    const flush = async () => {
      const n = nudge.current;
      if (n.busy || (!n.dx && !n.dy)) return;
      const { dx, dy } = n;
      n.dx = 0;
      n.dy = 0;
      n.busy = true;
      try {
        const { model: m, boxes: bx } = latest.current;
        const els = getState().selection.map((u) => findEl(m, u)).filter((x): x is El => !!x);
        const edits: { el: El; key: string; value: string }[] = [];
        for (const el of els) {
          const b = bx.get(el.uid);
          if (!b) continue;
          const enc = encodeMove(b, el.props.get('xpos')?.value, el.props.get('ypos')?.value, b.x + dx, b.y + dy);
          if (dx) edits.push({ el, key: 'xpos', value: enc.xpos });
          if (dy) edits.push({ el, key: 'ypos', value: enc.ypos });
        }
        if (edits.length) await editElements(edits);
      } finally {
        n.busy = false;
        // wait for React to lay out the new model before applying anything typed meanwhile
        if (n.dx || n.dy) n.timer = setTimeout(() => void flush(), 30);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement;
      if (t.closest('input,textarea,select,.select,.cm-editor') || getState().view !== 'design') return;
      const dirs: Record<string, [number, number]> = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };
      const d = dirs[e.key];
      if (!d || !getState().selection.length) return;
      e.preventDefault();
      const step = e.shiftKey ? 10 : 1;
      const n = nudge.current;
      n.dx += d[0] * step;
      n.dy += d[1] * step;
      if (n.timer) clearTimeout(n.timer);
      n.timer = setTimeout(() => void flush(), 60);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  const ctx: Ctx = { k, value, boxes, scheme, screenPx: resolution.h, showHidden, hidden, localized, selection: sel, onDown };
  const top = [...model].sort((a, b) => (boxes.get(a.uid)?.z ?? 0) - (boxes.get(b.uid)?.z ?? 0));
  const selBoxes = selection.map((u) => boxes.get(u)).filter((b): b is LayoutBox => !!b);
  const single = selBoxes.length === 1 ? selBoxes[0] : null;
  const singleEl = single ? findEl(model, selection[0]) : null;

  return (
    <div className="canvas-wrap" ref={wrapRef} onPointerDown={onBackgroundDown}>
      <div
        className={`surface backdrop-${backdrop}${showGrid ? ' grid' : ''}`}
        ref={surfaceRef}
        style={{ width: screen.w * k, height: screen.h * k, ['--grid' as string]: `${Math.max(4, (snap > 1 ? snap : 10) * k)}px` }}
        onPointerMove={(e) => setCursor(toUnits(e))}
        onPointerLeave={() => setCursor(null)}
      >
        {top.map((el) => <ElementView key={el.uid} el={el} ctx={ctx} parentBox={screen} />)}
        <div className="overlay">
          {selBoxes.map((b) => (
            <div key={b.uid} className="sel-box" style={{ left: b.x * k, top: b.y * k, width: b.w * k, height: b.h * k }}>
              {b.pinnedTo && <div className="pin-line" />}
            </div>
          ))}
          {single?.pinnedTo && (() => {
            const p = single.pinnedTo.rect;
            return <div className="pin-target" style={{ left: p.x * k, top: p.y * k, width: p.w * k, height: p.h * k }} />;
          })()}
          {single && singleEl && (
            <div className="handles" style={{ left: single.x * k, top: single.y * k, width: single.w * k, height: single.h * k }}>
              {HANDLES.map((h) => <div key={h} className={`handle h-${h}`} onPointerDown={(e) => onHandleDown(e, h)} />)}
              <div className="size-tag">{Math.round(single.w)} × {Math.round(single.h)}</div>
            </div>
          )}
          {guides.x.map((x, i) => <div key={'gx' + i} className="guide gx" style={{ left: x * k }} />)}
          {guides.y.map((y, i) => <div key={'gy' + i} className="guide gy" style={{ top: y * k }} />)}
          {drag?.kind === 'marquee' && (
            <div className="marquee" style={{ left: Math.min(drag.startX, drag.x) * k, top: Math.min(drag.startY, drag.y) * k, width: Math.abs(drag.x - drag.startX) * k, height: Math.abs(drag.y - drag.startY) * k }} />
          )}
        </div>
      </div>
      <div className="canvas-hud">
        {cursor ? `${Math.round(cursor.x)}, ${Math.round(cursor.y)}  ·  c${Math.round(cursor.x - screen.w / 2)}  r${Math.round(screen.w - cursor.x)}` : `${Math.round(screen.w)} × 480 units`}
      </div>
    </div>
  );
}
