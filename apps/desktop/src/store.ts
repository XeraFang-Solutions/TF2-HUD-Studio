// Global editor state with a tiny subscribe/notify store (no external state library needed).
import type { Theme } from './theme.ts';
import { useSyncExternalStore } from 'react';
import { api, errorText, inTauri, type Animations, type AppInfo, type GameStatus, type Settings, type Doc, type ElementEdit, type FileEntry, type Problem, type ProjectInfo, type Resolved, type Scheme } from './api.ts';
import { buildComposite, fileElements, findEl, editTarget, originLabel, walk, type El, type Home } from './vgui/model.ts';
import { SchemeIndex, clearFonts, registerFonts } from './vgui/scheme.ts';
import { ASPECTS } from './vgui/layout.ts';
import { clearImageCache } from './vgui/imageCache.ts';

export type Sidebar = 'files' | 'elements' | 'search' | 'problems' | 'assets' | 'game';
export type GameLine = { id: number; kind: 'log' | 'cmd' | 'reply' | 'error' | 'info'; text: string };
export type View = 'design' | 'source' | 'scheme' | 'animations';
export type Toast = { id: number; kind: 'ok' | 'error' | 'info'; text: string };
export type Playback = { event: string; t: number; playing: boolean };
export const CLASSES = ['scout', 'soldier', 'pyro', 'demoman', 'heavy', 'engineer', 'medic', 'sniper', 'spy'] as const;
export type Scenario = (typeof CLASSES)[number] | 'all';

export type State = {
  app: AppInfo | null;
  project: ProjectInfo | null;
  files: FileEntry[];
  tabs: string[];
  active: string | null;
  docs: Record<string, Doc>;
  view: View;
  sidebar: Sidebar;
  scheme: Scheme | null;
  schemeIndex: SchemeIndex;
  fontsVersion: number;
  /** Theme being edited (shown live, saved shortly after). */
  themeDraft: Theme | null;
  /** TF2 font files found for the interface (null until checked). */
  uiFonts: string[] | null;
  /** hudlayout element → .res files it loads */
  elementFiles: Record<string, string[]>;
  model: El[];
  modelLabel: string;
  modelError: string | null;
  selection: string[];
  hidden: Set<string>;
  resolution: { w: number; h: number };
  zoom: number | 'fit';
  showHidden: boolean;
  showGrid: boolean;
  snap: number;
  composite: boolean;
  /** Whole-HUD preview: only what's on screen during normal play, or every element. */
  scenario: Scenario;
  animations: Animations | null;
  playback: Playback | null;
  problems: Problem[];
  localized: Record<string, string>;
  toasts: Toast[];
  busy: string | null;
  /** files edited, most recent last — Ctrl+Z walks this across files */
  history: string[];
  future: string[];
  reveal: { file: string; offset?: number; line?: number; nonce: number } | null;
  dialog: null | { kind: 'settings' | 'new-hud' | 'deploy' | 'tools' | 'quick-open' | 'add-element' | 'about' | 'library' | 'hud-details' | 'theme'; path?: string };
  /** TF2 started from HUD Studio: console.log lines and remote console traffic */
  gameLog: GameLine[];
  game: GameStatus | null;
  ask: null | { message: string; ok: string; resolve: (yes: boolean) => void };
  menu: null | { x: number; y: number; items: { label: string; run: () => void; disabled?: boolean }[] };
};

const saved = (() => {
  try {
    return JSON.parse(localStorage.getItem('hudstudio.ui') ?? '{}');
  } catch {
    return {};
  }
})();

let state: State = {
  app: null, project: null, files: [], tabs: [], active: null, docs: {}, view: 'design', sidebar: 'files',
  scheme: null, schemeIndex: new SchemeIndex(null), fontsVersion: 0, themeDraft: null, uiFonts: null, elementFiles: {}, model: [], modelLabel: '', modelError: null,
  selection: [], hidden: new Set(), resolution: saved.resolution ?? { w: ASPECTS[0].w, h: ASPECTS[0].h }, zoom: 'fit',
  showHidden: saved.showHidden ?? false, showGrid: saved.showGrid ?? false, snap: saved.snap ?? 1, composite: saved.composite ?? true, scenario: (CLASSES as readonly string[]).includes(saved.scenario) || saved.scenario === 'all' ? saved.scenario : 'soldier',
  animations: null, playback: null, problems: [], localized: {}, toasts: [], busy: null, history: [], future: [], reveal: null, dialog: null, ask: null, menu: null, gameLog: [], game: null,
};

const listeners = new Set<() => void>();
export function getState() {
  return state;
}
export function set(patch: Partial<State> | ((s: State) => Partial<State>)) {
  const p = typeof patch === 'function' ? patch(state) : patch;
  state = { ...state, ...p };
  if ('resolution' in p || 'showHidden' in p || 'showGrid' in p || 'snap' in p || 'composite' in p || 'scenario' in p) {
    try {
      localStorage.setItem('hudstudio.ui', JSON.stringify({ resolution: state.resolution, showHidden: state.showHidden, showGrid: state.showGrid, snap: state.snap, composite: state.composite, scenario: state.scenario }));
    } catch {
      /* storage unavailable */
    }
  }
  listeners.forEach((l) => l());
}
export function useStore<T>(sel: (s: State) => T): T {
  return useSyncExternalStore((cb) => (listeners.add(cb), () => listeners.delete(cb)), () => sel(state));
}

/** In-app confirmation (the webview's native confirm() is unreliable on some platforms). */
export function ask(message: string, ok = 'OK'): Promise<boolean> {
  return new Promise((resolve) => {
    const prev = state.ask;
    if (prev) prev.resolve(false);
    set({ ask: { message, ok, resolve: (yes: boolean) => { set({ ask: null }); resolve(yes); } } });
  });
}

let toastId = 0;
export function toast(text: string, kind: Toast['kind'] = 'info') {
  const id = ++toastId;
  set((s) => ({ toasts: [...s.toasts, { id, kind, text }] }));
  setTimeout(() => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })), kind === 'error' ? 9000 : 4000);
}

export async function guard<T>(label: string | null, fn: () => Promise<T>): Promise<T | undefined> {
  if (label) set({ busy: label });
  try {
    return await fn();
  } catch (e) {
    toast(errorText(e), 'error');
    return undefined;
  } finally {
    if (label) set({ busy: null });
  }
}

// ── project ────────────────────────────────────────────────────────────────

export const isRes = (p: string) => /\.res$/i.test(p);
export const isAnimFile = (p: string) => /(^|\/)hudanimations[^/]*\.txt$/i.test(p) && !/manifest/i.test(p);
export const isSchemeFile = (p: string) => /clientscheme|(^|\/)resource\/scheme\//i.test(p);
export const isText = (p: string) => /\.(res|vdf|txt|cfg|rc|vmt|md)$/i.test(p);

let gameLineId = 0;
const GAME_LOG_MAX = 4000;
export function addGameLines(kind: GameLine['kind'], lines: string[]) {
  if (!lines.length) return;
  set((s) => {
    const next = [...s.gameLog, ...lines.map((text) => ({ id: ++gameLineId, kind, text }))];
    return { gameLog: next.length > GAME_LOG_MAX ? next.slice(next.length - GAME_LOG_MAX) : next };
  });
}

export async function refreshGame() {
  const game = await api.gameStatus().catch(() => null);
  set({ game });
  return game;
}

/** Runs a console command in TF2 through the remote console and logs it. */
export async function gameCommand(command: string, quiet = false): Promise<string | undefined> {
  if (!quiet) addGameLines('cmd', [`] ${command}`]);
  try {
    const out = await api.gameRcon(command);
    if (!quiet && out.trim()) addGameLines('reply', out.replace(/\n$/, '').split('\n'));
    return out;
  } catch (e) {
    if (!quiet) addGameLines('error', [errorText(e)]);
    else throw e;
    return undefined;
  }
}

export async function updateSettings(patch: Partial<Settings>) {
  const app = state.app;
  if (!app) return;
  const settings = await guard(null, () => api.settingsSave({ ...app.settings, ...patch }));
  if (settings) set({ app: { ...app, settings } });
  return settings;
}

export async function init() {
  if (inTauri) {
    const { listen } = await import('@tauri-apps/api/event');
    void listen<string[]>('game-log', (e) => addGameLines('log', e.payload));
    void listen<string>('game-state', () => void refreshGame());
  }
  const app = await guard(null, () => api.appInit());
  if (app) set({ app });
  if (app?.launchPath) await openProject(app.launchPath);
}

export async function openProject(path: string) {
  const project = await guard('Opening HUD…', () => api.projectOpen(path));
  if (!project) return;
  set({ project, files: project.files, tabs: [], active: null, docs: {}, selection: [], model: [], history: [], future: [], problems: [], animations: null, playback: null, localized: {} });
  clearImageCache();
  if (project.gameError) toast(`TF2 files unavailable: ${project.gameError}`, 'error');
  await refreshSemantics();
  const start = project.files.find((f) => /^scripts\/hudlayout\.res$/i.test(f.path)) ?? project.files.find((f) => isRes(f.path));
  if (start) await openFile(start.path);
  void refreshProblems();
  if (state.app) void guard(null, () => api.appInit()).then((a) => a && set({ app: { ...a, launchPath: null } }));
}

export async function refreshFiles() {
  const files = await guard(null, () => api.projectRefresh());
  if (files) { set({ files }); clearImageCache(); }
}

/** Scheme, fonts, element→file mapping and animations: things that change rarely. */
let semanticsGeneration = 0;
export async function refreshSemantics() {
  const generation = ++semanticsGeneration;
  clearImageCache();
  clearFonts();
  const [scheme, animations] = await Promise.all([guard(null, () => api.scheme()), guard(null, () => api.animations())]);
  if (generation !== semanticsGeneration) return;
  set({ scheme: scheme ?? null, schemeIndex: new SchemeIndex(scheme ?? null), animations: animations ?? null });
  void guard(null, () => api.fonts()).then(async (fonts) => {
    if (fonts && generation === semanticsGeneration) {
      await registerFonts(fonts);
      set((s) => ({ fontsVersion: s.fontsVersion + 1 }));
    }
  });
  const layout = await guard(null, () => api.resolve('scripts/hudlayout.res'));
  if (generation !== semanticsGeneration) return;
  const names = (layout?.roots.find((r) => r.value === undefined)?.children ?? []).filter((c) => c.value === undefined).map((c) => c.key);
  const elementFiles = (await guard(null, () => api.elementFiles(names))) ?? {};
  if (generation !== semanticsGeneration) return;
  set({ elementFiles });
}

export async function refreshProblems() {
  const problems = await guard(null, () => api.problems());
  if (problems) set({ problems });
}

// ── documents ──────────────────────────────────────────────────────────────

export function viewFor(path: string): View {
  if (isAnimFile(path)) return 'animations';
  if (isSchemeFile(path)) return 'scheme';
  if (isRes(path)) return 'design';
  return 'source';
}

export async function openFile(path: string, opts: { offset?: number; line?: number; view?: View } = {}) {
  const doc = await guard(null, () => api.open(path));
  if (!doc) return;
  set((s) => ({
    docs: { ...s.docs, [doc.path]: doc },
    tabs: s.tabs.includes(doc.path) ? s.tabs : [...s.tabs, doc.path],
    active: doc.path,
    view: opts.view ?? (opts.offset !== undefined || opts.line !== undefined ? 'source' : viewFor(doc.path)),
    selection: s.active === doc.path ? s.selection : [],
    reveal: opts.offset !== undefined || opts.line !== undefined ? { file: doc.path, offset: opts.offset, line: opts.line, nonce: Date.now() } : s.reveal,
  }));
  await rebuildModel();
  void localizeModel();
}

export async function closeTab(path: string) {
  const doc = state.docs[path];
  if (doc?.dirty && !(await ask(`${path} has unsaved changes. Close the tab anyway? The changes stay in memory and are still saved by Save all.`, 'Close tab'))) return;
  void api.close(path);
  set((s) => {
    const tabs = s.tabs.filter((t) => t !== path);
    const idx = s.tabs.indexOf(path);
    const active = s.active === path ? tabs[Math.min(idx, tabs.length - 1)] ?? null : s.active;
    return { tabs, active, view: active ? viewFor(active) : s.view };
  });
  await rebuildModel();
}

function applyDoc(doc: Doc, recordHistory = true) {
  set((s) => ({
    docs: { ...s.docs, [doc.path]: doc },
    history: recordHistory ? [...s.history.slice(-400), doc.path] : s.history,
    future: recordHistory ? [] : s.future,
  }));
}

let rebuildTimer: ReturnType<typeof setTimeout> | null = null;
function scheduleRebuild(path: string) {
  if (rebuildTimer) clearTimeout(rebuildTimer);
  rebuildTimer = setTimeout(async () => {
    if (isSchemeFile(path)) {
      const scheme = await guard(null, () => api.scheme());
      if (scheme) set({ scheme, schemeIndex: new SchemeIndex(scheme) });
    }
    if (isAnimFile(path) || /manifest/i.test(path)) {
      const animations = await guard(null, () => api.animations());
      if (animations) set({ animations });
    }
    if (/hudlayout\.res$/i.test(path)) await refreshSemantics();
    await rebuildModel();
  }, 60);
}

export async function docEdit(path: string, fn: () => Promise<Doc>) {
  const doc = await guard(null, fn);
  if (!doc) return;
  applyDoc(doc);
  scheduleRebuild(path);
  return doc;
}

export async function setSource(path: string, source: string) {
  if (state.docs[path]?.source === source) return;
  return docEdit(path, () => api.setSource(path, source));
}

export async function undo() {
  const path = state.history[state.history.length - 1];
  if (!path) return;
  const doc = await guard(null, () => api.undo(path));
  if (!doc) return;
  set((s) => ({ docs: { ...s.docs, [path]: doc }, history: s.history.slice(0, -1), future: [...s.future, path] }));
  scheduleRebuild(path);
}

export async function redo() {
  const path = state.future[state.future.length - 1];
  if (!path) return;
  const doc = await guard(null, () => api.redo(path));
  if (!doc) return;
  set((s) => ({ docs: { ...s.docs, [path]: doc }, future: s.future.slice(0, -1), history: [...s.history, path] }));
  scheduleRebuild(path);
}

export async function saveAll() {
  const r = await guard('Saving…', () => api.saveAll());
  if (!r) return;
  set((s) => {
    const docs = { ...s.docs };
    for (const d of r.documents) docs[d.path] = d;
    return { docs };
  });
  toast(r.saved.length ? `Saved ${r.saved.length} file${r.saved.length > 1 ? 's' : ''}${r.backup ? ' · previous versions backed up' : ''}` : 'Nothing to save', 'ok');
  void refreshProblems();
  if (r.saved.length) void reloadHudInGame();
}

/** After a save: tell a running TF2 (started from HUD Studio with the remote console) to reload the HUD. */
async function reloadHudInGame() {
  if (!state.app?.settings.launch?.reloadOnSave || !state.game?.rcon) return;
  try {
    await gameCommand('hud_reloadscheme', true);
    addGameLines('info', ['HUD reloaded in TF2 after saving']);
  } catch {
    /* game closed or still loading */
  }
}

export async function saveActive() {
  const p = state.active;
  if (!p) return;
  const r = await guard('Saving…', () => api.save(p));
  if (!r) return;
  set((s) => {
    const docs = { ...s.docs };
    for (const d of r.documents) docs[d.path] = d;
    return { docs };
  });
  toast(r.saved.length ? `Saved ${p}` : 'No changes to save', 'ok');
  void refreshProblems();
}

// ── element model ──────────────────────────────────────────────────────────

const HUDLAYOUT = 'scripts/hudlayout.res';

/** HUD elements on screen during ordinary play; everything else (menus, win panels, game-mode HUDs) is situational. */
const IN_GAME_ELEMENTS = ['hudmatchstatus', 'hudchat', 'huddeathnotice', 'hudplayerstatus', 'hudweaponammo', 'hudobjectivestatus', 'huditemeffectmeter', 'chealthaccountpanel', 'cdamageaccountpanel', 'hudcrosshair', 'hudtfcrosshair', 'hudroundtimer', 'hudcontrolpointicons'];
const CLASS_ELEMENTS: Record<string, string[]> = {
  medic: ['hudmediccharge'],
  demoman: ['huddemomanpipes', 'huddemomancharge'],
  engineer: ['buildingstatus_engineer', 'chudaccountpanel'],
  spy: ['disguisestatus', 'buildingstatus_spy'],
  sniper: ['hudbowcharge'],
  pyro: ['hudflamerocketcharge'],
};
export function scenarioElements(s: Scenario): Set<string> | undefined {
  return s === 'all' ? undefined : new Set([...IN_GAME_ELEMENTS, ...(CLASS_ELEMENTS[s] ?? [])]);
}

/** Rebuilds the preview model for the active file from the backend's resolved view (includes unsaved edits). */
export async function rebuildModel() {
  const active = state.active;
  if (!active || (!isRes(active) && !isAnimFile(active)) || isSchemeFile(active)) {
    set({ model: [], modelLabel: '', modelError: null });
    return;
  }
  try {
    const isLayout = /(^|\/)hudlayout\.res$/i.test(active) || isAnimFile(active);
    const owners = Object.entries(state.elementFiles).filter(([, fs]) => fs.some((f) => f.toLowerCase() === active.toLowerCase())).map(([n]) => n.toLowerCase());
    if (isLayout || (state.composite && owners.length)) {
      const wanted = new Set<string>([HUDLAYOUT]);
      const only = isLayout ? scenarioElements(state.scenario) : new Set(owners);
      for (const [name, fs] of Object.entries(state.elementFiles)) if (!only || only.has(name.toLowerCase())) fs.forEach((f) => wanted.add(f));
      const resolved = await api.resolveMany([...wanted]);
      const model = buildComposite(resolved[HUDLAYOUT] ?? null, state.elementFiles, resolved, only);
      set({ model, modelLabel: isLayout ? (state.scenario === 'all' ? 'Whole HUD · every element' : `Whole HUD · playing ${state.scenario}`) : `Inside ${owners.join(', ')} from hudlayout`, modelError: null });
    } else {
      const r = await api.resolve(active);
      const model = r ? fileElements(r, originLabel(r)) : [];
      set({ model, modelLabel: 'This file on a full screen', modelError: r ? null : 'File could not be resolved' });
    }
  } catch (e) {
    set({ modelError: errorText(e) });
  }
}

async function localizeModel() {
  const tokens = new Set<string>();
  walk(state.model, (el) => {
    for (const [k, v] of el.props) if ((k === 'labeltext' || k.endsWith('text')) && v.value.startsWith('#') && !(v.value in state.localized)) tokens.add(v.value);
  });
  if (!tokens.size) return;
  const found = await guard(null, () => api.localize([...tokens]));
  if (found) set((s) => ({ localized: { ...s.localized, ...found, ...Object.fromEntries([...tokens].filter((t) => !(t in found)).map((t) => [t, t.slice(1)])) } }));
}

/** Writes element property edits, grouped per file, copying stock files into the HUD first when needed. */
export async function editElements(edits: { el: El; key: string; value: string | null }[]) {
  // An element shown with TF2's built-in defaults (not in the file yet) is written out in full on its first edit.
  for (const e of [...edits]) {
    if (!e.el.synthetic) continue;
    for (const [k, pv] of e.el.props) if (!edits.some((x) => x.el === e.el && x.key.toLowerCase() === k)) edits.unshift({ el: e.el, key: pv.key, value: pv.value });
    e.el.synthetic = false;
  }
  const copied = new Set<string>();
  const byFile = new Map<string, ElementEdit[]>();
  for (const e of edits) {
    let home: Home = editTarget(e.el, e.key);
    if (home.file.startsWith('stock:') || home.file.startsWith('tf/')) {
      const stockPath = home.file.replace(/^stock:|^tf\//, '');
      if (!copied.has(stockPath)) {
        if (!(await ask(`${stockPath} comes from TF2's default HUD. Copy it into your HUD so it can be edited?`, 'Copy and edit'))) return;
        copied.add(stockPath);
        const files = await guard('Copying file…', () => api.copyStock(stockPath));
        if (!files) return;
        set({ files });
        await refreshSemantics();
        toast(`Copied ${stockPath} into your HUD`, 'ok');
      }
      home = { file: stockPath, path: home.path };
    }
    const list = byFile.get(home.file) ?? [];
    const pv = e.el.props.get(e.key.toLowerCase());
    const blockId = pv && pv.editable && pv.file === home.file && pv.blockId ? pv.blockId : undefined;
    list.push({ element: home.path, property: e.key, value: e.value, blockId });
    byFile.set(home.file, list);
  }
  for (const [file, list] of byFile) {
    const doc = await guard(null, () => api.elementEdits(file, list));
    if (!doc) return;
    applyDoc(doc);
    if (!state.tabs.includes(file)) set((s) => ({ tabs: [...s.tabs, file] }));
  }
  await rebuildModel();
}

export function selectedElements(): El[] {
  return state.selection.map((u) => findEl(state.model, u)).filter((e): e is El => !!e);
}

export function resolvedPath(r: Resolved) {
  return r.origin.kind === 'project' ? r.origin.path : r.path;
}
