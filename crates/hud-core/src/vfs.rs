//! Layered, case-insensitive file access the way TF2 sees a custom HUD:
//! unsaved editor buffers → the HUD folder → loose files in `tf/` → the stock VPKs.
use crate::vpk::Vpk;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Lowercase, forward slashes, `.`/`..` collapsed, no leading slash.
pub fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    let lowered = path.replace('\\', "/").to_ascii_lowercase();
    for seg in lowered.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    parts.join("/")
}

/// Joins `rel` onto the directory containing `from` (both project-relative).
pub fn relative_to(from: &str, rel: &str) -> String {
    let dir = normalize(from).rsplit_once('/').map(|(d, _)| d.to_string()).unwrap_or_default();
    if dir.is_empty() {
        normalize(rel)
    } else {
        normalize(&format!("{dir}/{rel}"))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "path")]
pub enum Origin {
    /// Project-relative path with the on-disk casing.
    Project(String),
    /// Loose file inside the game's `tf/` folder.
    Game(String),
    /// File inside a stock VPK.
    Stock(String),
}

impl Origin {
    pub fn label(&self) -> String {
        match self {
            Origin::Project(p) => p.clone(),
            Origin::Game(p) => format!("tf/{p}"),
            Origin::Stock(p) => format!("stock:{p}"),
        }
    }
    pub fn project_path(&self) -> Option<&str> {
        match self {
            Origin::Project(p) => Some(p),
            _ => None,
        }
    }
}

/// Stock TF2 content: the `tf` directory and its VPKs (opened lazily, once).
pub struct GameAssets {
    pub tf_dir: PathBuf,
    pub vpks: Vec<Vpk>,
}

impl GameAssets {
    /// Opens tf2_misc_dir.vpk (resource files, vmt) and tf2_textures_dir.vpk (vtf) when present.
    pub fn open(tf_dir: &Path) -> Result<GameAssets, String> {
        if !tf_dir.join("gameinfo.txt").is_file() {
            return Err(format!("{} does not look like a TF2 'tf' folder (gameinfo.txt missing)", tf_dir.display()));
        }
        let mut vpks = Vec::new();
        for name in ["tf2_misc_dir.vpk", "tf2_textures_dir.vpk"] {
            let p = tf_dir.join(name);
            if p.is_file() {
                vpks.push(Vpk::open(&p)?);
            }
        }
        Ok(GameAssets { tf_dir: tf_dir.to_path_buf(), vpks })
    }
    pub fn read(&self, norm: &str) -> Option<(Vec<u8>, Origin)> {
        // loose files in tf/ (fonts like tf/resource/tf2build.ttf live here) but never other custom HUDs
        if !norm.starts_with("custom/") {
            let p = self.tf_dir.join(norm);
            if let Ok(b) = fs::read(&p) {
                return Some((b, Origin::Game(norm.to_string())));
            }
            #[cfg(not(windows))]
            if let Some(found) = find_case_insensitive(&self.tf_dir, norm) {
                if let Ok(b) = fs::read(found) {
                    return Some((b, Origin::Game(norm.to_string())));
                }
            }
        }
        self.vpks.iter().find_map(|v| v.read(norm)).map(|b| (b, Origin::Stock(norm.to_string())))
    }
    pub fn exists(&self, norm: &str) -> bool {
        self.vpks.iter().any(|v| v.contains(norm)) || self.tf_dir.join(norm).is_file()
    }
    pub fn list(&self, prefix: &str) -> Vec<String> {
        let mut v: Vec<String> = self.vpks.iter().flat_map(|p| p.list(prefix)).collect();
        v.sort();
        v.dedup();
        v
    }
}

#[cfg(not(windows))]
fn find_case_insensitive(root: &Path, norm: &str) -> Option<PathBuf> {
    let mut cur = root.to_path_buf();
    for seg in norm.split('/') {
        let entry = fs::read_dir(&cur).ok()?.flatten().find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(seg))?;
        cur = entry.path();
    }
    Some(cur)
}

/// Lowercase-normalized project path → path with on-disk casing.
pub type ProjectIndex = HashMap<String, String>;

pub fn index_project(files: &[String]) -> ProjectIndex {
    files.iter().map(|f| (normalize(f), f.clone())).collect()
}

pub struct Vfs<'a> {
    pub root: &'a Path,
    pub index: &'a ProjectIndex,
    /// Unsaved editor buffers keyed by on-disk project path.
    pub overlay: &'a dyn Fn(&str) -> Option<String>,
    pub game: Option<&'a GameAssets>,
}

impl Vfs<'_> {
    pub fn project_path(&self, rel: &str) -> Option<&String> {
        self.index.get(&normalize(rel))
    }
    pub fn exists(&self, rel: &str) -> bool {
        let n = normalize(rel);
        self.index.contains_key(&n) || self.game.is_some_and(|g| g.exists(&n))
    }
    pub fn read_bytes(&self, rel: &str) -> Option<(Vec<u8>, Origin)> {
        let n = normalize(rel);
        if let Some(actual) = self.index.get(&n) {
            if let Some(text) = (self.overlay)(actual) {
                return Some((text.into_bytes(), Origin::Project(actual.clone())));
            }
            if let Ok(b) = fs::read(self.root.join(actual)) {
                return Some((b, Origin::Project(actual.clone())));
            }
        }
        self.game.and_then(|g| g.read(&n))
    }
    pub fn read_text(&self, rel: &str) -> Option<(String, Origin)> {
        let n = normalize(rel);
        if let Some(actual) = self.index.get(&n) {
            if let Some(text) = (self.overlay)(actual) {
                return Some((text, Origin::Project(actual.clone())));
            }
        }
        let (bytes, origin) = self.read_bytes(rel)?;
        hud_syntax::decode(&bytes).ok().map(|(t, _)| (t, origin))
    }
}
