//! Network and external extraction for the HUD marketplace. All blocking work runs off the UI thread.
use crate::{session::SessionState, tools, StoreState};
use base64::Engine;
use hud_core::marketplace::{self as core, Entry};
use serde::Serialize;
use std::{
    fs,
    hash::{Hash, Hasher},
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Mutex,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{Manager, State};

#[derive(Default)]
pub struct MarketplaceState {
    entries: Mutex<Vec<Entry>>,
    importing: Mutex<()>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalogue {
    entries: Vec<Entry>,
    cached: bool,
    warning: Option<String>,
}

fn get(url: &str, limit: u64) -> Result<Vec<u8>, String> {
    if !core::https(url) {
        return Err("Only HTTPS links are supported".into());
    }
    let timeout = if limit <= 4 * 1024 * 1024 { 20 } else { 120 };
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .https_only(true)
        .timeout_global(Some(Duration::from_secs(timeout)))
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_resolve(Some(Duration::from_secs(10)))
        .max_redirects(8)
        .build()
        .into();
    let mut response = agent.get(url).header("User-Agent", "TF2-HUD-Studio").call().map_err(|e| format!("Download failed: {e}"))?;
    let mut bytes = Vec::new();
    response.body_mut().as_reader().take(limit + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("Download exceeds the size limit".into());
    }
    Ok(bytes)
}

#[tauri::command]
pub async fn marketplace_list(app: tauri::AppHandle, state: State<'_, MarketplaceState>) -> Result<Catalogue, String> {
    let cache = app.path().app_cache_dir().map_err(|e| e.to_string())?.join("marketplace-list.md");
    let result: Result<Catalogue, String> = tauri::async_runtime::spawn_blocking(move || {
        let live = get(core::SOURCE, 2 * 1024 * 1024)
            .and_then(|b| String::from_utf8(b).map_err(|e| e.to_string()))
            .and_then(|text| core::parse(&text).map(|entries| (text, entries)));
        match live {
            Ok((text, entries)) => {
                if let Some(parent) = cache.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::write(cache, text);
                Ok(Catalogue { entries, cached: false, warning: None })
            }
            Err(error) => {
                let entries = fs::read_to_string(cache)
                    .ok()
                    .and_then(|s| core::parse(&s).ok())
                    .ok_or_else(|| format!("Cannot load the HUD marketplace: {error}. Check your connection and retry."))?;
                Ok(Catalogue { entries, cached: true, warning: Some(format!("Showing the last saved list: {error}")) })
            }
        }
    })
    .await
    .map_err(|e| e.to_string())?;
    let result: Catalogue = result?;
    *state.entries.lock().map_err(|_| "Marketplace state poisoned")? = result.entries.clone();
    Ok(result)
}

#[tauri::command]
pub fn marketplace_open(url: String) -> Result<(), String> {
    if !core::https(&url) {
        return Err("Only HTTPS links can be opened".into());
    }
    open::that_detached(url).map_err(|e| e.to_string())
}

const MAX_PREVIEW: u64 = 4 * 1024 * 1024;

fn album_preview(album: &str, cache: &Path) -> Option<Vec<u8>> {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    album.hash(&mut hash);
    let file = cache.join(format!("{:016x}.image", hash.finish()));
    // only read the file once its size is known to be within the limit
    let saved = match fs::metadata(&file) {
        Ok(m) if m.len() <= MAX_PREVIEW => fs::read(&file).ok().filter(|b| crate::album::image_type(b).is_some()).map(|b| (m, b)),
        _ => None,
    };
    if let Some((meta, bytes)) = &saved {
        if meta.modified().ok().and_then(|t| t.elapsed().ok()).is_some_and(|age| age < Duration::from_secs(86400)) {
            return Some(bytes.clone());
        }
    }
    let fetch = || -> Option<Vec<u8>> {
        let album = crate::album::raw_github(album);
        let image = if crate::album::direct_image(&album) {
            album
        } else {
            let page = String::from_utf8(get(&album, 2 * 1024 * 1024).ok()?).ok()?;
            crate::album::first_image(&album, &page)?
        };
        let bytes = get(&image, MAX_PREVIEW).ok()?;
        crate::album::image_type(&bytes)?;
        Some(bytes)
    };
    if let Some(bytes) = fetch() {
        if fs::create_dir_all(cache).is_ok() {
            let _ = fs::write(file, &bytes);
        }
        Some(bytes)
    } else {
        saved.map(|(_, bytes)| bytes)
    }
}

/// Resolve only catalogue albums, keeping fetches and image decoding out of the webview.
#[tauri::command]
pub async fn marketplace_preview(id: String, app: tauri::AppHandle, state: State<'_, MarketplaceState>) -> Result<Option<String>, String> {
    let album = state.entries.lock().map_err(|_| "Marketplace state poisoned")?.iter().find(|e| e.id == id).and_then(|e| e.album.clone());
    let Some(album) = album else { return Ok(None) };
    let cache = app.path().app_cache_dir().map_err(|e| e.to_string())?.join("album-previews");
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = album_preview(&album, &cache)?;
        let (mime, _) = crate::album::image_type(&bytes)?;
        Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
    })
    .await
    .map_err(|e| e.to_string())
}

// A private, uniquely created staging directory on the same volume as the library. Publication is a rename.
struct Staging(PathBuf);
impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
impl Staging {
    fn new(lib: &Path) -> Result<Self, String> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos();
        let path = lib.join(format!(".marketplace-{}-{now}", std::process::id()));
        fs::create_dir(&path).map_err(|e| e.to_string())?;
        Ok(Self(path))
    }
}

/// Bound CLI run time and output, avoid interactive password prompts, never invoke a shell.
fn run7z(exe: &Path, args: &[String], work: &Path) -> Result<String, String> {
    let log = work.join("extractor.log");
    let file = fs::File::create(&log).map_err(|e| e.to_string())?;
    let mut command = Command::new(exe);
    command.args(args).stdin(Stdio::null()).stdout(file).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().map_err(|e| format!("Cannot start archive tool: {e}"))?;
    let start = Instant::now();
    loop {
        if start.elapsed() > Duration::from_secs(120) || fs::metadata(&log).map(|m| m.len() > 8 * 1024 * 1024).unwrap_or(false) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Archive tool exceeded the time or output limit".into());
        }
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            if !status.success() {
                return Err(format!("Archive tool failed ({status}). The archive may be damaged, encrypted, or unsupported."));
            }
            return fs::read_to_string(log).map_err(|e| e.to_string());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn check_listing(list: &str) -> Result<(), String> {
    let mut files = 0;
    let mut total = 0u64;
    for line in list.lines() {
        if let Some(path) = line.strip_prefix("Path = ") {
            core::archive_path(path)?;
            files += 1;
        }
        if let Some(size) = line.strip_prefix("Size = ") {
            total = total.checked_add(size.parse::<u64>().map_err(|_| "Invalid archive size")?).ok_or("Archive is too large")?;
        }
        if line.starts_with("Symbolic Link = ")
            || line.starts_with("Hard Link = ")
            || line.starts_with("Reparse Point = ")
            || line.starts_with("Alternate Stream = +")
            || line.starts_with("Encrypted = +")
            || line.strip_prefix("Attributes = ").is_some_and(|a| a.contains('l') || a.contains("Reparse"))
        {
            return Err("Encrypted archives, links and alternate streams cannot be imported".into());
        }
    }
    if files == 0 || files > core::MAX_FILES || total > core::MAX_EXPANDED {
        return Err("Archive has no files or exceeds import limits".into());
    }
    Ok(())
}

fn check_tree(dir: &Path, files: &mut usize, size: &mut u64) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if hud_core::library::is_link(&entry.path()) {
            return Err("Extracted HUD contains a link".into());
        }
        let meta = entry.metadata().map_err(|e| e.to_string())?;
        *files += 1;
        *size = size.saturating_add(meta.len());
        if *files > core::MAX_FILES || *size > core::MAX_EXPANDED {
            return Err("Extracted HUD exceeds import limits".into());
        }
        if meta.is_dir() {
            check_tree(&entry.path(), files, size)?;
        } else if !meta.is_file() {
            return Err("Archive contains a special file".into());
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn marketplace_import(
    id: String,
    app: tauri::AppHandle,
    store: State<'_, StoreState>,
    session: State<'_, SessionState>,
    state: State<'_, MarketplaceState>,
) -> Result<String, String> {
    let entry = state
        .entries
        .lock()
        .map_err(|_| "Marketplace state poisoned")?
        .iter()
        .find(|e| e.id == id)
        .cloned()
        .ok_or("Refresh the marketplace before downloading this HUD")?;
    let (lib, tf) = {
        let store = store.lock().map_err(|_| "Settings state poisoned")?;
        (store.settings.hud_library.clone().ok_or("Choose a HUD library folder first")?, store.settings.tf_dir.clone())
    };
    let tool = tools::tool_path(&session, "7zip").or_else(|| tools::tool_path(&session, "peazip-7z"));
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<MarketplaceState>();
        let _import = state.importing.try_lock().map_err(|_| "Another HUD download is already running")?;
        let lib = dunce::canonicalize(lib).map_err(|e| e.to_string())?;
        if let Some(custom) = tf.and_then(|t| dunce::canonicalize(Path::new(&t).join("custom")).ok()) {
            if lib.starts_with(custom) {
                return Err("Choose a library folder outside tf/custom before downloading HUDs".into());
            }
        }
        let staging = Staging::new(&lib)?;
        let url = entry.download.as_ref().ok_or("This HUD has no download link")?;
        let bytes = get(url, core::MAX_DOWNLOAD)?;
        let dest = staging.0.join("contents");
        fs::create_dir(&dest).map_err(|e| e.to_string())?;
        if bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06") {
            core::extract_zip(&bytes, &dest)?;
        } else if bytes.starts_with(b"7z\xbc\xaf\x27\x1c") || bytes.starts_with(b"Rar!\x1a\x07") {
            let exe = tool.ok_or("This download needs 7-Zip. Install it or locate its executable in the marketplace, then retry.")?;
            let archive = staging.0.join("download.archive");
            fs::write(&archive, bytes).map_err(|e| e.to_string())?;
            let archive = archive.to_string_lossy().to_string();
            let listing =
                run7z(&exe, &["l".into(), "-slt".into(), "-ba".into(), "-sccUTF-8".into(), "-pHUDStudio-no-password".into(), archive.clone()], &staging.0)?;
            check_listing(&listing)?;
            run7z(
                &exe,
                &[
                    "x".into(),
                    "-y".into(),
                    "-bd".into(),
                    "-bb0".into(),
                    "-sccUTF-8".into(),
                    "-pHUDStudio-no-password".into(),
                    format!("-o{}", dest.display()),
                    archive,
                ],
                &staging.0,
            )?;
        } else {
            return Err("The link did not return a supported ZIP, 7z or RAR archive. Use Download to open its download page.".into());
        }
        check_tree(&dest, &mut 0, &mut 0)?;
        let hud = core::hud_root(&dest, 5)?;
        let has_manifest = fs::read_dir(&hud)
            .map_err(|e| e.to_string())?
            .flatten()
            .any(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(hud_core::manifest::FILE_NAME));
        core::prepare_manifest(&hud, &entry)?;
        // Give newly generated library cards the same preview, without modifying authored manifests.
        if !has_manifest {
            if let (Some(album), Ok(cache)) = (&entry.album, app.path().app_cache_dir()) {
                if let Some(bytes) = album_preview(album, &cache.join("album-previews")) {
                    if let (Some((_, ext)), Some(mut manifest)) = (crate::album::image_type(&bytes), hud_core::manifest::read(&hud)) {
                        let relative = format!("tfhud/marketplace-preview.{ext}");
                        let image = hud.join(&relative);
                        if !image.exists() && fs::create_dir_all(image.parent().unwrap()).is_ok() && fs::write(image, bytes).is_ok() {
                            manifest.banner = relative;
                            // Preview failure must never prevent an otherwise valid HUD import.
                            let _ = hud_core::manifest::write(&hud, &manifest);
                        }
                    }
                }
            }
        }
        core::publish(&hud, &lib, &entry.name)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installed_7zip_extracts_real_archive() {
        let exe = std::env::var_os("HUD_STUDIO_TEST_7ZIP").map(PathBuf::from);
        let Some(exe) = exe else { return }; // Opt in on CI or developer machines with 7-Zip installed.
        let root = std::env::temp_dir();
        let staging = Staging::new(&root).unwrap();
        let source = staging.0.join("source/info.vdf");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, b"\"HUD\" { \"ui_version\" \"3\" }").unwrap();
        let archive = staging.0.join("hud.7z").to_string_lossy().to_string();
        run7z(&exe, &["a".into(), "-t7z".into(), archive.clone(), source.to_string_lossy().to_string()], &staging.0).unwrap();
        let listing = run7z(&exe, &["l".into(), "-slt".into(), "-ba".into(), "-sccUTF-8".into(), archive.clone()], &staging.0).unwrap();
        check_listing(&listing).unwrap();
        let dest = staging.0.join("out");
        run7z(&exe, &["x".into(), "-y".into(), format!("-o{}", dest.display()), archive], &staging.0).unwrap();
        check_tree(&dest, &mut 0, &mut 0).unwrap();
        assert!(core::hud_root(&dest, 5).unwrap().join("info.vdf").is_file());
    }
    #[test]
    fn validates_external_archive_listing() {
        assert!(check_listing("Path = hud/info.vdf\nSize = 20\nAttributes = A\n").is_ok());
        for list in [
            "Path = ../outside\nSize = 1",
            "Path = hud/link\nSymbolic Link = /outside",
            "Path = hud/info.vdf\nEncrypted = +",
            "Path = hud/file\nSize = 1099511627776",
            "not a listing",
        ] {
            assert!(check_listing(list).is_err());
        }
    }
}
