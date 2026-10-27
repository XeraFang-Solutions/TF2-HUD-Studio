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

/// Stock TF2 content, searched the way the game does: the VPKs and loose folders listed in `tf/gameinfo.txt`
/// (TF2's own VPKs, then Half-Life 2's and the platform VPK, then loose files in `tf/`, `hl2/` and `platform/`).
pub struct GameAssets {
    pub tf_dir: PathBuf,
    pub vpks: Vec<Vpk>,
    /// Loose folders after the VPKs, in search order (`tf/` first).
    pub loose: Vec<PathBuf>,
}

/// A search path from gameinfo.txt, resolved against the install.
#[derive(Debug, PartialEq, Eq)]
pub enum SearchPath {
    Vpk(PathBuf),
    Dir(PathBuf),
}

/// The game and platform search paths of a `gameinfo.txt`, in order. Sound VPKs, `custom/*` (HUD Studio shows one HUD at a
/// time) and the download folder are left out.
pub fn search_paths(gameinfo: &str, tf_dir: &Path) -> Vec<SearchPath> {
    let install = tf_dir.parent().unwrap_or(tf_dir);
    // gameinfo has unquoted wildcards like tf/custom/*, which the KeyValues lexer would read as a /* comment
    let tree = hud_syntax::tree(&gameinfo.replace("/*", "/<wildcard>"));
    let Some(paths) = tree.roots.iter().find_map(|r| r.find("FileSystem")).and_then(|f| f.find("SearchPaths")) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in paths.children.iter().filter(|c| c.condition.as_deref().is_none_or(hud_syntax::condition_met)) {
        let Some(value) = entry.value.as_deref() else { continue };
        let kinds = entry.key.to_ascii_lowercase();
        if !kinds.split('+').any(|k| k == "game" || k == "mod" || k == "platform" || k == "vgui") || kinds.contains("download") {
            continue;
        }
        let rel = value.replace('\\', "/");
        let lower = rel.to_ascii_lowercase();
        if lower.contains("<wildcard>") || lower.contains("sound") {
            continue;
        }
        let path = if let Some(r) = lower.strip_prefix("|gameinfo_path|") {
            tf_dir.join(&rel[rel.len() - r.len()..])
        } else if let Some(r) = lower.strip_prefix("|all_source_engine_paths|") {
            install.join(&rel[rel.len() - r.len()..])
        } else {
            install.join(&rel)
        };
        let item = if lower.ends_with(".vpk") {
            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            SearchPath::Vpk(path.with_file_name(format!("{}_dir.vpk", &name[..name.len() - 4])))
        } else {
            SearchPath::Dir(path)
        };
        if !out.contains(&item) {
            out.push(item);
        }
    }
    out
}

impl GameAssets {
    /// Opens the stock VPKs listed in gameinfo.txt (TF2's misc and texture VPKs at least).
    pub fn open(tf_dir: &Path) -> Result<GameAssets, String> {
        let gameinfo = tf_dir.join("gameinfo.txt");
        if !gameinfo.is_file() {
            return Err(format!("{} does not look like a TF2 'tf' folder (gameinfo.txt missing)", tf_dir.display()));
        }
        let install = tf_dir.parent().unwrap_or(tf_dir);
        let mut paths = fs::read(&gameinfo).ok().and_then(|b| hud_syntax::decode(&b).ok()).map(|(t, _)| search_paths(&t, tf_dir)).unwrap_or_default();
        // Older or trimmed gameinfo files: TF2's two VPKs and the HL2/platform content it always mounts.
        for p in [
            SearchPath::Vpk(tf_dir.join("tf2_misc_dir.vpk")),
            SearchPath::Vpk(tf_dir.join("tf2_textures_dir.vpk")),
            SearchPath::Vpk(install.join("hl2/hl2_textures_dir.vpk")),
            SearchPath::Vpk(install.join("hl2/hl2_misc_dir.vpk")),
            SearchPath::Vpk(install.join("platform/platform_misc_dir.vpk")),
            SearchPath::Dir(tf_dir.to_path_buf()),
            SearchPath::Dir(install.join("hl2")),
            SearchPath::Dir(install.join("platform")),
        ] {
            if !paths.contains(&p) {
                paths.push(p);
            }
        }
        let mut vpks = Vec::new();
        let mut loose: Vec<PathBuf> = Vec::new();
        for p in paths {
            match p {
                SearchPath::Vpk(path) => {
                    if let Some(found) = existing_file(&path) {
                        match Vpk::open(&found) {
                            Ok(v) => vpks.push(v),
                            // TF2's own VPKs are required; a damaged HL2/platform VPK only loses its extras
                            Err(e) if found.starts_with(tf_dir) => return Err(e),
                            Err(_) => {}
                        }
                    }
                }
                SearchPath::Dir(path) => {
                    let canon = fs::canonicalize(&path).unwrap_or(path.clone());
                    if path.is_dir() && !loose.iter().any(|l| fs::canonicalize(l).unwrap_or(l.clone()) == canon) {
                        loose.push(path);
                    }
                }
            }
        }
        if !loose.iter().any(|l| l == tf_dir) {
            loose.insert(0, tf_dir.to_path_buf());
        }
        Ok(GameAssets { tf_dir: tf_dir.to_path_buf(), vpks, loose })
    }
    fn loose_file(&self, norm: &str) -> Option<PathBuf> {
        // never other custom HUDs
        if norm.starts_with("custom/") {
            return None;
        }
        for dir in &self.loose {
            let p = dir.join(norm);
            if p.is_file() {
                return Some(p);
            }
            #[cfg(not(windows))]
            if let Some(found) = find_case_insensitive(dir, norm).filter(|p| p.is_file()) {
                return Some(found);
            }
        }
        None
    }
    pub fn read(&self, norm: &str) -> Option<(Vec<u8>, Origin)> {
        if let Some(b) = self.vpks.iter().find_map(|v| v.read(norm)) {
            return Some((b, Origin::Stock(norm.to_string())));
        }
        // loose files (fonts like tf/resource/tf2build.ttf live here); like the game, after the VPKs
        let p = self.loose_file(norm)?;
        fs::read(p).ok().map(|b| (b, Origin::Game(norm.to_string())))
    }
    pub fn exists(&self, norm: &str) -> bool {
        self.vpks.iter().any(|v| v.contains(norm)) || self.loose_file(norm).is_some()
    }
    pub fn list(&self, prefix: &str) -> Vec<String> {
        let mut v: Vec<String> = self.vpks.iter().flat_map(|p| p.list(prefix)).collect();
        v.sort();
        v.dedup();
        v
    }
}

/// `path` itself, or (outside Windows) the same path with different letter case.
fn existing_file(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path.to_path_buf());
    }
    #[cfg(not(windows))]
    {
        let parent = path.parent()?;
        let name = path.file_name()?.to_string_lossy().to_string();
        let found = fs::read_dir(parent).ok()?.flatten().find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(&name))?;
        return Some(found.path());
    }
    #[allow(unreachable_code)]
    None
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_paths_follow_tf2_gameinfo() {
        let gameinfo = r#""GameInfo"
{
	game	"Team Fortress 2"
	FileSystem
	{
		SteamAppId				440
		SearchPaths
		{
			game+mod+custom_mod	tf/custom/*
			game_lv				tf/tf2_lv.vpk
			game+mod			tf/tf2_text.vpk
			game+mod			tf/tf2_textures.vpk
			game+mod			tf/tf2_sound_vo_english.vpk
			game+mod			tf/tf2_sound_misc.vpk
			game+mod+vgui		tf/tf2_misc.vpk
			game				|all_source_engine_paths|hl2/hl2_textures.vpk
			game				|all_source_engine_paths|hl2/hl2_sound_vo_english.vpk
			game				|all_source_engine_paths|hl2/hl2_misc.vpk
			platform			|all_source_engine_paths|platform/platform_misc.vpk
			mod+mod_write+default_write_path		|gameinfo_path|.
			game+game_write		tf
			gamebin				tf/bin
			game				|all_source_engine_paths|hl2
			platform			|all_source_engine_paths|platform
			game+download	tf/download
		}
	}
}"#;
        let install = Path::new("/steam/Team Fortress 2");
        let tf = install.join("tf");
        let got = search_paths(gameinfo, &tf);
        let want = vec![
            SearchPath::Vpk(install.join("tf/tf2_text_dir.vpk")),
            SearchPath::Vpk(install.join("tf/tf2_textures_dir.vpk")),
            SearchPath::Vpk(install.join("tf/tf2_misc_dir.vpk")),
            SearchPath::Vpk(install.join("hl2/hl2_textures_dir.vpk")),
            SearchPath::Vpk(install.join("hl2/hl2_misc_dir.vpk")),
            SearchPath::Vpk(install.join("platform/platform_misc_dir.vpk")),
            SearchPath::Dir(tf.clone()), // "|gameinfo_path|." and "tf" are the same folder
            SearchPath::Dir(install.join("hl2")),
            SearchPath::Dir(install.join("platform")),
        ];
        assert_eq!(got, want);
    }
}
