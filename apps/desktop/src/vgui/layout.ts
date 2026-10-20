// VGUI coordinate engine (vgui_controls Panel::ComputePos / ComputeWide / ComputeTall / pinning),
// working in proportional units: the screen is always 480 units tall and 480 × aspect wide.

export type Rect = { x: number; y: number; w: number; h: number };

/** C atoi: leading integer, ignoring trailing junk. */
export function atoi(s: string): number {
  const m = /^\s*[+-]?\d+/.exec(s);
  return m ? parseInt(m[0], 10) : 0;
}
/** C atof. */
export function atof(s: string): number {
  const m = /^\s*[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?/.exec(s);
  return m ? parseFloat(m[0]) : 0;
}

export type PosMode = 'abs' | 'right' | 'center';
export type PosScale = 'none' | 'self' | 'parent';
export type ParsedPos = { mode: PosMode; scale: PosScale; num: number };

export function parsePos(raw: string | undefined): ParsedPos {
  let s = (raw ?? '0').trim();
  let mode: PosMode = 'abs';
  let scale: PosScale = 'none';
  const c0 = s[0]?.toLowerCase();
  if (c0 === 'r') { mode = 'right'; s = s.slice(1); } else if (c0 === 'c') { mode = 'center'; s = s.slice(1); }
  const c1 = s[0]?.toLowerCase();
  if (c1 === 's') { scale = 'self'; s = s.slice(1); } else if (c1 === 'p') { scale = 'parent'; s = s.slice(1); }
  return { mode, scale, num: scale === 'none' ? atoi(s) : atof(s) };
}

export function computePos(raw: string | undefined, size: number, parentSize: number): number {
  const p = parsePos(raw);
  const delta = p.scale === 'self' ? size * p.num : p.scale === 'parent' ? parentSize * p.num : p.num;
  if (p.mode === 'right') return parentSize - delta;
  if (p.mode === 'center') return parentSize / 2 + delta;
  return delta;
}

/** Inverse of computePos: keeps the notation (mode + scale) of `original` and returns a new raw value. */
export function encodePos(original: string | undefined, pos: number, size: number, parentSize: number): string {
  const p = parsePos(original);
  let delta = p.mode === 'right' ? parentSize - pos : p.mode === 'center' ? pos - parentSize / 2 : pos;
  const prefix = (p.mode === 'right' ? 'r' : p.mode === 'center' ? 'c' : '') + (p.scale === 'self' ? 's' : p.scale === 'parent' ? 'p' : '');
  if (p.scale !== 'none') {
    const base = p.scale === 'self' ? size : parentSize;
    if (!base) return prefix + '0';
    delta = delta / base;
    return prefix + trimFloat(delta, 5);
  }
  return prefix + Math.round(delta);
}

export type SizeMode = 'abs' | 'full' | 'parent' | 'other';
export function parseSize(raw: string | undefined): { mode: SizeMode; num: number } {
  const s = (raw ?? '').trim();
  const c = s[0]?.toLowerCase();
  if (c === 'f') return { mode: 'full', num: atoi(s.slice(1)) };
  if (c === 'p') return { mode: 'parent', num: atof(s.slice(1)) };
  if (c === 'o') return { mode: 'other', num: atof(s.slice(1)) };
  return { mode: 'abs', num: atoi(s) };
}

export function computeSize(raw: string | undefined, parentSize: number, other: number, fallback: number): number {
  if (raw === undefined || raw.trim() === '') return fallback;
  const p = parseSize(raw);
  const v = p.mode === 'full' ? parentSize - p.num : p.mode === 'parent' ? parentSize * p.num : p.mode === 'other' ? other * p.num : p.num;
  return Math.max(0, v);
}

export function encodeSize(original: string | undefined, size: number, parentSize: number, other: number): string {
  const p = parseSize(original);
  if (p.mode === 'full') return 'f' + Math.round(parentSize - size);
  if (p.mode === 'parent') return 'p' + trimFloat(parentSize ? size / parentSize : 0, 4);
  if (p.mode === 'other') return 'o' + trimFloat(other ? size / other : 0, 4);
  return String(Math.max(0, Math.round(size)));
}

export function trimFloat(v: number, digits: number): string {
  const s = v.toFixed(digits).replace(/\.?0+$/, '');
  return s === '-0' ? '0' : s;
}

// ── pinning ────────────────────────────────────────────────────────────────

export const PIN_CORNERS = ['PIN_TOPLEFT', 'PIN_TOPRIGHT', 'PIN_BOTTOMLEFT', 'PIN_BOTTOMRIGHT', 'PIN_CENTER_TOP', 'PIN_CENTER_RIGHT', 'PIN_CENTER_BOTTOM', 'PIN_CENTER_LEFT'];

export function parseCorner(raw: string | undefined): number {
  if (!raw) return 0;
  const t = raw.trim().toUpperCase();
  const named = PIN_CORNERS.indexOf(t);
  if (named >= 0) return named;
  const n = atoi(t);
  return n >= 0 && n < 8 ? n : 0;
}

/** Point of a w×h box for a corner index, relative to its top-left. */
export function cornerPoint(corner: number, w: number, h: number): [number, number] {
  switch (corner) {
    case 1: return [w, 0];
    case 2: return [0, h];
    case 3: return [w, h];
    case 4: return [w / 2, 0];
    case 5: return [w, h / 2];
    case 6: return [w / 2, h];
    case 7: return [0, h / 2];
    default: return [0, 0];
  }
}

/**
 * Pinned offsets point outward from the sibling's corner: on a left-side corner positive xpos moves left, on a
 * right-side corner it moves right; on a top corner positive ypos moves up, on a bottom corner down.
 * (So the classic `-1 -1` pinned to a label's top-left is a drop shadow down-right, and `xpos 5` on a
 * top-right corner leaves a 5-unit gap to the right.)
 */
export function pinSigns(siblingCorner: number): [number, number] {
  const sx = siblingCorner === 0 || siblingCorner === 2 || siblingCorner === 7 ? -1 : 1;
  const sy = siblingCorner === 0 || siblingCorner === 1 || siblingCorner === 4 ? -1 : 1;
  return [sx, sy];
}

export function pinnedPos(sibling: Rect, siblingCorner: number, myCorner: number, w: number, h: number, dx: number, dy: number): [number, number] {
  const [ax, ay] = cornerPoint(siblingCorner, sibling.w, sibling.h);
  const [mx, my] = cornerPoint(myCorner, w, h);
  const [sx, sy] = pinSigns(siblingCorner);
  return [sibling.x + ax - mx + sx * dx, sibling.y + ay - my + sy * dy];
}

/** Inverse of pinnedPos for one axis: the delta that puts the box at `pos`. */
export function pinnedDelta(axis: 'x' | 'y', pos: number, sibling: Rect, siblingCorner: number, myCorner: number, w: number, h: number): number {
  const [ax, ay] = cornerPoint(siblingCorner, sibling.w, sibling.h);
  const [mx, my] = cornerPoint(myCorner, w, h);
  const [sx, sy] = pinSigns(siblingCorner);
  return axis === 'x' ? (pos - (sibling.x + ax - mx)) * sx : (pos - (sibling.y + ay - my)) * sy;
}

// ── element trees ──────────────────────────────────────────────────────────

export type LayoutInput = {
  uid: string;
  /** Names a sibling may pin to (block key and fieldName, lowercased). */
  names: string[];
  get(key: string): string | undefined;
  children: LayoutInput[];
};

export type LayoutBox = Rect & {
  uid: string;
  parent: Rect;
  pinnedTo?: { uid: string; rect: Rect; siblingCorner: number; myCorner: number };
  z: number;
  order: number;
};

/** Lays out siblings inside `parent` (absolute coordinates), then recurses into children. */
export function layoutTree(nodes: LayoutInput[], parent: Rect, out: Map<string, LayoutBox>, overrides?: (uid: string, key: string) => string | undefined): void {
  const get = (n: LayoutInput, k: string) => overrides?.(n.uid, k) ?? n.get(k);
  const sizes = new Map<string, [number, number]>();
  for (const n of nodes) {
    const wideRaw = get(n, 'wide');
    const tallRaw = get(n, 'tall');
    let w: number, h: number;
    if (parseSize(wideRaw).mode === 'other' && wideRaw) {
      h = computeSize(tallRaw, parent.h, 0, 0);
      w = computeSize(wideRaw, parent.w, h, 0);
    } else {
      w = computeSize(wideRaw, parent.w, 0, 0);
      h = computeSize(tallRaw, parent.h, w, 0);
    }
    sizes.set(n.uid, [w, h]);
  }
  const byName = new Map<string, LayoutInput>();
  for (const n of nodes) for (const name of n.names) if (!byName.has(name)) byName.set(name, n);
  const placing = new Set<string>();
  const place = (n: LayoutInput): LayoutBox => {
    const done = out.get(n.uid);
    if (done) return done;
    const [w, h] = sizes.get(n.uid)!;
    let x: number, y: number;
    let pinnedTo: LayoutBox['pinnedTo'];
    const pin = get(n, 'pin_to_sibling')?.toLowerCase();
    const target = pin ? byName.get(pin) : undefined;
    if (target && target !== n && !placing.has(n.uid)) {
      placing.add(n.uid);
      const s = place(target);
      placing.delete(n.uid);
      const myCorner = parseCorner(get(n, 'pin_corner_to_sibling'));
      const siblingCorner = parseCorner(get(n, 'pin_to_sibling_corner'));
      [x, y] = pinnedPos(s, siblingCorner, myCorner, w, h, atoi(get(n, 'xpos') ?? '0'), atoi(get(n, 'ypos') ?? '0'));
      pinnedTo = { uid: target.uid, rect: s, siblingCorner, myCorner };
    } else {
      x = parent.x + computePos(get(n, 'xpos'), w, parent.w);
      y = parent.y + computePos(get(n, 'ypos'), h, parent.h);
    }
    const box: LayoutBox = { uid: n.uid, x, y, w, h, parent, pinnedTo, z: atoi(get(n, 'zpos') ?? '0'), order: out.size };
    out.set(n.uid, box);
    return box;
  };
  for (const n of nodes) place(n);
  for (const n of nodes) if (n.children.length) layoutTree(n.children, out.get(n.uid)!, out, overrides);
}

/** Raw xpos/ypos value that moves a laid-out box to absolute (x, y), preserving each value's notation. */
export function encodeMove(box: LayoutBox, rawX: string | undefined, rawY: string | undefined, x: number, y: number): { xpos: string; ypos: string } {
  if (box.pinnedTo) {
    const p = box.pinnedTo;
    return {
      xpos: String(Math.round(pinnedDelta('x', x, p.rect, p.siblingCorner, p.myCorner, box.w, box.h))),
      ypos: String(Math.round(pinnedDelta('y', y, p.rect, p.siblingCorner, p.myCorner, box.w, box.h))),
    };
  }
  return {
    xpos: encodePos(rawX, x - box.parent.x, box.w, box.parent.w),
    ypos: encodePos(rawY, y - box.parent.y, box.h, box.parent.h),
  };
}

export const ASPECTS: { label: string; w: number; h: number }[] = [
  { label: '1920 × 1080 (16:9)', w: 1920, h: 1080 },
  { label: '2560 × 1440 (16:9)', w: 2560, h: 1440 },
  { label: '1280 × 720 (16:9)', w: 1280, h: 720 },
  { label: '1920 × 1200 (16:10)', w: 1920, h: 1200 },
  { label: '1680 × 1050 (16:10)', w: 1680, h: 1050 },
  { label: '1280 × 1024 (5:4)', w: 1280, h: 1024 },
  { label: '1024 × 768 (4:3)', w: 1024, h: 768 },
  { label: '2560 × 1080 (21:9)', w: 2560, h: 1080 },
  { label: '3440 × 1440 (21:9)', w: 3440, h: 1440 },
  { label: '1280 × 800 (Steam Deck)', w: 1280, h: 800 },
];

/** Screen size in proportional units for a pixel resolution. */
export function screenUnits(px: { w: number; h: number }): Rect {
  return { x: 0, y: 0, w: (480 * px.w) / px.h, h: 480 };
}
