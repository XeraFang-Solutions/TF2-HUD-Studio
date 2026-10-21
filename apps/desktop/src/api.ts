// Typed wrappers around the Rust backend commands.
import { invoke } from '@tauri-apps/api/core';

export type Span = { start: number; end: number };
export type Property = { name: string; value: string; span: Span; condition: string | null };
export type Diagnostic = { message: string; span: Span };
export type Panel = { id: string; name: string; control: string | null; properties: Property[]; span: Span };
export type Doc = {
  path: string; source: string; panels: Panel[]; diagnostics: Diagnostic[]; dirty: boolean;
  encoding: 'utf8' | 'utf16Le' | 'utf16Be'; canUndo: boolean; canRedo: boolean;
};
export type FileEntry = { path: string; size: number; modified: number };
export type CustomHud = { name: string; folder: string; path: string };
export type GameInstall = { tfDir: string; huds: CustomHud[] };
export type GameInfo = { tfDir: string; vpkFiles: number; huds: CustomHud[] };
export type Settings = {
  version: number; tfDir: string | null; tools: Record<string, string>; recentProjects: string[];
  backups: boolean; platform: 'windows' | 'linux' | 'mac'; ui: Record<string, unknown>;
  editor: AppChoice; imageEditor: AppChoice;
  hudLibrary: string | null; installMode: InstallMode; launch: LaunchSettings;
};
export type InstallMode = 'link' | 'copy';
export type LaunchSettings = {
  bits: '64' | '32'; method: 'direct' | 'steam'; width: number | null; height: number | null;
  itemtest: boolean; rcon: boolean; consoleLog: boolean; reloadOnSave: boolean; extraArgs: string;
};
export type Manifest = {
  name: string; version: string; author: string; description: string; icon: string; banner: string;
  homepage: string; source: string; license: string; tags: string;
};
export type HudDetails = { hasManifest: boolean; manifest: Manifest; iconUrl: string | null; bannerUrl: string | null };
export type CustomEntry = { name: string; folder: string; path: string; kind: 'link' | 'copy'; target: string | null; isHud: boolean; libraryFolder: string | null };
export type LibraryHud = { name: string; folder: string; path: string; installs: CustomEntry[]; details: HudDetails };
export type LibraryState = { library: string | null; huds: LibraryHud[]; custom: CustomEntry[] };
export type MarketplaceEntry = { id: string; name: string; author: string; album: string | null; repository: string | null; homepage: string | null; download: string | null };
export type MarketplaceCatalogue = { entries: MarketplaceEntry[]; cached: boolean; warning: string | null };
export type InstallResult = { target: string; mode: InstallMode; files: number; backedUp: string[]; unlinked: string[] };
export type LaunchInfo = { method: string; program: string; args: string[]; rconPort: number | null; logPath: string | null; size: [number, number] };
export type GameStatus = { running: boolean | null; rcon: boolean; logPath: string | null };
export type ImportResult = { image: string; texture: string; material: string | null; width: number; height: number };
export type AppChoice = { id: string; path: string | null; args: string | null };
export type EditorInfo = { id: string; name: string; detected: boolean; path: string | null; terminal: boolean; opensAtLine: boolean };
export type AppCategory = 'text' | 'image';
export type AppInfo = { version: string; settings: Settings; installs: GameInstall[]; backupDir: string; launchPath: string | null };
export type ProjectInfo = {
  root: string; name: string; files: FileEntry[]; game: GameInfo | null; gameError: string | null; insideTfCustom: boolean;
};
export type RNode = {
  key: string; value?: string; children?: RNode[]; file: string; editable: boolean; id: string; offset?: number;
};
export type Resolved = { path: string; origin: { kind: string; path: string }; roots: RNode[]; chain: string[]; missing: { from: string; path: string; offset: number }[] };
export type ColorDef = { name: string; raw: string; rgba: [number, number, number, number] | null; file: string; editable: boolean; id: string; offset: number | null };
export type FontVariant = { key: string; props: Record<string, string> };
export type FontDef = { name: string; variants: FontVariant[]; file: string; editable: boolean; id: string };
export type FontFile = { path: string; declaredName: string | null; found: boolean; origin: string | null };
export type Scheme = {
  file: string | null; chain: string[]; colors: ColorDef[]; baseSettings: [string, string][]; fonts: FontDef[];
  borders: RNode[]; fontFiles: FontFile[];
};
export type FontData = { families: string[]; path: string; base64: string; cellRatio: number; ascentRatio: number };
export type ImageResult = { dataUrl: string; width: number; height: number; material: string | null; texture: string; origin: string; format: string };
export type AnimCommand = {
  kind: string; panel?: string; property?: string; value?: string; interpolator?: string; param?: number;
  event?: string; start: number; duration: number; condition?: string; line: number;
};
export type AnimEvent = { name: string; file: string; editable: boolean; line: number; start: number; end: number; commands: AnimCommand[]; shadowedBy?: string };
export type Animations = { files: string[]; events: AnimEvent[]; problems: { file: string; line: number; message: string }[] };
export type Problem = { severity: 'error' | 'warning' | 'info'; file: string; offset: number | null; line: number; message: string };
export type SearchHit = { file: string; line: number; column: number; offset: number; length: number; preview: string };
export type ExternalTool = { id: string; name: string; kind: 'gui' | 'cli'; about: string; path: string | null; available: boolean; detected: boolean; opens: string[] };
export type SaveResult = { saved: string[]; backup: string | null; documents: Doc[] };
export type BufferState = { path: string; dirty: boolean; changedOnDisk: boolean };
export type ElementEdit = { element: string[]; property: string; value: string | null; blockId?: string };
export type BlockResult = { document: Doc; id: string };

export const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export const api = {
  appInit: () => invoke<AppInfo>('app_init'),
  settingsSave: (settings: Settings) => invoke<Settings>('settings_save', { settings }),
  gameSetDir: (path: string) => invoke<GameInfo>('game_set_dir', { path }),
  projectOpen: (path: string) => invoke<ProjectInfo>('project_open', { path }),
  projectRefresh: () => invoke<FileEntry[]>('project_refresh'),
  projectCreate: (parent: string, name: string, template: string) => invoke<string>('project_create', { parent, name, template }),
  projectCreateFile: (path: string, content: string) => invoke<FileEntry[]>('project_create_file', { file: { path, content } }),
  open: (path: string) => invoke<Doc>('document_open', { path }),
  setSource: (path: string, source: string) => invoke<Doc>('document_set_source', { path, source }),
  setProperty: (path: string, panelId: string, property: string, value: string, at?: number | null) =>
    invoke<Doc>('document_set_property', { path, panelId, property, value, at: at ?? null }),
  removeProperty: (path: string, panelId: string, property: string, at?: number | null) =>
    invoke<Doc>('document_remove_property', { path, panelId, property, at: at ?? null }),
  insertBlock: (path: string, parentId: string, name: string, properties: [string, string][]) =>
    invoke<BlockResult>('document_insert_block', { path, parentId, name, properties }),
  duplicateBlock: (path: string, id: string, newName: string) => invoke<BlockResult>('document_duplicate_block', { path, id, newName }),
  renameBlock: (path: string, id: string, newName: string) => invoke<BlockResult>('document_rename_block', { path, id, newName }),
  removeBlock: (path: string, id: string) => invoke<Doc>('document_remove_block', { path, id }),
  elementEdits: (path: string, edits: ElementEdit[]) => invoke<Doc>('element_edits', { path, edits }),
  undo: (path: string) => invoke<Doc>('document_undo', { path }),
  redo: (path: string) => invoke<Doc>('document_redo', { path }),
  revert: (path: string) => invoke<Doc>('document_revert', { path }),
  reload: (path: string) => invoke<Doc>('document_reload', { path }),
  close: (path: string) => invoke<void>('document_close', { path }),
  save: (path: string) => invoke<SaveResult>('document_save', { path }),
  saveAll: () => invoke<SaveResult>('document_save_all'),
  status: () => invoke<BufferState[]>('documents_status'),
  fileText: (path: string) => invoke<[string, string] | null>('file_text', { path }),
  stockFiles: () => invoke<string[]>('stock_files'),
  copyStock: (path: string) => invoke<FileEntry[]>('project_copy_stock', { path }),
  resolve: (path: string) => invoke<Resolved | null>('resolve_file', { path }),
  resolveMany: (paths: string[]) => invoke<Record<string, Resolved | null>>('resolve_files', { paths }),
  elementFiles: (names: string[]) => invoke<Record<string, string[]>>('element_files', { names }),
  scheme: () => invoke<Scheme>('scheme_get'),
  fonts: () => invoke<FontData[]>('fonts_get'),
  image: (name: string) => invoke<ImageResult>('image_get', { name }),
  animations: () => invoke<Animations>('animations_get'),
  problems: () => invoke<Problem[]>('problems_get'),
  search: (query: string, regex: boolean, caseSensitive: boolean) => invoke<SearchHit[]>('search', { query, regex, caseSensitive }),
  assets: (includeStock: boolean) => invoke<{ path: string; stock: boolean }[]>('assets_list', { includeStock }),
  localize: (tokens: string[]) => invoke<Record<string, string>>('localize', { tokens }),
  deploy: (folder: string, mode: InstallMode) => invoke<InstallResult>('deploy', { folder, mode }),
  exportZip: (dest: string) => invoke<number>('export_zip', { dest }),
  openPath: (path: string) => invoke<void>('open_path', { path }),
  editorsList: (category: AppCategory) => invoke<EditorInfo[]>('editors_list', { category }),
  editorOpen: (path: string, line?: number) => invoke<string>('editor_open', { path, line: line ?? null }),
  imageEditorOpen: (path: string, asPng: boolean) => invoke<string>('image_editor_open', { path, asPng }),
  toolsList: () => invoke<ExternalTool[]>('tools_list'),
  toolsRegister: (id: string, path: string | null) => invoke<void>('tools_register', { id, path }),
  toolsLaunch: (id: string) => invoke<void>('tools_launch', { id }),
  toolsOpenWith: (id: string, path: string) => invoke<void>('tools_open_with', { id, path }),
  imageImport: (source: string, options: { name: string; compress: boolean; powerOfTwo: boolean; overwrite: boolean }) => invoke<ImportResult>('image_import', { source, options }),
  imagePngBack: (texture: string, compress: boolean) => invoke<string>('image_png_back', { texture, compress }),
  libraryGet: () => invoke<LibraryState>('library_get'),
  marketplaceList: () => invoke<MarketplaceCatalogue>('marketplace_list'),
  marketplaceImport: (id: string) => invoke<string>('marketplace_import', { id }),
  marketplaceOpen: (url: string) => invoke<void>('marketplace_open', { url }),
  marketplacePreview: (id: string) => invoke<string | null>('marketplace_preview', { id }),
  librarySet: (path: string | null) => invoke<LibraryState>('library_set', { path }),
  libraryInstall: (args: { folder?: string; path?: string; name?: string; mode?: InstallMode }) =>
    invoke<InstallResult>('library_install', { folder: args.folder ?? null, path: args.path ?? null, name: args.name ?? null, mode: args.mode ?? null }),
  librarySwapPlan: (folder: string) => invoke<CustomEntry[]>('library_swap_plan', { folder }),
  librarySwap: (folder: string, mode?: InstallMode) => invoke<InstallResult>('library_swap', { folder, mode: mode ?? null }),
  libraryUninstall: (customFolder: string) => invoke<InstallResult>('library_uninstall', { customFolder }),
  libraryAdopt: (customFolder: string, mode?: InstallMode) => invoke<LibraryState>('library_adopt', { customFolder, mode: mode ?? null }),
  libraryAdd: (path: string) => invoke<string>('library_add', { path }),
  manifestGet: (path?: string) => invoke<HudDetails>('manifest_get', { path: path ?? null }),
  manifestSave: (manifest: Manifest, path?: string) => invoke<HudDetails>('manifest_save', { path: path ?? null, manifest }),
  manifestSetImage: (slot: 'icon' | 'banner', image: string, path?: string) => invoke<HudDetails>('manifest_set_image', { path: path ?? null, slot, image }),
  displaySize: () => invoke<[number, number]>('display_size'),
  gameLaunch: (bits?: '64' | '32', itemtest?: boolean) => invoke<LaunchInfo>('game_launch', { bits: bits ?? null, itemtest: itemtest ?? null }),
  gameRcon: (command: string) => invoke<string>('game_rcon', { command }),
  gameStatus: () => invoke<GameStatus>('game_status'),
  gameDetach: () => invoke<void>('game_detach'),
};

export function errorText(e: unknown): string {
  if (typeof e === 'string') return e;
  if (e instanceof Error) return e.message;
  return JSON.stringify(e);
}
