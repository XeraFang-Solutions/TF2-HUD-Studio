//! HUD library commands (list, install as link/copy, swap, adopt, add) and `TFHUD.manifest` editing.
use crate::session::SessionState;
use crate::{backup_root, StoreState};
use hud_core::library::{self, InstallMode, InstallResult, LibraryState};
use hud_core::manifest::{self, HudDetails, Manifest};
use std::path::{Path, PathBuf};
use tauri::State;

type Res<T> = Result<T, String>;

fn lock<T>(m: &std::sync::Mutex<T>) -> Res<std::sync::MutexGuard<'_, T>> {
    m.lock().map_err(|_| "Internal state poisoned; restart HUD Studio".to_string())
}

fn paths(store: &State<'_, StoreState>) -> Res<(Option<PathBuf>, Option<PathBuf>, InstallMode)> {
    let st = lock(store)?;
    Ok((st.settings.hud_library.clone().map(PathBuf::from), st.settings.tf_dir.clone().map(PathBuf::from), st.settings.install_mode))
}

fn need<T>(v: Option<T>, what: &str) -> Res<T> {
    v.ok_or_else(|| format!("Set your {what} in Settings first"))
}

/// A library folder name (no paths).
fn folder_name(s: &str) -> Res<&str> {
    if s.is_empty() || s.contains(['/', '\\', ':']) || s == "." || s == ".." {
        return Err("Not a library folder".into());
    }
    Ok(s)
}

#[tauri::command]
pub async fn library_get(store: State<'_, StoreState>) -> Res<LibraryState> {
    let (lib, tf, _) = paths(&store)?;
    Ok(library::state(lib.as_deref(), tf.as_deref()))
}

/// Sets the library folder (creating it if needed).
#[tauri::command]
pub async fn library_set(path: Option<String>, store: State<'_, StoreState>) -> Res<LibraryState> {
    let path = match path.filter(|p| !p.trim().is_empty()) {
        Some(p) => {
            std::fs::create_dir_all(&p).map_err(|e| format!("{p}: {e}"))?;
            Some(dunce::canonicalize(&p).map_err(|e| e.to_string())?.to_string_lossy().to_string())
        }
        None => None,
    };
    let tf = {
        let mut st = lock(&store)?;
        st.settings.hud_library = path.clone();
        st.save()?;
        st.settings.tf_dir.clone().map(PathBuf::from)
    };
    if let (Some(p), Some(t)) = (&path, &tf) {
        if Path::new(p).starts_with(t.join("custom")) {
            return Err("Keep the library outside tf/custom — TF2 would load every HUD in it at once.".into());
        }
    }
    Ok(library::state(path.as_deref().map(Path::new), tf.as_deref()))
}

/// Installs a library HUD (or, with `path`, any HUD folder such as the open project) into tf/custom.
#[tauri::command]
pub async fn library_install(
    app: tauri::AppHandle,
    folder: Option<String>,
    path: Option<String>,
    name: Option<String>,
    mode: Option<InstallMode>,
    store: State<'_, StoreState>,
) -> Res<InstallResult> {
    let (lib, tf, default_mode) = paths(&store)?;
    let tf = need(tf, "TF2 folder")?;
    let hud = match (folder, path) {
        (Some(f), _) => need(lib, "HUD library folder")?.join(folder_name(&f)?),
        (None, Some(p)) => PathBuf::from(p),
        _ => return Err("Nothing to install".into()),
    };
    let name = name.unwrap_or_else(|| hud.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
    library::install(&hud, &tf, &name, mode.unwrap_or(default_mode), &backup_root(&app).join("_custom"))
}

/// What swapping to a library HUD would remove from tf/custom.
#[tauri::command]
pub async fn library_swap_plan(folder: String, store: State<'_, StoreState>) -> Res<Vec<library::CustomEntry>> {
    let (lib, tf, _) = paths(&store)?;
    Ok(library::swap_plan(&need(lib, "HUD library folder")?, folder_name(&folder)?, &need(tf, "TF2 folder")?))
}

#[tauri::command]
pub async fn library_swap(app: tauri::AppHandle, folder: String, mode: Option<InstallMode>, store: State<'_, StoreState>) -> Res<InstallResult> {
    let (lib, tf, default_mode) = paths(&store)?;
    library::swap(
        &need(lib, "HUD library folder")?,
        folder_name(&folder)?,
        &need(tf, "TF2 folder")?,
        mode.unwrap_or(default_mode),
        &backup_root(&app).join("_custom"),
    )
}

/// Removes a tf/custom entry: links are unlinked, folders are moved into backups.
#[tauri::command]
pub async fn library_uninstall(app: tauri::AppHandle, custom_folder: String, store: State<'_, StoreState>) -> Res<InstallResult> {
    let (_, tf, _) = paths(&store)?;
    library::uninstall(&need(tf, "TF2 folder")?, folder_name(&custom_folder)?, &backup_root(&app).join("_custom"))
}

/// Moves a HUD out of tf/custom into the library and links it back.
#[tauri::command]
pub async fn library_adopt(
    custom_folder: String,
    mode: Option<InstallMode>,
    store: State<'_, StoreState>,
    session: State<'_, SessionState>,
) -> Res<LibraryState> {
    let (lib, tf, default_mode) = paths(&store)?;
    let (lib, tf) = (need(lib, "HUD library folder")?, need(tf, "TF2 folder")?);
    let src = tf.join("custom").join(folder_name(&custom_folder)?);
    if let Ok(root) = lock(&session)?.root() {
        if dunce::canonicalize(&src).ok().as_ref() == Some(root) {
            return Err("That HUD is open in HUD Studio. Close it (open another HUD) before moving it.".into());
        }
    }
    library::adopt(&tf, &custom_folder, &lib, mode.unwrap_or(default_mode))?;
    Ok(library::state(Some(&lib), Some(&tf)))
}

/// Copies a HUD folder (a download, an extracted zip) into the library.
#[tauri::command]
pub async fn library_add(path: String, store: State<'_, StoreState>) -> Res<String> {
    let (lib, _, _) = paths(&store)?;
    library::add(Path::new(&path), &need(lib, "HUD library folder")?)
}

// ───────────────────────────── TFHUD.manifest ─────────────────────────────

/// `path` is a HUD folder; when omitted, the open project.
fn hud_dir(path: Option<String>, session: &State<'_, SessionState>) -> Res<PathBuf> {
    match path {
        Some(p) => {
            let p = PathBuf::from(p);
            if !p.is_dir() {
                return Err(format!("{} is not a folder", p.display()));
            }
            Ok(p)
        }
        None => Ok(lock(session)?.root()?.clone()),
    }
}

#[tauri::command]
pub async fn manifest_get(path: Option<String>, session: State<'_, SessionState>) -> Res<HudDetails> {
    Ok(manifest::details(&hud_dir(path, &session)?))
}

#[tauri::command]
pub async fn manifest_save(path: Option<String>, manifest: Manifest, session: State<'_, SessionState>) -> Res<HudDetails> {
    let dir = hud_dir(path, &session)?;
    manifest::write(&dir, &manifest)?;
    refresh_if_project(&dir, &session);
    Ok(manifest::details(&dir))
}

/// Copies an image into the HUD as its icon or banner and records it in the manifest.
#[tauri::command]
pub async fn manifest_set_image(path: Option<String>, slot: String, image: String, session: State<'_, SessionState>) -> Res<HudDetails> {
    let dir = hud_dir(path, &session)?;
    let rel = manifest::import_image(&dir, &slot, Path::new(&image))?;
    let mut m = manifest::read(&dir).unwrap_or_else(|| manifest::details(&dir).manifest);
    if slot == "icon" {
        m.icon = rel;
    } else {
        m.banner = rel;
    }
    manifest::write(&dir, &m)?;
    refresh_if_project(&dir, &session);
    Ok(manifest::details(&dir))
}

fn refresh_if_project(dir: &Path, session: &State<'_, SessionState>) {
    if let Ok(mut s) = session.lock() {
        let Ok(root) = s.root().cloned() else { return };
        if dunce::canonicalize(dir).ok().as_ref() == Some(&root) {
            if let Ok(files) = hud_core::project::scan(&root) {
                s.set_files(files);
            }
        }
    }
}
