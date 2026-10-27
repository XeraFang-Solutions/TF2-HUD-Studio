//! App themes: TF2's own fonts for the interface (read from the user's install, never bundled) and theme files.
use crate::{lock, Res, StoreState};
use base64::Engine;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;

/// Theme files are small JSON documents.
const MAX_THEME_BYTES: u64 = 256 * 1024;
const TF2_FONTS: [&str; 3] = ["tf2build", "tf2secondary", "tf2professor"];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiFont {
    /// "tf2build", "tf2secondary" or "tf2professor".
    id: String,
    base64: String,
}

/// `dir/name`, matching the name's letter case loosely (Linux installs can differ).
fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    let direct = dir.join(name);
    if direct.is_file() {
        return Some(direct);
    }
    fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).find(|p| p.is_file() && p.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(name)))
}

/// TF2's interface fonts from `tf/resource` (or `tf/resource/fonts`), for themes that use them.
#[tauri::command]
pub fn ui_fonts(store: State<'_, StoreState>) -> Res<Vec<UiFont>> {
    let Some(tf) = lock(&store)?.settings.tf_dir.clone().map(PathBuf::from) else { return Ok(Vec::new()) };
    let mut out = Vec::new();
    for id in TF2_FONTS {
        let file = format!("{id}.ttf");
        let found = ["resource", "resource/fonts"].iter().find_map(|d| find_file(&tf.join(d), &file));
        if let Some(bytes) = found.and_then(|p| fs::read(p).ok()) {
            out.push(UiFont { id: id.into(), base64: base64::engine::general_purpose::STANDARD.encode(bytes) });
        }
    }
    Ok(out)
}

fn is_json(path: &Path) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("json"))
}

/// Reads a theme file the user picked.
#[tauri::command]
pub fn theme_read(path: String) -> Res<String> {
    let p = PathBuf::from(&path);
    if !is_json(&p) {
        return Err("Theme files are .json files".into());
    }
    let meta = fs::metadata(&p).map_err(|e| format!("{path}: {e}"))?;
    if meta.len() > MAX_THEME_BYTES {
        return Err("That file is too big to be a theme".into());
    }
    fs::read_to_string(&p).map_err(|e| format!("{path}: {e}"))
}

/// Writes a theme file to the place the user picked in the save dialog.
#[tauri::command]
pub fn theme_write(path: String, text: String) -> Res<()> {
    let p = PathBuf::from(&path);
    if !is_json(&p) {
        return Err("Theme files are .json files".into());
    }
    if text.len() as u64 > MAX_THEME_BYTES {
        return Err("Theme is too big".into());
    }
    fs::write(&p, text).map_err(|e| format!("{path}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn theme_files_must_be_json() {
        let dir = std::env::temp_dir().join(format!("hs-theme-{}", std::process::id()));
        fs::create_dir_all(dir.join("resource")).unwrap();
        let ok = dir.join("t.json");
        theme_write(ok.to_string_lossy().into(), "{}".into()).unwrap();
        assert_eq!(theme_read(ok.to_string_lossy().into()).unwrap(), "{}");
        assert!(theme_write(dir.join("t.exe").to_string_lossy().into(), "{}".into()).is_err());
        fs::write(dir.join("resource/TF2Build.TTF"), b"x").unwrap();
        assert!(find_file(&dir.join("resource"), "tf2build.ttf").is_some());
        fs::remove_dir_all(dir).unwrap();
    }
}
