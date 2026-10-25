mod album;
mod editors;
mod game;
mod hudlib;
mod imagetools;
mod marketplace;
mod session;
mod settings;
#[cfg(any(not(windows), test))]
mod steam;
mod tools;

use hud_core::{anim, images, lint, project, resolve, scheme, vfs};
use serde::{Deserialize, Serialize};
use session::{Document, Session, SessionState};
use settings::{Settings, Store};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};

type Res<T> = Result<T, String>;
pub type StoreState = Mutex<Store>;

fn lock<T>(m: &Mutex<T>) -> Res<std::sync::MutexGuard<'_, T>> {
    m.lock().map_err(|_| "Internal state poisoned; restart HUD Studio".to_string())
}

// ───────────────────────────── settings / game ─────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    version: String,
    settings: Settings,
    installs: Vec<project::GameInstall>,
    backup_dir: String,
    /// HUD folder passed on the command line (`tf2-hud-studio <folder>`), opened at startup.
    launch_path: Option<String>,
}

#[tauri::command]
async fn app_init(app: tauri::AppHandle, store: State<'_, StoreState>) -> Res<AppInfo> {
    let installs = project::discover();
    let mut st = lock(&store)?;
    if st.settings.tf_dir.is_none() {
        if let Some(first) = installs.first() {
            st.settings.tf_dir = Some(first.tf_dir.clone());
            let _ = st.save();
        }
    }
    let launch_path = std::env::args().skip(1).find(|a| !a.starts_with('-') && Path::new(a).is_dir());
    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        settings: st.settings.clone(),
        installs,
        backup_dir: backup_root(&app).to_string_lossy().to_string(),
        launch_path,
    })
}

#[tauri::command]
fn settings_save(settings: Settings, state: State<'_, SessionState>, store: State<'_, StoreState>) -> Res<Settings> {
    hud_syntax::set_platform(settings.platform);
    lock(&state)?.images.clear();
    let mut st = lock(&store)?;
    st.settings = settings;
    st.save()?;
    Ok(st.settings.clone())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GameInfo {
    tf_dir: String,
    vpk_files: usize,
    huds: Vec<project::CustomHud>,
}

fn load_game(session: &mut Session, dir: &Path) -> Res<GameInfo> {
    let same = session.game_dir.as_deref() == Some(dir) && session.game.is_some();
    if !same {
        let assets = vfs::GameAssets::open(dir)?;
        session.game = Some(Arc::new(assets));
        session.game_dir = Some(dir.to_path_buf());
        session.images.clear();
        session.strings = None;
    }
    let g = session.game.as_ref().unwrap();
    Ok(GameInfo { tf_dir: dir.to_string_lossy().to_string(), vpk_files: g.vpks.iter().map(|v| v.len()).sum(), huds: project::list_custom_huds(dir) })
}

/// Sets (and remembers) the TF2 `tf` folder and loads the stock VPK indexes.
#[tauri::command]
async fn game_set_dir(path: String, state: State<'_, SessionState>, store: State<'_, StoreState>) -> Res<GameInfo> {
    let mut dir = PathBuf::from(&path);
    if !dir.join("gameinfo.txt").is_file() && dir.join("tf").join("gameinfo.txt").is_file() {
        dir = dir.join("tf");
    }
    let info = {
        let mut s = lock(&state)?;
        load_game(&mut s, &dir)?
    };
    let mut st = lock(&store)?;
    st.settings.tf_dir = Some(info.tf_dir.clone());
    st.save()?;
    Ok(info)
}

// ───────────────────────────── project ─────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectInfo {
    root: String,
    name: String,
    files: Vec<project::FileEntry>,
    game: Option<GameInfo>,
    game_error: Option<String>,
    inside_tf_custom: bool,
}

#[tauri::command]
async fn project_open(path: String, state: State<'_, SessionState>, store: State<'_, StoreState>) -> Res<ProjectInfo> {
    let root = dunce::canonicalize(PathBuf::from(&path)).map_err(|e| format!("{path}: {e}"))?;
    if !root.is_dir() {
        return Err("Not a folder".into());
    }
    let files = project::scan(&root)?;
    let configured = lock(&store)?.settings.tf_dir.clone();
    let tf_dir = project::tf_dir_of_project(&root).or_else(|| configured.map(PathBuf::from));
    let mut s = lock(&state)?;
    s.root = Some(root.clone());
    s.buffers.clear();
    s.images.clear();
    s.strings = None;
    s.set_files(files.clone());
    let (game, game_error) = match tf_dir {
        Some(dir) => match load_game(&mut s, &dir) {
            Ok(g) => (Some(g), None),
            Err(e) => (None, Some(e)),
        },
        None => (None, None),
    };
    drop(s);
    let mut st = lock(&store)?;
    st.push_recent(&root.to_string_lossy());
    if let Some(g) = &game {
        st.settings.tf_dir = Some(g.tf_dir.clone());
    }
    st.save()?;
    Ok(ProjectInfo {
        name: project::hud_name(&root),
        inside_tf_custom: project::tf_dir_of_project(&root).is_some(),
        root: root.to_string_lossy().to_string(),
        files,
        game,
        game_error,
    })
}

#[tauri::command]
async fn project_refresh(state: State<'_, SessionState>) -> Res<Vec<project::FileEntry>> {
    let mut s = lock(&state)?;
    let files = project::scan(s.root()?)?;
    s.set_files(files.clone());
    s.images.clear();
    Ok(files)
}

#[tauri::command]
async fn project_create(parent: String, name: String, template: String, state: State<'_, SessionState>, store: State<'_, StoreState>) -> Res<String> {
    let game = {
        let mut s = lock(&state)?;
        if s.game.is_none() {
            if let Some(dir) = lock(&store)?.settings.tf_dir.clone() {
                let _ = load_game(&mut s, Path::new(&dir));
            }
        }
        s.game.clone()
    };
    let root = project::scaffold(Path::new(&parent), &name, &template, game.as_deref())?;
    Ok(root.to_string_lossy().to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewFile {
    path: String,
    content: String,
}

#[tauri::command]
fn project_create_file(file: NewFile, state: State<'_, SessionState>) -> Res<Vec<project::FileEntry>> {
    let mut s = lock(&state)?;
    let target = s.project_file(&file.path)?;
    if target.exists() {
        return Err(format!("{} already exists", file.path));
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&target, file.content).map_err(|e| e.to_string())?;
    let files = project::scan(s.root()?)?;
    s.set_files(files.clone());
    Ok(files)
}

// ───────────────────────────── documents ─────────────────────────────

#[tauri::command]
fn document_open(path: String, state: State<'_, SessionState>) -> Res<Document> {
    let mut s = lock(&state)?;
    let path = s.ensure_buffer(&path)?;
    s.document(&path)
}

fn edit(state: &State<'_, SessionState>, path: &str, f: impl FnOnce(&str) -> Res<String>) -> Res<Document> {
    let mut s = lock(state)?;
    let path = s.ensure_buffer(path)?;
    let buf = s.buffers.get_mut(&path).unwrap();
    let next = f(&buf.content)?;
    if next.len() > 8_000_000 {
        return Err("Document exceeds editor size limit".into());
    }
    buf.apply(next);
    s.document(&path)
}

#[tauri::command]
fn document_set_source(path: String, source: String, state: State<'_, SessionState>) -> Res<Document> {
    edit(&state, &path, |_| Ok(source))
}

#[tauri::command]
fn document_set_property(path: String, panel_id: String, property: String, value: String, at: Option<usize>, state: State<'_, SessionState>) -> Res<Document> {
    edit(&state, &path, |src| hud_syntax::set_property(src, &panel_id, &property, &value, at))
}

#[tauri::command]
fn document_remove_property(path: String, panel_id: String, property: String, at: Option<usize>, state: State<'_, SessionState>) -> Res<Document> {
    edit(&state, &path, |src| hud_syntax::remove_property(src, &panel_id, &property, at))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BlockResult {
    document: Document,
    id: String,
}

fn edit_block(state: &State<'_, SessionState>, path: &str, f: impl FnOnce(&str) -> Res<(String, String)>) -> Res<BlockResult> {
    let mut id = String::new();
    let document = edit(state, path, |src| {
        let (out, new_id) = f(src)?;
        id = new_id;
        Ok(out)
    })?;
    Ok(BlockResult { document, id })
}

#[tauri::command]
fn document_insert_block(path: String, parent_id: String, name: String, properties: Vec<(String, String)>, state: State<'_, SessionState>) -> Res<BlockResult> {
    edit_block(&state, &path, |src| hud_syntax::insert_block(src, &parent_id, &name, &properties))
}

#[tauri::command]
fn document_duplicate_block(path: String, id: String, new_name: String, state: State<'_, SessionState>) -> Res<BlockResult> {
    edit_block(&state, &path, |src| hud_syntax::duplicate_block(src, &id, &new_name))
}

#[tauri::command]
fn document_rename_block(path: String, id: String, new_name: String, state: State<'_, SessionState>) -> Res<BlockResult> {
    edit_block(&state, &path, |src| hud_syntax::rename_block(src, &id, &new_name))
}

#[tauri::command]
fn document_remove_block(path: String, id: String, state: State<'_, SessionState>) -> Res<Document> {
    edit(&state, &path, |src| hud_syntax::remove_block(src, &id))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ElementEdit {
    /// Block names from below the file's root block down to the element, e.g. ["HudPlayerHealth"].
    element: Vec<String>,
    property: String,
    /// None removes the property from this file (reverting to the inherited value).
    value: Option<String>,
    /// Exact block holding the current value (e.g. the second of two same-named blocks); preferred when present.
    #[serde(default)]
    block_id: Option<String>,
}

/// Applies several element-level edits as one undo step. Missing blocks are created, so editing a value
/// that is inherited from a #base file (or not set at all) writes an override into this file.
fn apply_element_edits(src: &str, path: &str, edits: &[ElementEdit]) -> Res<String> {
    let mut out = src.to_owned();
    for e in edits {
        let t = hud_syntax::tree(&out);
        let root_id = match t.roots.iter().find(|r| r.is_block()) {
            Some(r) => r.id.clone(),
            None => {
                let nl = if out.contains("\r\n") { "\r\n" } else { "\n" };
                if !out.is_empty() && !out.ends_with('\n') {
                    out.push_str(nl);
                }
                out.push_str(&format!("\"{path}\"{nl}{{{nl}}}{nl}"));
                hud_syntax::tree(&out).roots.iter().find(|r| r.is_block()).map(|r| r.id.clone()).ok_or("Could not create root block")?
            }
        };
        if let Some(bid) = e.block_id.as_deref().filter(|b| !b.is_empty()) {
            if hud_syntax::find_node(&hud_syntax::tree(&out).roots, bid).is_some() {
                out = match &e.value {
                    Some(v) => hud_syntax::set_property(&out, bid, &e.property, v, None)?,
                    None => hud_syntax::remove_property(&out, bid, &e.property, None).unwrap_or(out),
                };
                continue;
            }
        }
        match &e.value {
            Some(v) => {
                let (next, id) = hud_syntax::ensure_blocks(&out, &root_id, &e.element)?;
                out = hud_syntax::set_property(&next, &id, &e.property, v, None)?;
            }
            None => {
                let mut id = root_id.clone();
                let mut found = true;
                let tree = hud_syntax::tree(&out);
                for seg in &e.element {
                    match hud_syntax::find_node(&tree.roots, &id).and_then(|n| n.children.iter().find(|c| c.is_block() && c.key.eq_ignore_ascii_case(seg))) {
                        Some(c) => id = c.id.clone(),
                        None => {
                            found = false;
                            break;
                        }
                    }
                }
                if found {
                    if let Ok(next) = hud_syntax::remove_property(&out, &id, &e.property, None) {
                        out = next;
                    }
                }
            }
        }
    }
    Ok(out)
}

#[tauri::command]
fn element_edits(path: String, edits: Vec<ElementEdit>, state: State<'_, SessionState>) -> Res<Document> {
    let p = path.clone();
    edit(&state, &path, |src| apply_element_edits(src, &p, &edits))
}

#[tauri::command]
fn document_undo(path: String, state: State<'_, SessionState>) -> Res<Document> {
    let mut s = lock(&state)?;
    let path = s.ensure_buffer(&path)?;
    let buf = s.buffers.get_mut(&path).unwrap();
    if let Some(prev) = buf.undo.pop() {
        buf.redo.push(std::mem::replace(&mut buf.content, prev));
    }
    s.document(&path)
}

#[tauri::command]
fn document_redo(path: String, state: State<'_, SessionState>) -> Res<Document> {
    let mut s = lock(&state)?;
    let path = s.ensure_buffer(&path)?;
    let buf = s.buffers.get_mut(&path).unwrap();
    if let Some(next) = buf.redo.pop() {
        buf.undo.push(std::mem::replace(&mut buf.content, next));
    }
    s.document(&path)
}

#[tauri::command]
fn document_revert(path: String, state: State<'_, SessionState>) -> Res<Document> {
    let mut s = lock(&state)?;
    let path = s.ensure_buffer(&path)?;
    let buf = s.buffers.get_mut(&path).unwrap();
    let original = buf.original.clone();
    buf.apply(original); // undoable
    s.document(&path)
}

/// Discards the buffer (and its history) and re-reads the file from disk.
#[tauri::command]
fn document_reload(path: String, state: State<'_, SessionState>) -> Res<Document> {
    let mut s = lock(&state)?;
    let path = s.canonical(&path);
    s.buffers.remove(&path);
    s.images.clear();
    let path = s.ensure_buffer(&path)?;
    s.document(&path)
}

#[tauri::command]
fn document_close(path: String, state: State<'_, SessionState>) -> Res<()> {
    let mut s = lock(&state)?;
    let path = s.canonical(&path);
    if s.buffers.get(&path).is_some_and(|b| b.content == b.original) {
        s.buffers.remove(&path);
    }
    Ok(())
}

fn utc_stamp() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()) as i64;
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    // civil-from-days (Howard Hinnant)
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}{m:02}{d:02}-{:02}{:02}{:02}", rem / 3600, rem % 3600 / 60, rem % 60)
}

pub(crate) fn backup_root(app: &tauri::AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap_or_else(|_| std::env::temp_dir().join("tf2-hud-studio")).join("backups")
}

fn prune_backups(dir: &Path, keep: usize) {
    if let Ok(entries) = fs::read_dir(dir) {
        let mut v: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
        v.sort();
        if v.len() > keep {
            for old in &v[..v.len() - keep] {
                let _ = fs::remove_dir_all(old);
            }
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveResult {
    saved: Vec<String>,
    backup: Option<String>,
    documents: Vec<Document>,
}

fn save_paths(app: &tauri::AppHandle, s: &mut Session, paths: &[String], backups: bool) -> Res<SaveResult> {
    let root = s.root()?.clone();
    let project_name = root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "hud".into());
    let snapshot = backup_root(app).join(&project_name).join(utc_stamp());
    // Preflight every file first so a conflict aborts the whole batch before anything is written.
    let mut plan = Vec::new();
    for path in paths {
        let file = s.project_file(path)?;
        let buf = s.buffers.get(path).ok_or_else(|| format!("{path} is not open"))?;
        if buf.content == buf.original {
            continue;
        }
        if file.exists() {
            let disk = fs::read(&file).map_err(|e| e.to_string())?;
            let current = hud_syntax::decode(&disk).map(|(t, _)| t).unwrap_or_default();
            if current != buf.original {
                return Err(format!("{path} changed on disk since it was opened. Nothing was saved — reload it or copy your changes first."));
            }
        }
        plan.push((path.clone(), file, hud_syntax::encode(&buf.content, buf.encoding)));
    }
    let mut backup = None;
    for (path, file, bytes) in &plan {
        if backups && file.exists() {
            let dest = snapshot.join(path);
            if let Some(p) = dest.parent() {
                fs::create_dir_all(p).map_err(|e| format!("Backup failed: {e}"))?;
            }
            fs::copy(file, &dest).map_err(|e| format!("Backup failed: {e}"))?;
            backup = Some(snapshot.to_string_lossy().to_string());
        }
        let temp = file.with_file_name(format!(".{}.hudstudio-tmp", file.file_name().unwrap_or_default().to_string_lossy()));
        fs::write(&temp, bytes).map_err(|e| format!("{path}: {e}"))?;
        if let Ok(meta) = fs::metadata(file) {
            let _ = fs::set_permissions(&temp, meta.permissions());
        }
        // rename over the original; Windows refuses to replace an existing file with rename
        if fs::rename(&temp, file).is_err() {
            fs::remove_file(file).map_err(|e| format!("{path}: {e}"))?;
            fs::rename(&temp, file).map_err(|e| format!("{path}: {e}"))?;
        }
        let buf = s.buffers.get_mut(path).unwrap();
        buf.original = buf.content.clone();
    }
    if backup.is_some() {
        prune_backups(snapshot.parent().unwrap(), 60);
    }
    let documents = paths.iter().filter_map(|p| s.document(p).ok()).collect();
    Ok(SaveResult { saved: plan.into_iter().map(|(p, _, _)| p).collect(), backup, documents })
}

#[tauri::command]
fn document_save(app: tauri::AppHandle, path: String, state: State<'_, SessionState>, store: State<'_, StoreState>) -> Res<SaveResult> {
    let backups = lock(&store)?.settings.backups;
    let mut s = lock(&state)?;
    let path = s.canonical(&path);
    let r = save_paths(&app, &mut s, &[path], backups)?;
    s.images.clear();
    Ok(r)
}

#[tauri::command]
fn document_save_all(app: tauri::AppHandle, state: State<'_, SessionState>, store: State<'_, StoreState>) -> Res<SaveResult> {
    let backups = lock(&store)?.settings.backups;
    let mut s = lock(&state)?;
    let dirty: Vec<String> = s.buffers.iter().filter(|(_, b)| b.content != b.original).map(|(p, _)| p.clone()).collect();
    let r = save_paths(&app, &mut s, &dirty, backups)?;
    s.images.clear();
    Ok(r)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BufferState {
    path: String,
    dirty: bool,
    changed_on_disk: bool,
}

/// State of every open buffer, including whether the file was modified by another program.
#[tauri::command]
fn documents_status(state: State<'_, SessionState>) -> Res<Vec<BufferState>> {
    let s = lock(&state)?;
    let mut out = Vec::new();
    for (path, buf) in &s.buffers {
        let changed = match s.project_file(path).ok().and_then(|f| fs::read(f).ok()) {
            Some(bytes) => hud_syntax::decode(&bytes).map(|(t, _)| t != buf.original).unwrap_or(true),
            None => true,
        };
        out.push(BufferState { path: path.clone(), dirty: buf.content != buf.original, changed_on_disk: changed });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

// ───────────────────────────── semantic model ─────────────────────────────

/// Text of any file TF2 would see at `path` (the HUD's copy first, then tf/ and the stock VPKs).
#[tauri::command]
async fn file_text(path: String, state: State<'_, SessionState>) -> Res<Option<(String, String)>> {
    let s = lock(&state)?;
    s.with_vfs(|v| v.read_text(&path).map(|(t, o)| (t, o.label())))
}

/// Stock HUD text files the HUD doesn't override yet (resource/, scripts/), for "customize a TF2 file".
#[tauri::command]
async fn stock_files(state: State<'_, SessionState>) -> Res<Vec<String>> {
    let s = lock(&state)?;
    let Some(g) = &s.game else { return Ok(Vec::new()) };
    let mut out: Vec<String> = ["resource/", "scripts/"]
        .iter()
        .flat_map(|p| g.list(p))
        .filter(|p| (p.ends_with(".res") || p.ends_with(".txt")) && !s.index.contains_key(p) && !p.contains("_english") && !p.contains("/closecaption"))
        .filter(|p| p.starts_with("resource/ui/") || p.starts_with("scripts/hud") || p == "resource/clientscheme.res" || p.starts_with("resource/scheme"))
        .collect();
    out.sort();
    Ok(out)
}

/// Copies a stock file (from tf/ or the VPKs) into the HUD so it can be edited.
#[tauri::command]
async fn project_copy_stock(path: String, state: State<'_, SessionState>) -> Res<Vec<project::FileEntry>> {
    let mut s = lock(&state)?;
    let norm = vfs::normalize(&path);
    if s.index.contains_key(&norm) {
        return Err(format!("{path} is already part of this HUD"));
    }
    let bytes = s.with_vfs(|v| v.read_bytes(&norm).map(|(b, _)| b))?.ok_or_else(|| format!("{path} not found in TF2's files"))?;
    let target = s.project_file(&norm)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&target, bytes).map_err(|e| e.to_string())?;
    let files = project::scan(s.root()?)?;
    s.set_files(files.clone());
    s.images.clear();
    Ok(files)
}

#[tauri::command]
async fn resolve_file(path: String, state: State<'_, SessionState>) -> Res<Option<resolve::Resolved>> {
    let s = lock(&state)?;
    s.with_vfs(|v| resolve::resolve(v, &path))
}

#[tauri::command]
async fn resolve_files(paths: Vec<String>, state: State<'_, SessionState>) -> Res<HashMap<String, Option<resolve::Resolved>>> {
    let s = lock(&state)?;
    s.with_vfs(|v| paths.iter().map(|p| (p.clone(), resolve::resolve(v, p))).collect())
}

#[tauri::command]
async fn element_files(names: Vec<String>, state: State<'_, SessionState>) -> Res<HashMap<String, Vec<String>>> {
    let s = lock(&state)?;
    s.with_vfs(|v| names.iter().map(|n| (n.clone(), resolve::element_files(v, n))).collect())
}

#[tauri::command]
async fn scheme_get(state: State<'_, SessionState>) -> Res<scheme::Scheme> {
    let s = lock(&state)?;
    s.with_vfs(scheme::load)
}

#[tauri::command]
async fn fonts_get(state: State<'_, SessionState>) -> Res<Vec<scheme::FontData>> {
    let s = lock(&state)?;
    s.with_vfs(|v| {
        let sc = scheme::load(v);
        scheme::font_data(v, &sc)
    })
}

#[tauri::command]
async fn image_get(name: String, state: State<'_, SessionState>) -> Res<images::ImageResult> {
    let key = vfs::normalize(&name);
    {
        let s = lock(&state)?;
        if let Some(cached) = s.images.get(&key) {
            return cached.clone();
        }
    }
    let result = {
        let s = lock(&state)?;
        s.with_vfs(|v| images::load(v, &name))?
    };
    lock(&state)?.images.insert(key, result.clone());
    result
}

#[tauri::command]
async fn animations_get(state: State<'_, SessionState>) -> Res<anim::Animations> {
    let s = lock(&state)?;
    s.with_vfs(anim::load)
}

#[tauri::command]
async fn problems_get(state: State<'_, SessionState>) -> Res<Vec<lint::Problem>> {
    let s = lock(&state)?;
    let files = s.file_paths();
    s.with_vfs(|v| {
        let sc = scheme::load(v);
        lint::run(v, &files, &sc)
    })
}

#[tauri::command]
async fn search(query: String, regex: bool, case_sensitive: bool, state: State<'_, SessionState>) -> Res<Vec<project::SearchHit>> {
    let s = lock(&state)?;
    let files = s.file_paths();
    s.with_vfs(|v| project::search(v, &files, &query, regex, case_sensitive))?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AssetEntry {
    path: String,
    stock: bool,
}

/// Images available to ImagePanels: the HUD's own materials plus (optionally) stock `materials/vgui`.
#[tauri::command]
async fn assets_list(include_stock: bool, state: State<'_, SessionState>) -> Res<Vec<AssetEntry>> {
    let s = lock(&state)?;
    let mut out: Vec<AssetEntry> = s
        .files
        .iter()
        .filter(|f| f.path.to_ascii_lowercase().starts_with("materials/") && f.path.to_ascii_lowercase().ends_with(".vmt"))
        .map(|f| AssetEntry { path: f.path.clone(), stock: false })
        .collect();
    if include_stock {
        if let Some(g) = &s.game {
            let own: Vec<String> = out.iter().map(|a| vfs::normalize(&a.path)).collect();
            for p in g.list("materials/vgui/").into_iter().filter(|p| p.ends_with(".vmt") && !own.contains(p)) {
                out.push(AssetEntry { path: p, stock: true });
            }
        }
    }
    Ok(out)
}

/// Looks up `#Token` strings for label previews (stock tf_english.txt, then the HUD's resource/*_english.txt).
#[tauri::command]
async fn localize(tokens: Vec<String>, state: State<'_, SessionState>) -> Res<HashMap<String, String>> {
    let mut s = lock(&state)?;
    if s.strings.is_none() {
        let mut map = HashMap::new();
        let _ = s.with_vfs(|v| {
            for file in ["resource/tf_english.txt", "resource/chat_english.txt", "resource/closecaption_english.txt"] {
                if let Some((text, _)) = v.read_text(file) {
                    for root in hud_syntax::tree(&text).roots {
                        if let Some(tokens) = root.children.iter().find(|c| c.key.eq_ignore_ascii_case("Tokens")) {
                            for t in &tokens.children {
                                if let Some(val) = &t.value {
                                    map.entry(t.key.to_ascii_lowercase()).or_insert_with(|| val.clone());
                                }
                            }
                        }
                    }
                }
            }
        });
        s.strings = Some(map);
    }
    let strings = s.strings.as_ref().unwrap();
    Ok(tokens.into_iter().filter_map(|t| strings.get(&t.trim_start_matches('#').to_ascii_lowercase()).map(|v| (t, v.clone()))).collect())
}

// ───────────────────────────── deploy / export / os ─────────────────────────────

#[tauri::command]
async fn deploy(
    app: tauri::AppHandle,
    folder: String,
    mode: Option<hud_core::library::InstallMode>,
    state: State<'_, SessionState>,
) -> Res<hud_core::library::InstallResult> {
    let (root, tf) = {
        let s = lock(&state)?;
        (s.root()?.clone(), s.game_dir.clone().ok_or("Set your TF2 folder first")?)
    };
    hud_core::library::install(&root, &tf, &folder, mode.unwrap_or_default(), &backup_root(&app).join("_deploy"))
}

#[tauri::command]
async fn export_zip(dest: String, state: State<'_, SessionState>) -> Res<usize> {
    let root = lock(&state)?.root()?.clone();
    project::export_zip(&root, Path::new(&dest))
}

/// Opens a folder (or reveals a file) in the OS file manager.
#[tauri::command]
fn open_path(path: String, state: State<'_, SessionState>) -> Res<()> {
    let s = lock(&state)?;
    let target = if Path::new(&path).is_absolute() { PathBuf::from(&path) } else { s.project_file(&path)? };
    if !target.exists() {
        return Err("Path does not exist".into());
    }
    #[cfg(windows)]
    {
        let mut c = std::process::Command::new("explorer.exe");
        if target.is_file() {
            c.arg(format!("/select,{}", target.display()));
        } else {
            c.arg(&target);
        }
        c.spawn().map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    std::process::Command::new("open")
        .arg(if target.is_file() { target.parent().unwrap().to_path_buf() } else { target })
        .spawn()
        .map_err(|e| e.to_string())?;
    #[cfg(all(unix, not(target_os = "macos")))]
    std::process::Command::new("xdg-open")
        .arg(if target.is_file() { target.parent().unwrap().to_path_buf() } else { target })
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(SessionState::default())
        .manage(game::GameState::default())
        .manage(marketplace::MarketplaceState::default())
        .setup(|app| {
            let dir = app.path().app_config_dir().unwrap_or_else(|_| std::env::temp_dir().join("tf2-hud-studio"));
            let store = Store::load(&dir);
            hud_syntax::set_platform(store.settings.platform);
            {
                let session = app.state::<SessionState>();
                let mut s = session.lock().unwrap();
                for (id, p) in &store.settings.tools {
                    s.tools.insert(id.clone(), PathBuf::from(p));
                }
            }
            app.manage(Mutex::new(store));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_init,
            settings_save,
            game_set_dir,
            project_open,
            project_refresh,
            project_create,
            project_create_file,
            document_open,
            document_set_source,
            document_set_property,
            document_remove_property,
            document_insert_block,
            document_duplicate_block,
            document_rename_block,
            document_remove_block,
            element_edits,
            document_undo,
            document_redo,
            document_revert,
            document_reload,
            document_close,
            document_save,
            document_save_all,
            documents_status,
            file_text,
            project_copy_stock,
            stock_files,
            resolve_file,
            resolve_files,
            element_files,
            scheme_get,
            fonts_get,
            image_get,
            animations_get,
            problems_get,
            search,
            assets_list,
            localize,
            deploy,
            export_zip,
            open_path,
            editors::editors_list,
            editors::editor_open,
            editors::image_editor_open,
            tools::tools_list,
            tools::tools_register,
            tools::tools_launch,
            tools::tools_open_with,
            imagetools::image_import,
            imagetools::image_png_back,
            hudlib::library_get,
            marketplace::marketplace_list,
            marketplace::marketplace_import,
            marketplace::marketplace_open,
            marketplace::marketplace_preview,
            hudlib::library_set,
            hudlib::library_install,
            hudlib::library_swap_plan,
            hudlib::library_swap,
            hudlib::library_uninstall,
            hudlib::library_adopt,
            hudlib::library_add,
            hudlib::manifest_get,
            hudlib::manifest_save,
            hudlib::manifest_set_image,
            game::display_size,
            game::game_launch,
            game::game_rcon,
            game::game_status,
            game::game_detach,
        ])
        .run(tauri::generate_context!())
        .expect("Failed to start HUD Studio");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn element_edits_create_overrides() {
        let src = "#base \"base.res\"\n\"resource/ui/x.res\"\n{\n\t\"A\"\n\t{\n\t\t\"xpos\"\t\"1\"\n\t}\n}\n";
        let out = apply_element_edits(
            src,
            "resource/ui/x.res",
            &[
                ElementEdit { element: vec!["A".into()], property: "xpos".into(), value: Some("c-10".into()), block_id: None },
                ElementEdit { element: vec!["B".into()], property: "wide".into(), value: Some("50".into()), block_id: None },
                ElementEdit { element: vec!["A".into()], property: "missing".into(), value: None, block_id: None },
            ],
        )
        .unwrap();
        let t = hud_syntax::tree(&out);
        let root = &t.roots.iter().find(|r| r.is_block()).unwrap();
        assert_eq!(root.find("A").unwrap().get("xpos"), Some("c-10"));
        assert_eq!(root.find("B").unwrap().get("wide"), Some("50"));
        assert!(t.diagnostics.is_empty());
    }
    #[test]
    fn element_edit_on_base_only_file_creates_root() {
        let out = apply_element_edits(
            "#base \"x.res\"\n",
            "resource/ui/y.res",
            &[ElementEdit { element: vec!["P".into()], property: "ypos".into(), value: Some("5".into()), block_id: None }],
        )
        .unwrap();
        assert!(out.starts_with("#base \"x.res\"\n\"resource/ui/y.res\"\n{"), "{out}");
        assert!(hud_syntax::tree(&out).diagnostics.is_empty());
    }
    #[test]
    fn edits_target_the_duplicate_block_that_holds_the_value() {
        let src = "\"r\"\n{\n\t\"A\"\n\t{\n\t\t\"xpos\"\t\"1\"\n\t}\n\t\"A\"\n\t{\n\t\t\"xpos\"\t\"9999\"\n\t}\n}\n";
        let out = apply_element_edits(
            src,
            "x.res",
            &[ElementEdit { element: vec!["A".into()], property: "xpos".into(), value: Some("5".into()), block_id: Some("r/A[2]".into()) }],
        )
        .unwrap();
        assert!(out.contains("\"1\"") && out.contains("\"5\"") && !out.contains("9999"), "{out}");
    }
    #[test]
    fn stamp_shape() {
        assert_eq!(utc_stamp().len(), 15);
    }
}
