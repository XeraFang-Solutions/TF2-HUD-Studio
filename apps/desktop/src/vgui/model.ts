// Element model: resolved KeyValues blocks → editable VGUI elements with per-property provenance.
import type { RNode, Resolved } from '../api.ts';
import type { LayoutInput } from './layout.ts';

export type PropVal = {
  key: string;
  value: string;
  /** File the value came from (project path, `stock:…` or `tf/…`). */
  file: string;
  editable: boolean;
  /** Byte offset of the value in `file`. */
  offset?: number;
  /** Block id in `file` that holds the value. */
  blockId: string;
};

/** Where edits to an element are written: a file and the chain of block names below that file's root block. */
export type Home = { file: string; path: string[] };

export type El = {
  uid: string;
  name: string;
  control: string;
  props: Map<string, PropVal>;
  /** Nested blocks that are not elements (if_mvm, animation triggers, sub-settings). */
  sub: RNode[];
  children: El[];
  home: Home;
  /** Additional homes whose values were merged in (a hudlayout entry + the element's own .res self-entry). */
  alsoFrom?: Home[];
  /** Synthetic grouping node (e.g. a whole file) that has no settings of its own. */
  group?: boolean;
  /** Not in the file: shown with TF2's built-in defaults; the first edit writes the whole block. */
  synthetic?: boolean;
};

/** TF2's default HudDeathNotice (killfeed) settings, used when a hudlayout doesn't define it. */
export const DEFAULT_DEATHNOTICE: [string, string][] = [
  ['fieldName', 'HudDeathNotice'], ['visible', '1'], ['enabled', '1'], ['xpos', 'r640'], ['ypos', '18'], ['wide', '628'], ['tall', '468'],
  ['MaxDeathNotices', '4'], ['IconScale', '0.35'], ['LineHeight', '16'], ['LineSpacing', '4'], ['CornerRadius', '3'], ['RightJustify', '1'],
  ['TextFont', 'Default'], ['TeamBlue', 'HUDBlueTeamSolid'], ['TeamRed', 'HUDRedTeamSolid'], ['IconColor', 'HudWhite'], ['LocalPlayerColor', 'HUDBlack'],
  ['BaseBackgroundColor', '46 43 42 220'], ['LocalBackgroundColor', '245 229 196 200'],
];

const ELEMENT_KEYS = ['controlname', 'fieldname', 'xpos', 'ypos', 'wide', 'tall', 'visible', 'zpos'];

export function isElementBlock(n: RNode): boolean {
  if (n.value !== undefined) return false;
  return (n.children ?? []).some((c) => c.value !== undefined && ELEMENT_KEYS.includes(c.key.toLowerCase()));
}

/** File label for a resolved file: project path, or `stock:…` / `tf/…` for files the HUD doesn't have. */
export function originLabel(r: Resolved): string {
  if (r.origin.kind === 'project') return r.origin.path;
  if (r.origin.kind === 'game') return `tf/${r.origin.path}`;
  return `stock:${r.origin.path}`;
}

export function rootOf(r: Resolved | null | undefined): RNode | undefined {
  return r?.roots.find((n) => n.value === undefined);
}

/**
 * VGUI applies every block in order: a second block with the same name re-applies its settings to the same
 * panel, so later values override earlier ones (HUDs use this to hide stock elements at the end of a file).
 */
function mergeDuplicates(blocks: RNode[]): RNode[][] {
  const groups = new Map<string, RNode[]>();
  for (const b of blocks) {
    const k = b.key.toLowerCase();
    groups.set(k, [...(groups.get(k) ?? []), b]);
  }
  return [...groups.values()];
}

function groupToEl(group: RNode[], home: Home, uidPrefix: string): El {
  const props = new Map<string, PropVal>();
  const sub: RNode[] = [];
  const childBlocks: RNode[] = [];
  for (const node of group) {
    for (const c of node.children ?? []) {
      if (c.value !== undefined) {
        const k = c.key.toLowerCase();
        // within one block the first occurrence wins; a later duplicate block overrides
        if (!props.has(k) || props.get(k)!.blockId !== c.id) props.set(k, { key: c.key, value: c.value, file: c.file, editable: c.editable, offset: c.offset, blockId: c.id });
      } else if (isElementBlock(c)) {
        childBlocks.push(c);
      } else {
        sub.push(c);
      }
    }
  }
  const children = mergeDuplicates(childBlocks).map((g) => groupToEl(g, { file: home.file, path: [...home.path, g[0].key] }, `${uidPrefix}/${g[0].key}`));
  return { uid: uidPrefix, name: group[0].key, control: (props.get('controlname')?.value ?? '').toLowerCase(), props, sub, children, home };
}

export function toEl(node: RNode, home: Home, uidPrefix: string): El {
  return groupToEl([node], home, uidPrefix);
}

/** Elements of one file (the root block's element children), duplicates merged like VGUI does. */
export function fileElements(r: Resolved, file: string, uidPrefix = file): El[] {
  const root = rootOf(r);
  if (!root) return [];
  return mergeDuplicates((root.children ?? []).filter(isElementBlock)).map((g) => groupToEl(g, { file, path: [g[0].key] }, `${uidPrefix}>${g[0].key}`));
}

function stem(path: string): string {
  const f = path.split('/').pop() ?? path;
  return f.replace(/\.res$/i, '').toLowerCase();
}

/** Fills properties missing from `into` with those of `from` (into wins, like hudlayout over the self-entry). */
function absorb(into: El, from: El) {
  for (const [k, v] of from.props) if (!into.props.has(k)) into.props.set(k, v);
  into.children.push(...from.children);
  into.sub.push(...from.sub);
  into.alsoFrom = [...(into.alsoFrom ?? []), from.home];
  if (!into.control) into.control = from.control;
}

/**
 * The in-game composition: every hudlayout element, with the .res files it loads nested inside.
 * A file's self-entry (block named like the element or the file) configures the element itself; its other
 * blocks are the element's children.
 */
export function buildComposite(hudlayout: Resolved | null, elementFiles: Record<string, string[]>, files: Record<string, Resolved | null>, only?: Set<string>): El[] {
  const root = rootOf(hudlayout);
  if (!root || !hudlayout) return [];
  const layoutFile = originLabel(hudlayout);
  const out: El[] = [];
  for (const block of (root.children ?? []).filter((c) => c.value === undefined)) {
    const fs = elementFiles[block.key] ?? [];
    if (only && !only.has(block.key.toLowerCase())) continue;
    const el = toEl(block, { file: layoutFile, path: [block.key] }, `${layoutFile}>${block.key}`);
    for (const f of fs) {
      const r = files[f];
      if (!r) continue;
      const own = originLabel(r);
      const els = fileElements(r, own);
      const selfIdx = els.findIndex((e) => e.name.toLowerCase() === block.key.toLowerCase() || e.name.toLowerCase() === stem(f));
      const self = selfIdx >= 0 ? els.splice(selfIdx, 1)[0] : undefined;
      if (self && self.name.toLowerCase() === block.key.toLowerCase()) {
        absorb(el, self);
        el.children.push(...els);
      } else if (self) {
        self.children.push(...els);
        el.children.push(self);
      } else {
        el.children.push(...els);
      }
    }
    out.push(el);
  }
  if (!out.some((e) => e.name.toLowerCase() === 'huddeathnotice') && (!only || only.has('huddeathnotice'))) {
    const props = new Map<string, PropVal>(DEFAULT_DEATHNOTICE.map(([k, v]) => [k.toLowerCase(), { key: k, value: v, file: 'TF2 built-in default', editable: false, blockId: '' }]));
    out.push({ uid: `${layoutFile}>HudDeathNotice`, name: 'HudDeathNotice', control: '', props, sub: [], children: [], home: { file: layoutFile, path: ['HudDeathNotice'] }, synthetic: true });
  }
  return out;
}

export function layoutInputs(els: El[], value: (el: El, key: string) => string | undefined): LayoutInput[] {
  return els.map((el) => ({
    uid: el.uid,
    names: [el.name.toLowerCase(), (el.props.get('fieldname')?.value ?? el.name).toLowerCase()],
    get: (k: string) => value(el, k),
    children: layoutInputs(el.children, value),
  }));
}

export function walk(els: El[], fn: (el: El, depth: number, parent: El | null) => void, depth = 0, parent: El | null = null) {
  for (const el of els) {
    fn(el, depth, parent);
    walk(el.children, fn, depth + 1, el);
  }
}

export function findEl(els: El[], uid: string): El | undefined {
  let found: El | undefined;
  walk(els, (e) => {
    if (!found && e.uid === uid) found = e;
  });
  return found;
}

/** Home that should receive an edit of `key`: where the value currently lives if that's an editable
 *  project file of this element, otherwise the element's primary home (creating an override). */
export function editTarget(el: El, key: string): Home {
  const pv = el.props.get(key.toLowerCase());
  if (pv && pv.editable) {
    for (const h of [el.home, ...(el.alsoFrom ?? [])]) if (h.file === pv.file) return h;
  }
  if (!el.home.file.startsWith('stock:') && !el.home.file.startsWith('tf/')) return el.home;
  const editable = (el.alsoFrom ?? []).find((h) => !h.file.startsWith('stock:') && !h.file.startsWith('tf/'));
  return editable ?? el.home;
}

export function displayName(el: El): string {
  return el.name;
}
