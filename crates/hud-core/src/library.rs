//! The HUD library: a folder the user keeps their HUDs in, and installing one of them into `tf/custom`.
//!
//! A HUD can be installed as a **link** (a junction on Windows, a symlink elsewhere) so the game reads the library
//! copy directly and nothing is duplicated, or as a plain **copy**. Removing an install never deletes a real folder:
//! links are unlinked and copies are moved into the backups folder.
use crate::project::{copy_dir, hud_name, safe_folder_name};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum InstallMode {
    #[default]
    Link,
    Copy,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryHud {
    pub name: String,
    pub folder: String,
    pub path: String,
    /// `tf/custom` folders that are this HUD: links pointing at it, or copies with the same folder name.
    pub installs: Vec<CustomEntry>,
    /// From `TFHUD.manifest` (or `info.vdf` when there is none).
    pub details: crate::manifest::HudDetails,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomEntry {
    pub name: String,
    pub folder: String,
    pub path: String,
    /// "link" or "copy".
    pub kind: String,
    /// Where a link points.
    pub target: Option<String>,
    /// Whether the folder looks like a HUD (other custom content — configs, sounds — is left alone).
    pub is_hud: bool,
    /// Library folder this entry belongs to, if any.
    pub library_folder: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryState {
    pub library: Option<String>,
    pub huds: Vec<LibraryHud>,
    /// Everything in `tf/custom` (folders and links), for the "installed" view.
    pub custom: Vec<CustomEntry>,
}

/// A folder that looks like a HUD.
pub fn is_hud(p: &Path) -> bool {
    crate::manifest::read(p).is_some()
        || p.join("info.vdf").is_file()
        || p.join("scripts/hudlayout.res").is_file()
        || p.join("resource/ui").is_dir()
        || p.join("resource/clientscheme.res").is_file()
}

/// Whether `p` itself is a link (symlink or junction), without following it.
pub fn is_link(p: &Path) -> bool {
    #[cfg(windows)]
    if junction::exists(p).unwrap_or(false) {
        return true;
    }
    fs::symlink_metadata(p).map(|m| m.file_type().is_symlink()).unwrap_or(false)
}

/// Where a link points, made absolute.
pub fn link_target(p: &Path) -> Option<PathBuf> {
    #[cfg(windows)]
    if junction::exists(p).unwrap_or(false) {
        return junction::get_target(p).ok().map(|t| dunce::simplified(&t).to_path_buf());
    }
    let t = fs::read_link(p).ok()?;
    let t = if t.is_absolute() { t } else { p.parent()?.join(t) };
    Some(dunce::canonicalize(&t).unwrap_or(t))
}

fn same_path(a: &Path, b: &Path) -> bool {
    let ca = dunce::canonicalize(a).unwrap_or_else(|_| a.to_path_buf());
    let cb = dunce::canonicalize(b).unwrap_or_else(|_| b.to_path_buf());
    if cfg!(windows) {
        ca.to_string_lossy().eq_ignore_ascii_case(&cb.to_string_lossy())
    } else {
        ca == cb
    }
}

fn within(child: &Path, parent: &Path) -> bool {
    let c = dunce::canonicalize(child).unwrap_or_else(|_| child.to_path_buf());
    let p = dunce::canonicalize(parent).unwrap_or_else(|_| parent.to_path_buf());
    if cfg!(windows) {
        c.to_string_lossy().to_ascii_lowercase().starts_with(&p.to_string_lossy().to_ascii_lowercase())
    } else {
        c.starts_with(&p)
    }
}

/// Lists `tf/custom`: folders, links and where links point.
pub fn custom_entries(tf_dir: &Path, library: Option<&Path>) -> Vec<CustomEntry> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(tf_dir.join("custom")) else { return out };
    for e in entries.flatten() {
        let p = e.path();
        let folder = e.file_name().to_string_lossy().to_string();
        let link = is_link(&p);
        if !link && !p.is_dir() {
            continue; // loose .vpk files and readmes
        }
        let target = if link { link_target(&p) } else { None };
        let real = target.clone().unwrap_or_else(|| p.clone());
        let library_folder = library.and_then(|lib| {
            if link {
                target.as_ref().filter(|t| within(t, lib) && !same_path(t, lib)).and_then(|t| {
                    let rel = dunce::canonicalize(t).unwrap_or(t.clone());
                    let libc = dunce::canonicalize(lib).unwrap_or(lib.to_path_buf());
                    rel.strip_prefix(&libc).ok().and_then(|r| r.components().next()).map(|c| c.as_os_str().to_string_lossy().to_string())
                })
            } else if lib.join(&folder).is_dir() {
                Some(folder.clone())
            } else {
                None
            }
        });
        out.push(CustomEntry {
            name: if real.is_dir() {
                crate::manifest::read(&real).map(|m| m.name).filter(|n| !n.trim().is_empty()).unwrap_or_else(|| hud_name(&real))
            } else {
                folder.clone()
            },
            folder,
            path: p.to_string_lossy().to_string(),
            kind: if link { "link".into() } else { "copy".into() },
            target: target.map(|t| t.to_string_lossy().to_string()),
            is_hud: real.is_dir() && is_hud(&real),
            library_folder,
        });
    }
    out.sort_by_key(|e| e.folder.to_ascii_lowercase());
    out
}

pub fn state(library: Option<&Path>, tf_dir: Option<&Path>) -> LibraryState {
    let custom = tf_dir.map(|t| custom_entries(t, library)).unwrap_or_default();
    let mut huds = Vec::new();
    if let Some(lib) = library {
        if let Ok(entries) = fs::read_dir(lib) {
            for e in entries.flatten() {
                let p = e.path();
                let folder = e.file_name().to_string_lossy().to_string();
                if folder.starts_with('.') || !p.is_dir() || !is_hud(&p) {
                    continue;
                }
                let installs = custom.iter().filter(|c| c.library_folder.as_deref() == Some(folder.as_str())).cloned().collect();
                let details = crate::manifest::details(&p);
                huds.push(LibraryHud { name: details.manifest.name.clone(), folder, path: p.to_string_lossy().to_string(), installs, details });
            }
        }
    }
    huds.sort_by_key(|h| h.name.to_ascii_lowercase());
    LibraryState { library: library.map(|l| l.to_string_lossy().to_string()), huds, custom }
}

fn create_link(target: &Path, link: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        // Junctions need no admin rights or Developer Mode, unlike directory symlinks.
        junction::create(target, link)
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }
}

/// Removes a link without touching what it points to.
fn remove_link(link: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        if junction::exists(link).unwrap_or(false) {
            junction::delete(link)?;
            return fs::remove_dir(link);
        }
        // A directory symlink (mklink /D) is removed with remove_dir; a file symlink with remove_file.
        fs::remove_dir(link).or_else(|_| fs::remove_file(link))
    }
    #[cfg(unix)]
    {
        fs::remove_file(link)
    }
}

/// Moves a real folder aside into `backup_dir` (never deletes it).
fn move_aside(path: &Path, backup_dir: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(backup_dir).map_err(|e| e.to_string())?;
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "hud".into());
    let dest = backup_dir.join(format!("{name}-{stamp}"));
    if fs::rename(path, &dest).is_err() {
        copy_dir(path, &dest)?;
        fs::remove_dir_all(path).map_err(|e| format!("Copied {} to backups but could not remove it: {e}", path.display()))?;
    }
    Ok(dest)
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub target: String,
    pub mode: InstallMode,
    pub files: usize,
    /// Real folders that were moved into backups to make room.
    pub backed_up: Vec<String>,
    /// Links that were removed.
    pub unlinked: Vec<String>,
}

/// Removes whatever is at `tf/custom/<folder>`: links are unlinked, real folders are moved to backups.
pub fn uninstall(tf_dir: &Path, folder: &str, backup_dir: &Path) -> Result<InstallResult, String> {
    let custom = tf_dir.join("custom");
    let target = custom.join(folder);
    if folder.is_empty() || folder.contains(['/', '\\', ':']) || folder == "." || folder == ".." {
        return Err("Not a tf/custom folder".into());
    }
    let mut r = InstallResult { target: target.to_string_lossy().to_string(), mode: InstallMode::Copy, files: 0, backed_up: vec![], unlinked: vec![] };
    if is_link(&target) {
        remove_link(&target).map_err(|e| format!("Could not remove the link {}: {e}", target.display()))?;
        r.unlinked.push(folder.to_string());
    } else if target.exists() {
        r.backed_up.push(move_aside(&target, backup_dir)?.to_string_lossy().to_string());
    }
    Ok(r)
}

/// Installs `hud` as `tf/custom/<folder>`, replacing whatever is there.
pub fn install(hud: &Path, tf_dir: &Path, folder: &str, mode: InstallMode, backup_dir: &Path) -> Result<InstallResult, String> {
    let custom = tf_dir.join("custom");
    fs::create_dir_all(&custom).map_err(|e| e.to_string())?;
    let folder = safe_folder_name(folder);
    let target = custom.join(&folder);
    let hud = dunce::canonicalize(hud).map_err(|e| format!("{}: {e}", hud.display()))?;
    if is_link(&target) && link_target(&target).is_some_and(|t| same_path(&t, &hud)) && mode == InstallMode::Link {
        return Ok(InstallResult { target: target.to_string_lossy().to_string(), mode, files: 0, backed_up: vec![], unlinked: vec![] });
    }
    if !is_link(&target) && target.exists() && same_path(&target, &hud) {
        return Err("This HUD already lives in tf/custom, so TF2 loads it directly.".into());
    }
    if within(&hud, &target) || (!is_link(&target) && within(&target, &hud)) {
        return Err("The install folder overlaps the HUD folder".into());
    }
    let mut r = uninstall(tf_dir, &folder, backup_dir)?;
    r.mode = mode;
    match mode {
        InstallMode::Link => create_link(&hud, &target)
            .map_err(|e| format!("Could not link {} → {}: {e}. Try installing as a copy instead (Settings → HUD library).", target.display(), hud.display()))?,
        InstallMode::Copy => r.files = copy_dir(&hud, &target)?,
    }
    Ok(r)
}

/// What swapping to `hud` would remove: every other HUD in `tf/custom` (links into the library and HUD folders).
/// Non-HUD custom content (configs, hitsounds, …) is never touched.
pub fn swap_plan(library: &Path, hud_folder: &str, tf_dir: &Path) -> Vec<CustomEntry> {
    custom_entries(tf_dir, Some(library)).into_iter().filter(|c| c.is_hud && c.library_folder.as_deref() != Some(hud_folder)).collect()
}

/// Makes `hud_folder` the installed HUD: removes the HUDs listed by [`swap_plan`], then installs it.
pub fn swap(library: &Path, hud_folder: &str, tf_dir: &Path, mode: InstallMode, backup_dir: &Path) -> Result<InstallResult, String> {
    let hud = library.join(hud_folder);
    if !hud.is_dir() {
        return Err(format!("{hud_folder} is not in the HUD library"));
    }
    let mut removed = InstallResult { target: String::new(), mode, files: 0, backed_up: vec![], unlinked: vec![] };
    for c in swap_plan(library, hud_folder, tf_dir) {
        let r = uninstall(tf_dir, &c.folder, backup_dir)?;
        removed.backed_up.extend(r.backed_up);
        removed.unlinked.extend(r.unlinked);
    }
    let mut r = install(&hud, tf_dir, hud_folder, mode, backup_dir)?;
    r.backed_up.extend(removed.backed_up);
    r.unlinked.extend(removed.unlinked);
    Ok(r)
}

/// Moves a HUD folder out of `tf/custom` into the library, then links it back so the game still loads it.
pub fn adopt(tf_dir: &Path, folder: &str, library: &Path, mode: InstallMode) -> Result<InstallResult, String> {
    let src = tf_dir.join("custom").join(folder);
    if is_link(&src) {
        return Err("That's already a link; nothing to move.".into());
    }
    if !src.is_dir() {
        return Err(format!("{folder} is not a folder in tf/custom"));
    }
    fs::create_dir_all(library).map_err(|e| e.to_string())?;
    let dest = library.join(safe_folder_name(folder));
    if dest.exists() {
        return Err(format!("The library already has a folder named {}", dest.display()));
    }
    if within(library, &src) {
        return Err("The library folder is inside that HUD".into());
    }
    let mut files = 0;
    if fs::rename(&src, &dest).is_err() {
        files = copy_dir(&src, &dest)?;
        fs::remove_dir_all(&src).map_err(|e| format!("Copied to the library but could not remove the original: {e}"))?;
    }
    let mut r = InstallResult { target: src.to_string_lossy().to_string(), mode, files, backed_up: vec![], unlinked: vec![] };
    match mode {
        InstallMode::Link => create_link(&dest, &src).map_err(|e| format!("Moved to the library, but linking it back failed: {e}"))?,
        InstallMode::Copy => r.files = copy_dir(&dest, &src)?,
    }
    Ok(r)
}

/// Copies a HUD folder (a download, an extracted zip) into the library.
pub fn add(src: &Path, library: &Path) -> Result<String, String> {
    let mut src = src.to_path_buf();
    // Zips often contain one top-level folder holding the HUD.
    if !is_hud(&src) {
        let inner: Vec<PathBuf> = fs::read_dir(&src).map_err(|e| e.to_string())?.flatten().map(|e| e.path()).filter(|p| p.is_dir() && is_hud(p)).collect();
        if inner.len() == 1 {
            src = inner[0].clone();
        } else {
            return Err("That folder doesn't look like a HUD (no info.vdf, scripts/hudlayout.res or resource/ui).".into());
        }
    }
    if within(library, &src) || within(&src, library) {
        return Err("That folder is already in (or contains) the library".into());
    }
    let name = safe_folder_name(&src.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "hud".into()));
    let mut dest = library.join(&name);
    let mut i = 2;
    while dest.exists() {
        dest = library.join(format!("{name}-{i}"));
        i += 1;
    }
    copy_dir(&src, &dest)?;
    Ok(dest.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("hudlib-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }
    fn mk_hud(dir: &Path, name: &str) {
        fs::create_dir_all(dir.join("resource/ui")).unwrap();
        fs::write(dir.join("info.vdf"), format!("\"{name}\"\n{{\n\t\"ui_version\"\t\"3\"\n}}\n")).unwrap();
        fs::write(dir.join("resource/ui/hudplayerhealth.res"), "x").unwrap();
    }

    #[test]
    fn link_swap_adopt_and_uninstall() {
        let root = tmp("swap");
        let lib = root.join("library");
        let tf = root.join("tf");
        let backups = root.join("backups");
        mk_hud(&lib.join("alpha"), "Alpha");
        mk_hud(&lib.join("beta"), "Beta");
        mk_hud(&tf.join("custom/old"), "Old"); // a real HUD folder in custom
        fs::create_dir_all(tf.join("custom/my-configs/cfg")).unwrap(); // not a HUD: must survive swaps

        let r = install(&lib.join("alpha"), &tf, "alpha", InstallMode::Link, &backups).unwrap();
        assert!(r.backed_up.is_empty());
        assert!(is_link(&tf.join("custom/alpha")));
        assert!(tf.join("custom/alpha/info.vdf").is_file(), "the link resolves");
        let s = state(Some(&lib), Some(&tf));
        assert_eq!(s.huds.iter().find(|h| h.folder == "alpha").unwrap().installs.len(), 1);

        // swapping to beta removes alpha's link and moves the real "old" HUD aside, but leaves configs alone
        let plan: Vec<String> = swap_plan(&lib, "beta", &tf).into_iter().map(|c| c.folder).collect();
        assert_eq!(plan, vec!["alpha", "old"]);
        let r = swap(&lib, "beta", &tf, InstallMode::Link, &backups).unwrap();
        assert_eq!(r.unlinked, vec!["alpha"]);
        assert_eq!(r.backed_up.len(), 1);
        assert!(!tf.join("custom/alpha").exists());
        assert!(lib.join("alpha/info.vdf").is_file(), "unlinking never touches the library copy");
        assert!(tf.join("custom/my-configs/cfg").is_dir());
        assert!(is_link(&tf.join("custom/beta")));

        // copy mode, then reinstall as link over the copy (the copy goes to backups)
        install(&lib.join("alpha"), &tf, "alpha", InstallMode::Copy, &backups).unwrap();
        assert!(!is_link(&tf.join("custom/alpha")) && tf.join("custom/alpha/info.vdf").is_file());
        let r = install(&lib.join("alpha"), &tf, "alpha", InstallMode::Link, &backups).unwrap();
        assert_eq!(r.backed_up.len(), 1);

        // adopt a HUD that lives in custom into the library
        mk_hud(&tf.join("custom/gamma"), "Gamma");
        adopt(&tf, "gamma", &lib, InstallMode::Link).unwrap();
        assert!(lib.join("gamma/info.vdf").is_file());
        assert!(is_link(&tf.join("custom/gamma")));

        uninstall(&tf, "gamma", &backups).unwrap();
        assert!(!tf.join("custom/gamma").exists());
        assert!(lib.join("gamma/info.vdf").is_file());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn add_unwraps_single_folder() {
        let root = tmp("add");
        let lib = root.join("library");
        fs::create_dir_all(&lib).unwrap();
        mk_hud(&root.join("download/somehud-master"), "Some");
        let p = add(&root.join("download"), &lib).unwrap();
        assert!(p.ends_with("somehud-master"));
        assert!(Path::new(&p).join("info.vdf").is_file());
        let _ = fs::remove_dir_all(&root);
    }
}
