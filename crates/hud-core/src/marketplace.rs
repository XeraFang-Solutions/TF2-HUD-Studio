//! Archive catalogue parsing and bounded ZIP import. Network access belongs to the desktop host.
use crate::{library, manifest, project};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const SOURCE: &str = "https://raw.githubusercontent.com/Hypnootize/TF2-HUDs-Archive/refs/heads/master/Updated%20HUDs%20List.md";
pub const MAX_DOWNLOAD: u64 = 256 * 1024 * 1024;
pub const MAX_EXPANDED: u64 = 1024 * 1024 * 1024;
pub const MAX_FILES: usize = 30_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub author: String,
    pub album: Option<String>,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub download: Option<String>,
}

pub fn https(s: &str) -> bool {
    s.starts_with("https://") && !s.chars().any(|c| c.is_control() || c.is_whitespace()) && s.len() > 8
}

/// The archive uses HTML anchors containing icon images inside Markdown table cells.
pub fn parse(text: &str) -> Result<Vec<Entry>, String> {
    let html = Regex::new(r#"(?i)href\s*=\s*["']([^"']+)["']"#).unwrap();
    let md = Regex::new(r"\[[^\]]*\]\((https://[^\s)]+)\)").unwrap();
    let tags = Regex::new(r"<[^>]*>").unwrap();
    let plain = |s: &str| tags.replace_all(s.trim(), "").replace("&amp;", "&").replace("&#39;", "'").replace("&quot;", "\"");
    let link = |s: &str| html.captures(s).or_else(|| md.captures(s)).map(|c| c[1].replace("&amp;", "&")).filter(|s| https(s));
    let mut columns: Option<Vec<String>> = None;
    let mut entries = Vec::new();
    for line in text.lines() {
        let mut cells: Vec<&str> = line.trim().split('|').map(str::trim).collect();
        if cells.first() == Some(&"") {
            cells.remove(0);
        }
        if cells.len() < 4 {
            continue;
        }
        if cells[0].eq_ignore_ascii_case("HUD") && cells[1].eq_ignore_ascii_case("Creator") {
            if cells.last() == Some(&"") {
                cells.pop();
            }
            columns = Some(cells.iter().map(|s| s.to_ascii_lowercase()).collect());
            continue;
        }
        let Some(cols) = &columns else { continue };
        if cells.len() == cols.len() + 1 && cells.last() == Some(&"") {
            cells.pop();
        }
        if cells.len() != cols.len() || cells[0].chars().all(|c| matches!(c, ':' | '-' | ' ')) {
            continue;
        }
        let get = |key: &str| cols.iter().position(|c| c == key).and_then(|i| cells.get(i)).copied().unwrap_or("");
        let name = plain(get("hud"));
        if name.is_empty() || name.starts_with('<') {
            continue;
        }
        let repository = link(get("repository"));
        let download = link(get("download"));
        if repository.is_none() && download.is_none() {
            continue;
        }
        // Include the original name to keep similarly named HUDs distinct without relying on folder slugs.
        let id = format!("{}|{}", name, download.as_ref().or(repository.as_ref()).unwrap());
        if entries.iter().any(|e: &Entry| e.id == id) {
            continue;
        }
        entries.push(Entry {
            id,
            name,
            author: plain(get("creator")),
            album: link(get("album")),
            repository,
            homepage: link(get("comfig huds")).or_else(|| link(get("tf2 huds"))),
            download,
        });
    }
    if entries.is_empty() {
        return Err("The updated HUD list has no readable entries. Its format may have changed.".into());
    }
    Ok(entries)
}

/// Reject traversal, NTFS streams and device names on every platform, including archives made elsewhere.
pub fn archive_path(name: &str) -> Result<PathBuf, String> {
    let normalized = name.replace('\\', "/");
    let p = normalized.trim_end_matches('/');
    if p.split('/').count() > 64
        || p.is_empty()
        || p.starts_with('/')
        || p.split('/').any(|c| {
            let stem = c.split('.').next().unwrap_or("").to_ascii_uppercase();
            c.is_empty()
                || matches!(c, "." | "..")
                || c.ends_with(['.', ' '])
                || c.chars().any(|v| v.is_control() || matches!(v, ':' | '*' | '?' | '"' | '<' | '>' | '|'))
                || matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                || (stem.len() == 4 && (stem.starts_with("COM") || stem.starts_with("LPT")) && matches!(stem.as_bytes()[3], b'1'..=b'9'))
        })
    {
        return Err(format!("Unsafe archive path: {name}"));
    }
    Ok(PathBuf::from(p))
}

pub fn extract_zip(bytes: &[u8], dest: &Path) -> Result<(), String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("Not a readable ZIP archive: {e}"))?;
    if zip.len() > MAX_FILES {
        return Err("Archive contains too many files".into());
    }
    let mut total = 0u64;
    // Validate all entries before writing anything.
    for i in 0..zip.len() {
        let file = zip.by_index(i).map_err(|e| e.to_string())?;
        archive_path(file.name())?;
        if file.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            return Err("Archive links are not supported".into());
        }
        total = total.checked_add(file.size()).ok_or("Archive is too large")?;
        if total > MAX_EXPANDED {
            return Err("Archive expands beyond 1 GiB".into());
        }
    }
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(|e| e.to_string())?;
        let target = dest.join(archive_path(file.name())?);
        if file.is_dir() {
            fs::create_dir_all(target).map_err(|e| e.to_string())?;
            continue;
        }
        fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
        let mut out = fs::OpenOptions::new().write(true).create_new(true).open(target).map_err(|e| e.to_string())?;
        let size = file.size();
        let copied = std::io::copy(&mut (&mut file).take(size + 1), &mut out).map_err(|e| e.to_string())?;
        if copied != size {
            return Err("Archive entry size mismatch".into());
        }
        out.flush().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Locate exactly one HUD through common wrapper folders; reject ambiguous packs.
pub fn hud_root(dir: &Path, depth: usize) -> Result<PathBuf, String> {
    // A manifest by itself isn't sufficient evidence that a download contains a HUD.
    if dir.join("info.vdf").is_file()
        || dir.join("scripts/hudlayout.res").is_file()
        || dir.join("resource/ui").is_dir()
        || dir.join("resource/clientscheme.res").is_file()
    {
        return Ok(dir.to_path_buf());
    }
    if depth == 0 {
        return Err("No HUD found in this archive".into());
    }
    let roots: Vec<_> = fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .flatten()
        .filter(|e| e.path().is_dir() && !library::is_link(&e.path()))
        .filter_map(|e| hud_root(&e.path(), depth - 1).ok())
        .collect();
    if roots.len() != 1 {
        return Err("Archive must contain exactly one HUD; extract it and use Add a HUD folder for packs.".into());
    }
    Ok(roots[0].clone())
}

pub fn prepare_manifest(hud: &Path, entry: &Entry) -> Result<(), String> {
    if fs::read_dir(hud).map_err(|e| e.to_string())?.flatten().any(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(manifest::FILE_NAME)) {
        return Ok(()); // Never replace an author's manifest, including one we can't parse.
    }
    let mut m = manifest::Manifest {
        name: entry.name.clone(),
        author: entry.author.clone(),
        homepage: entry.homepage.clone().or(entry.album.clone()).unwrap_or_default(),
        source: entry.repository.clone().unwrap_or_default(),
        ..Default::default()
    };
    // Album pages are links, not local banner images. Keep their provenance as extra manifest keys.
    m.description = "Imported from Hypnootize's updated TF2 HUD list.".into();
    manifest::write(hud, &m)?;
    let path = hud.join(manifest::FILE_NAME);
    let mut text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut extra = format!("\t\"catalogue\" \"{SOURCE}\"\n");
    for (key, value) in [("album", &entry.album), ("download", &entry.download)] {
        if let Some(v) = value {
            extra.push_str(&format!("\t\"{key}\" \"{}\"\n", v.replace(['"', '\r', '\n'], "'")));
        }
    }
    if let Some(end) = text.rfind('}') {
        text.insert_str(end, &extra);
    }
    fs::write(path, text).map_err(|e| e.to_string())
}

/// Publish a completely prepared HUD without overwriting another library entry.
pub fn publish(hud: &Path, lib: &Path, name: &str) -> Result<String, String> {
    let base = project::safe_folder_name(name);
    for i in 1..10_000 {
        let dest = lib.join(if i == 1 { base.clone() } else { format!("{base}-{i}") });
        if fs::symlink_metadata(&dest).is_ok() {
            continue;
        }
        fs::rename(hud, &dest).map_err(|e| e.to_string())?;
        return Ok(dest.to_string_lossy().to_string());
    }
    Err("Too many copies of this HUD in the library".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_archive_html_and_markdown_links() {
        let text = "HUD | Creator | Album | Repository | Download\n:---: | --- | --- | --- | ---\nTest | Me &amp; You | <a href=\"https://imgur.com/a/test\"><img src=\"/icons/screenshots.png\"></a> | <a href='https://github.com/a/b'>icon</a> <a href='https://github.com/old/b'>old</a> | [zip](https://github.com/a/b/archive/main.zip)";
        let entries = parse(text).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].author, "Me & You");
        assert_eq!(entries[0].repository.as_deref(), Some("https://github.com/a/b"));
        assert_eq!(entries[0].album.as_deref(), Some("https://imgur.com/a/test"));
        assert!(parse("an error page").is_err());
        let missing_download =
            parse("| HUD | Creator | Album | Repository | Download |\n| No download | Author | | [repo](https://github.com/a/b) | |").unwrap();
        assert_eq!(missing_download.len(), 1);
        assert!(missing_download[0].download.is_none());
    }
    #[test]
    fn rejects_nonportable_archive_paths() {
        for p in ["../outside", "x/../../outside", "C:\\outside", "\\\\server\\share", "x:ads", "x/CON.txt", "x/dir. /file", "/absolute", "x/../file"] {
            assert!(archive_path(p).is_err(), "{p}");
        }
        assert_eq!(archive_path("hud/resource/ui/").unwrap(), PathBuf::from("hud/resource/ui"));
    }
    #[test]
    fn rejects_zip_traversal_and_links_before_extraction() {
        for path in ["../escape.txt", "hud/C:stream", "hud/NUL.txt"] {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
            zip.start_file(path, zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(b"bad").unwrap();
            let bytes = zip.finish().unwrap().into_inner();
            assert!(extract_zip(&bytes, Path::new("unused-must-not-be-created")).is_err());
        }
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.add_symlink("hud/link", "../../outside", zip::write::SimpleFileOptions::default()).unwrap();
        assert!(extract_zip(&zip.finish().unwrap().into_inner(), Path::new("unused-must-not-be-created")).is_err());
    }
    #[test]
    fn imports_zip_and_preserves_manifests() {
        let root = std::env::temp_dir().join(format!("market-test-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.start_file("wrapper/hud/info.vdf", zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(b"\"HUD\" { \"name\" \"Test\" }").unwrap();
        let bytes = zip.finish().unwrap().into_inner();
        let extracted = root.join("extract");
        extract_zip(&bytes, &extracted).unwrap();
        let hud = hud_root(&extracted, 4).unwrap();
        let entry = Entry {
            id: "test".into(),
            name: "Test".into(),
            author: "Author".into(),
            album: Some("https://imgur.com/a/test".into()),
            repository: Some("https://github.com/a/b".into()),
            homepage: None,
            download: None,
        };
        prepare_manifest(&hud, &entry).unwrap();
        assert_eq!(manifest::read(&hud).unwrap().author, "Author");
        let original = fs::read(hud.join(manifest::FILE_NAME)).unwrap();
        prepare_manifest(&hud, &entry).unwrap();
        assert_eq!(fs::read(hud.join(manifest::FILE_NAME)).unwrap(), original);
        let lib = root.join("library");
        fs::create_dir_all(lib.join("test")).unwrap();
        let dest = publish(&hud, &lib, "Test").unwrap();
        assert!(dest.ends_with("test-2"));
        fs::remove_dir_all(root).unwrap();
    }
}
