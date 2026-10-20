//! Project-level operations: scanning, search, new-HUD scaffolding, deployment, zip export, Steam discovery.
use crate::resolve::resolve;
use crate::vfs::{normalize, GameAssets, Vfs};
use regex::RegexBuilder;
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub modified: u64,
}

pub const TEXT_EXTENSIONS: [&str; 7] = ["res", "vdf", "txt", "cfg", "rc", "vmt", "md"];

fn skip_dir(name: &str) -> bool {
    name.starts_with('.') || name.eq_ignore_ascii_case("node_modules")
}

pub fn scan(root: &Path) -> Result<Vec<FileEntry>, String> {
    fn walk(root: &Path, dir: &Path, files: &mut Vec<FileEntry>, depth: usize) -> Result<(), String> {
        if depth > 18 || files.len() > 20000 {
            return Err("Project exceeds safe scan limits (too deep or more than 20,000 files) — is this really a HUD folder?".into());
        }
        let mut entries: Vec<_> = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let ty = entry.file_type().map_err(|e| e.to_string())?;
            if ty.is_symlink() {
                continue;
            }
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if ty.is_dir() {
                if !skip_dir(&name) {
                    walk(root, &path, files, depth + 1)?
                }
            } else if ty.is_file() {
                let rel = path.strip_prefix(root).map_err(|e| e.to_string())?.to_string_lossy().replace('\\', "/");
                let meta = entry.metadata().map_err(|e| e.to_string())?;
                let modified = meta.modified().ok().and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_millis() as u64);
                files.push(FileEntry { path: rel, size: meta.len(), modified });
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    walk(root, root, &mut files, 0)?;
    Ok(files)
}

/// HUD display name from info.vdf (root key), else the folder name.
pub fn hud_name(root: &Path) -> String {
    if let Ok(bytes) = fs::read(root.join("info.vdf")) {
        if let Ok((text, _)) = hud_syntax::decode(&bytes) {
            if let Some(r) = hud_syntax::tree(&text).roots.into_iter().find(|r| r.is_block()) {
                return r.key;
            }
        }
    }
    root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "HUD".into())
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub offset: usize,
    pub length: usize,
    pub preview: String,
}

pub fn search(vfs: &Vfs, files: &[String], query: &str, regex: bool, case_sensitive: bool) -> Result<Vec<SearchHit>, String> {
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let pattern = if regex { query.to_string() } else { regex::escape(query) };
    let re = RegexBuilder::new(&pattern).case_insensitive(!case_sensitive).size_limit(1 << 20).build().map_err(|e| e.to_string())?;
    let mut hits = Vec::new();
    for f in files {
        let ext = f.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        if !TEXT_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }
        let Some((text, _)) = vfs.read_text(f) else { continue };
        for m in re.find_iter(&text) {
            let line_start = text[..m.start()].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let line_end = text[m.start()..].find('\n').map(|i| m.start() + i).unwrap_or(text.len());
            hits.push(SearchHit {
                file: f.clone(),
                line: text[..m.start()].matches('\n').count() + 1,
                column: text[line_start..m.start()].chars().count() + 1,
                offset: m.start(),
                length: m.len(),
                preview: text[line_start..line_end].trim_end_matches('\r').chars().take(240).collect(),
            });
            if hits.len() >= 2000 {
                return Ok(hits);
            }
        }
    }
    Ok(hits)
}

fn write_new(path: &Path, content: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, content).map_err(|e| format!("{}: {e}", path.display()))
}

fn safe_folder_name(name: &str) -> String {
    let s: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c.to_ascii_lowercase() } else { '-' }).collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() {
        "my-hud".into()
    } else {
        s
    }
}

/// Creates a new HUD folder. `template` is "empty" or "stock" (copies TF2's default HUD files from the VPKs).
pub fn scaffold(parent: &Path, name: &str, template: &str, game: Option<&GameAssets>) -> Result<PathBuf, String> {
    let folder = safe_folder_name(name);
    let root = parent.join(&folder);
    if root.exists() {
        return Err(format!("{} already exists", root.display()));
    }
    let anim_file = format!("scripts/hudanimations_{folder}.txt");
    write_new(&root.join("info.vdf"), format!("\"{}\"\n{{\n\t\"ui_version\"\t\"3\"\n}}\n", name.replace('"', "'")).as_bytes())?;
    write_new(
        &root.join("scripts/hudanimations_manifest.txt"),
        format!("hudanimations_manifest\n{{\n\t\"file\"\t\t\"{anim_file}\"\n\t\"file\"\t\t\"scripts/hudanimations.txt\"\n\t\"file\"\t\t\"scripts/hudanimations_tf.txt\"\n}}\n").as_bytes(),
    )?;
    write_new(&root.join(&anim_file), b"// Events here override the stock ones because this file is listed first in the manifest.\n\n")?;
    // A custom hudlayout.res or clientscheme.res replaces TF2's whole file, so an "empty" HUD must not ship
    // partial copies of them. The editor copies the stock file in the first time one is edited.
    if template == "stock" {
        let game = game.ok_or("Set your TF2 folder first: the default-HUD template copies files from TF2's VPKs")?;
        let empty = Default::default();
        let overlay = |_: &str| None;
        let vfs = Vfs { root: &root, index: &empty, overlay: &overlay, game: Some(game) };
        let wanted = [
            "resource/clientscheme.res",
            "scripts/hudlayout.res",
            "resource/ui/hudplayerhealth.res",
            "resource/ui/hudammoweapons.res",
            "resource/ui/hudplayerclass.res",
            "resource/ui/hudmediccharge.res",
            "resource/ui/huddamageaccount.res",
            "resource/ui/targetid.res",
        ];
        for w in wanted {
            let Some(resolved) = resolve(&vfs, w) else { continue };
            for chained in resolved.chain {
                let rel = chained.trim_start_matches("stock:").trim_start_matches("tf/").to_string();
                let target = root.join(&rel);
                if target.exists() {
                    continue;
                }
                if let Some((bytes, _)) = vfs.read_bytes(&rel) {
                    write_new(&target, &bytes)?;
                }
            }
        }
    }
    Ok(root)
}

fn copy_dir(src: &Path, dst: &Path) -> Result<usize, String> {
    let mut n = 0;
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        if ty.is_symlink() || skip_dir(&name) {
            continue;
        }
        if ty.is_dir() {
            n += copy_dir(&entry.path(), &dst.join(&name))?;
        } else {
            fs::copy(entry.path(), dst.join(&name)).map_err(|e| e.to_string())?;
            n += 1;
        }
    }
    Ok(n)
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployResult {
    pub target: String,
    pub files: usize,
    pub previous_backed_up_to: Option<String>,
}

/// Copies the project into `tf/custom/<folder>`, moving any existing copy into `backup_dir` first.
pub fn deploy(project: &Path, tf_dir: &Path, folder: &str, backup_dir: &Path) -> Result<DeployResult, String> {
    let custom = tf_dir.join("custom");
    let target = custom.join(safe_folder_name(folder));
    let p = dunce::canonicalize(project).map_err(|e| e.to_string())?;
    if target.exists() && dunce::canonicalize(&target).map_err(|e| e.to_string())? == p {
        return Err("This HUD already lives in tf/custom — your saved changes are what TF2 loads. Run 'hud_reloadscheme' in the console to see them.".into());
    }
    if p.starts_with(&target) || target.starts_with(&p) {
        return Err("Deploy target overlaps the project folder".into());
    }
    let mut backed_up = None;
    if target.exists() {
        fs::create_dir_all(backup_dir).map_err(|e| e.to_string())?;
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let dest = backup_dir.join(format!("{}-{stamp}", safe_folder_name(folder)));
        if fs::rename(&target, &dest).is_err() {
            copy_dir(&target, &dest)?;
            fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
        }
        backed_up = Some(dest.to_string_lossy().to_string());
    }
    let files = copy_dir(&p, &target)?;
    Ok(DeployResult { target: target.to_string_lossy().to_string(), files, previous_backed_up_to: backed_up })
}

pub fn export_zip(project: &Path, dest: &Path) -> Result<usize, String> {
    let file = fs::File::create(dest).map_err(|e| format!("{}: {e}", dest.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let folder = safe_folder_name(&project.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
    let mut n = 0;
    for f in scan(project)? {
        zip.start_file(format!("{folder}/{}", f.path), opts).map_err(|e| e.to_string())?;
        zip.write_all(&fs::read(project.join(&f.path)).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        n += 1;
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(n)
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomHud {
    pub name: String,
    pub folder: String,
    pub path: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameInstall {
    pub tf_dir: String,
    pub huds: Vec<CustomHud>,
}

fn steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    #[cfg(windows)]
    {
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
        use winreg::RegKey;
        if let Ok(k) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Software\\Valve\\Steam") {
            if let Ok(p) = k.get_value::<String, _>("SteamPath") {
                roots.push(PathBuf::from(p.replace('/', "\\")));
            }
        }
        for key in ["SOFTWARE\\WOW6432Node\\Valve\\Steam", "SOFTWARE\\Valve\\Steam"] {
            if let Ok(k) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(key) {
                if let Ok(p) = k.get_value::<String, _>("InstallPath") {
                    roots.push(PathBuf::from(p));
                }
            }
        }
        for env in ["ProgramFiles(x86)", "ProgramFiles"] {
            if let Some(d) = std::env::var_os(env) {
                roots.push(PathBuf::from(d).join("Steam"));
            }
        }
    }
    #[cfg(not(windows))]
    if let Some(home) = std::env::var_os("HOME") {
        let h = PathBuf::from(home);
        roots.push(h.join(".steam/steam"));
        roots.push(h.join(".local/share/Steam"));
        roots.push(h.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"));
        roots.push(h.join("Library/Application Support/Steam"));
    }
    roots
}

pub fn list_custom_huds(tf_dir: &Path) -> Vec<CustomHud> {
    let mut huds = Vec::new();
    if let Ok(entries) = fs::read_dir(tf_dir.join("custom")) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() && (p.join("info.vdf").is_file() || p.join("scripts/hudlayout.res").is_file() || p.join("resource/ui").is_dir()) {
                huds.push(CustomHud { name: hud_name(&p), folder: e.file_name().to_string_lossy().to_string(), path: p.to_string_lossy().to_string() });
            }
        }
    }
    huds.sort_by_key(|a| a.folder.to_ascii_lowercase());
    huds
}

/// Finds TF2 installs through Steam's library list.
pub fn discover() -> Vec<GameInstall> {
    let mut libraries: Vec<PathBuf> = Vec::new();
    for root in steam_roots() {
        if !root.is_dir() {
            continue;
        }
        libraries.push(root.clone());
        for vdf in [root.join("steamapps/libraryfolders.vdf"), root.join("config/libraryfolders.vdf")] {
            let Ok(bytes) = fs::read(&vdf) else { continue };
            let Ok((text, _)) = hud_syntax::decode(&bytes) else { continue };
            for r in hud_syntax::tree(&text).roots {
                for lib in r.children.iter().filter(|c| c.is_block()) {
                    if let Some(p) = lib.get("path") {
                        libraries.push(PathBuf::from(p.replace("\\\\", "\\")));
                    }
                }
            }
        }
    }
    let mut seen = Vec::new();
    let mut out = Vec::new();
    for lib in libraries {
        let tf = lib.join("steamapps").join("common").join("Team Fortress 2").join("tf");
        if !tf.join("gameinfo.txt").is_file() {
            continue;
        }
        let canon = dunce::canonicalize(&tf).unwrap_or(tf.clone());
        if seen.contains(&canon) {
            continue;
        }
        seen.push(canon);
        out.push(GameInstall { tf_dir: tf.to_string_lossy().to_string(), huds: list_custom_huds(&tf) });
    }
    out
}

/// If `project` sits inside `<...>/tf/custom/<hud>`, returns that `tf` folder.
pub fn tf_dir_of_project(project: &Path) -> Option<PathBuf> {
    let parent = project.parent()?;
    if parent.file_name()?.to_string_lossy().eq_ignore_ascii_case("custom") {
        let tf = parent.parent()?;
        if tf.join("gameinfo.txt").is_file() {
            return Some(tf.to_path_buf());
        }
    }
    None
}

pub fn is_text_file(path: &str) -> bool {
    let n = normalize(path);
    let ext = n.rsplit('.').next().unwrap_or("");
    TEXT_EXTENSIONS.contains(&ext)
}
