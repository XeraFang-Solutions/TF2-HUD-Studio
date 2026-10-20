// AnimationController playback for the preview: expands RunEvent chains into a timeline and evaluates
// Animate/SetVisible commands at any time t.
import type { AnimCommand, AnimEvent } from '../api.ts';
import { parseRGBA, type RGBA } from './scheme.ts';

export type Scheduled = AnimCommand & { at: number; from: string; depth: number };

const HORIZON = 12; // seconds of looping events to expand

export function expand(events: AnimEvent[], name: string, horizon = HORIZON): Scheduled[] {
  const byName = new Map<string, AnimEvent>();
  for (const e of events) if (!e.shadowedBy && !byName.has(e.name.toLowerCase())) byName.set(e.name.toLowerCase(), e);
  const out: Scheduled[] = [];
  const stops: { event: string; at: number }[] = [];
  let budget = 4000;
  const run = (ev: string, offset: number, depth: number) => {
    const e = byName.get(ev.toLowerCase());
    if (!e || offset > horizon || depth > 64) return;
    for (const c of e.commands) {
      if (--budget < 0) return;
      const at = offset + c.start;
      if (c.kind === 'RunEvent' && c.event) run(c.event, at, depth + 1);
      else if (c.kind === 'RunEventChild' && c.event) run(c.event, at, depth + 1);
      else if (c.kind === 'StopEvent' && c.event) stops.push({ event: c.event.toLowerCase(), at });
      out.push({ ...c, at, from: e.name, depth });
    }
  };
  run(name, 0, 0);
  // StopEvent cancels commands of that event (and the events it started) scheduled after the stop time
  const filtered = out.filter((c) => !stops.some((s) => s.event === c.from.toLowerCase() && c.at >= s.at && c.kind !== 'StopEvent'));
  return filtered.sort((a, b) => a.at - b.at);
}

export function timelineLength(cmds: Scheduled[]): number {
  return Math.min(HORIZON, Math.max(0.5, ...cmds.map((c) => c.at + (c.duration || 0))));
}

// ── interpolators (vgui AnimationController::GetInterpolatedValue) ─────────

function bias(x: number, b: number) {
  return Math.pow(x, Math.log(b) / Math.log(0.5));
}
function gain(x: number, g: number) {
  return x < 0.5 ? bias(2 * x, 1 - g) / 2 : 1 - bias(2 - 2 * x, 1 - g) / 2;
}
function hash(n: number) {
  const s = Math.sin(n * 12.9898) * 43758.5453;
  return s - Math.floor(s);
}

export function interpolate(kind: string | undefined, p: number, param = 0): number {
  const x = Math.max(0, Math.min(1, p));
  switch ((kind ?? 'linear').toLowerCase()) {
    case 'accel': return x * x;
    case 'deaccel': return Math.sqrt(x);
    case 'spline': return x * x * (3 - 2 * x);
    case 'pulse': return 0.5 - 0.5 * Math.cos(x * 2 * Math.PI * (param || 1));
    case 'flicker': return hash(Math.floor(x * 60)) < (param || 0.5) ? 1 : 0;
    case 'gain': return gain(x, param || 0.5);
    case 'bias': return bias(x, param || 0.5);
    case 'bounce': {
      const n1 = 7.5625, d1 = 2.75;
      if (x < 1 / d1) return n1 * x * x;
      if (x < 2 / d1) return n1 * (x - 1.5 / d1) ** 2 + 0.75;
      if (x < 2.5 / d1) return n1 * (x - 2.25 / d1) ** 2 + 0.9375;
      return n1 * (x - 2.625 / d1) ** 2 + 0.984375;
    }
    default: return x;
  }
}

export type AnimValue = { num?: number[]; color?: RGBA; raw?: string };

/** Animated property values per lowercased panel name at time t. A new Animate on the same panel property
 *  replaces the running one, starting from wherever that one had got to (as the engine does). */
export function evaluate(cmds: Scheduled[], t: number, base: (panel: string, prop: string) => string | undefined, color: (raw: string) => RGBA | null): Map<string, Map<string, string>> {
  const result = new Map<string, Map<string, string>>();
  const groups = new Map<string, Scheduled[]>();
  for (const c of cmds) {
    if (c.at > t || !c.panel) continue;
    const prop = c.kind === 'SetVisible' ? 'visible' : c.property?.toLowerCase();
    if (!prop || !['Animate', 'SetVisible', 'SetString', 'SetFont', 'SetTexture'].includes(c.kind)) continue;
    const key = c.panel.toLowerCase() + '\u0000' + prop;
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key)!.push(c);
  }
  for (const [key, list] of groups) {
    const [panel, prop] = key.split('\u0000');
    const valueAt = (time: number, upto: number): string => {
      for (let i = upto - 1; i >= 0; i--) {
        const c = list[i];
        if (c.at > time) continue;
        if (c.kind !== 'Animate') return c.kind === 'SetVisible' ? (c.value === '1' ? '1' : '0') : c.value ?? '';
        const from = valueAt(c.at, i);
        const p = c.duration > 0 ? (time - c.at) / c.duration : 1;
        return mix(from, c.value ?? '', interpolate(c.interpolator, p, c.param), prop, color);
      }
      return base(panel, prop) ?? defaultFor(prop);
    };
    if (!result.has(panel)) result.set(panel, new Map());
    result.get(panel)!.set(prop, valueAt(t, list.length));
  }
  return result;
}

function defaultFor(prop: string): string {
  if (prop === 'alpha') return '255';
  if (prop.endsWith('color')) return '255 255 255 255';
  return '0';
}

function nums(s: string): number[] {
  return s.trim().split(/\s+/).map((n) => parseFloat(n.replace(/^[crs]+/i, ''))).filter((n) => Number.isFinite(n));
}

function mix(from: string, to: string, k: number, prop: string, color: (raw: string) => RGBA | null): string {
  if (prop.endsWith('color')) {
    const a = color(from) ?? parseRGBA(from) ?? [255, 255, 255, 255];
    const b = color(to) ?? parseRGBA(to) ?? a;
    return a.map((v, i) => Math.round(v + (b[i] - v) * k)).join(' ');
  }
  if (prop === 'position' || prop === 'size') {
    const a = nums(from), b = nums(to);
    return b.map((v, i) => (a[i] ?? 0) + (v - (a[i] ?? 0)) * k).join(' ');
  }
  // xpos/ypos/wide/tall/alpha/fgcolor… keep the destination's notation prefix (c, r) when animating positions
  const pa = parseFloat(from.replace(/^[crs]+/i, '')), pb = parseFloat(to.replace(/^[crs]+/i, ''));
  if (!Number.isFinite(pb)) return k >= 1 ? to : from;
  const prefix = /^[crs]+/i.exec(to)?.[0] ?? '';
  const v = (Number.isFinite(pa) ? pa : 0) + (pb - (Number.isFinite(pa) ? pa : 0)) * k;
  return prefix + (Math.round(v * 100) / 100);
}

export const ANIMATABLE = ['Alpha', 'BgColor', 'FgColor', 'Position', 'Size', 'xpos', 'ypos', 'wide', 'tall', 'Blur', 'TextScan', 'ItemColor', 'SelectionAlpha', 'Ammo2Color', 'MenuColor'];
export const INTERPOLATORS = ['Linear', 'Accel', 'Deaccel', 'Spline', 'Pulse', 'Flicker', 'Gain', 'Bias', 'Bounce'];
