//! `TFHUD.manifest`: a HUD's name, version, author, description, icon and links, kept at the HUD's root.
//!
//! It's a KeyValues file like `info.vdf`, so it looks at home in a TF2 folder and TF2 simply ignores it:
//!
//! ```text
//! "TFHUD"
//! {
//!     "manifest_version"  "1"
//!     "name"              "budhud"
//!     "version"           "2.4.0"
//!     "author"            "rbjaxter"
//!     "description"       "A HUD for TF2."
//!     "icon"              "tfhud/icon.png"
//!     "banner"            "tfhud/banner.png"
//!     "homepage"          "https://…"
//!     "source"            "https://github.com/…"
//!     "license"           "…"
//!     "tags"              "minimal, competitive"
//! }
//! ```
//!
//! Edits are lossless: keys the studio doesn't know about, comments and spacing are kept.
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = "TFHUD.manifest";
pub const ROOT_KEY: &str = "TFHUD";
pub const VERSION: &str = "1";
/// Folder (inside the HUD) where the studio stores the icon and banner it's given.
pub const ASSET_DIR: &str = "tfhud";
const MAX_IMAGE_BYTES: u64 = 4 * 1024 * 1024;

/// The fields the studio edits, in the order they're written into a new manifest.
pub const FIELDS: [&str; 10] = ["name", "version", "author", "description", "icon", "banner", "homepage", "source", "license", "tags"];

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    /// Path relative to the HUD folder.
    pub icon: String,
    /// Path relative to the HUD folder; a wide preview image.
    pub banner: String,
    pub homepage: String,
    pub source: String,
    pub license: String,
    /// Comma-separated.
    pub tags: String,
}

impl Manifest {
    fn get(&self, key: &str) -> &str {
        match key {
            "name" => &self.name,
            "version" => &self.version,
            "author" => &self.author,
            "description" => &self.description,
            "icon" => &self.icon,
            "banner" => &self.banner,
            "homepage" => &self.homepage,
            "source" => &self.source,
            "license" => &self.license,
            "tags" => &self.tags,
            _ => "",
        }
    }
    fn set(&mut self, key: &str, v: String) {
        match key.to_ascii_lowercase().as_str() {
            "name" => self.name = v,
            "version" => self.version = v,
            "author" => self.author = v,
            "description" => self.description = v,
            "icon" => self.icon = v,
            "banner" => self.banner = v,
            "homepage" => self.homepage = v,
            "source" => self.source = v,
            "license" => self.license = v,
            "tags" => self.tags = v,
            _ => {}
        }
    }
}

/// A manifest as shown in the UI: the fields plus the images as data URLs.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HudDetails {
    /// Whether the HUD has a `TFHUD.manifest` (otherwise the fields come from `info.vdf` and the folder name).
    pub has_manifest: bool,
    pub manifest: Manifest,
    pub icon_url: Option<String>,
    pub banner_url: Option<String>,
}

/// Reads `TFHUD.manifest` (case-insensitive file name) from a HUD folder.
pub fn read(hud: &Path) -> Option<Manifest> {
    let path = find_file(hud)?;
    let bytes = fs::read(path).ok()?;
    let (text, _) = hud_syntax::decode(&bytes).ok()?;
    let t = hud_syntax::tree(&text);
    let root = t.roots.iter().find(|r| r.is_block())?;
    let mut m = Manifest::default();
    for c in root.children.iter().filter(|c| !c.is_block()) {
        if m.get(&c.key.to_ascii_lowercase()).is_empty() {
            m.set(&c.key, c.value.clone().unwrap_or_default());
        }
    }
    Some(m)
}

fn find_file(hud: &Path) -> Option<PathBuf> {
    let exact = hud.join(FILE_NAME);
    if exact.is_file() {
        return Some(exact);
    }
    fs::read_dir(hud)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.is_file() && p.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(FILE_NAME)))
}

fn image_url(hud: &Path, rel: &str) -> Option<String> {
    if rel.trim().is_empty() {
        return None;
    }
    let path = safe_join(hud, rel)?;
    if fs::metadata(&path).ok()?.len() > MAX_IMAGE_BYTES {
        return None;
    }
    let mime = match path.extension()?.to_string_lossy().to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        _ => return None,
    };
    let bytes = fs::read(&path).ok()?;
    Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

/// Joins a manifest-relative path, refusing anything that escapes the HUD folder.
fn safe_join(hud: &Path, rel: &str) -> Option<PathBuf> {
    let rel = rel.replace('\\', "/");
    if rel.starts_with('/') || rel.contains(':') || rel.split('/').any(|s| s == "..") {
        return None;
    }
    Some(hud.join(rel))
}

/// The manifest if there is one, otherwise what can be worked out from `info.vdf` and the folder.
pub fn details(hud: &Path) -> HudDetails {
    match read(hud) {
        Some(mut m) => {
            if m.name.trim().is_empty() {
                m.name = crate::project::hud_name(hud);
            }
            HudDetails { has_manifest: true, icon_url: image_url(hud, &m.icon), banner_url: image_url(hud, &m.banner), manifest: m }
        }
        None => HudDetails {
            has_manifest: false,
            manifest: Manifest { name: crate::project::hud_name(hud), ..Default::default() },
            icon_url: None,
            banner_url: None,
        },
    }
}

fn new_file(m: &Manifest) -> String {
    let mut s = format!("\"{ROOT_KEY}\"\n{{\n");
    let row = |k: &str, v: &str| {
        let pad = "\t".repeat(((24usize.saturating_sub(k.len() + 2)).div_ceil(4)).max(1));
        format!("\t\"{k}\"{pad}\"{}\"\n", v.replace('"', "'"))
    };
    s += &row("manifest_version", VERSION);
    for k in FIELDS {
        let v = m.get(k);
        if !v.is_empty() || matches!(k, "name" | "version" | "author" | "description") {
            s += &row(k, v);
        }
    }
    s += "}\n";
    s
}

/// Writes the manifest. An existing file is edited in place (unknown keys, comments and spacing survive);
/// fields left empty are removed from it.
pub fn write(hud: &Path, m: &Manifest) -> Result<PathBuf, String> {
    let clean = |v: &str| v.replace(['\r', '\n'], " ").replace('"', "'");
    let path = find_file(hud).unwrap_or_else(|| hud.join(FILE_NAME));
    let existing = fs::read(&path).ok().and_then(|b| hud_syntax::decode(&b).ok());
    let (out, enc) = match existing {
        Some((mut src, enc)) => {
            let t = hud_syntax::tree(&src);
            let Some(root) = t.roots.iter().find(|r| r.is_block()) else { return Err(format!("{} has no root block", path.display())) };
            let id = root.id.clone();
            for k in FIELDS {
                let v = clean(m.get(k));
                let present = root.find(k).is_some_and(|n| !n.is_block());
                if v.is_empty() {
                    if present {
                        src = hud_syntax::remove_property(&src, &id, k, None)?;
                    }
                } else if root.get(k) != Some(v.as_str()) {
                    src = hud_syntax::set_property(&src, &id, k, &v, None)?;
                }
            }
            if hud_syntax::tree(&src).roots.iter().find(|r| r.is_block()).and_then(|r| r.find("manifest_version")).is_none() {
                src = hud_syntax::set_property(&src, &id, "manifest_version", VERSION, None)?;
            }
            (src, enc)
        }
        None => {
            let mut c = m.clone();
            for k in FIELDS {
                let v = clean(c.get(k));
                c.set(k, v);
            }
            (new_file(&c), hud_syntax::Encoding::Utf8)
        }
    };
    fs::write(&path, hud_syntax::encode(&out, enc)).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

/// Copies an image into `<hud>/tfhud/<slot>.<ext>` and returns the manifest-relative path. `slot` is "icon" or "banner".
pub fn import_image(hud: &Path, slot: &str, src: &Path) -> Result<String, String> {
    if !matches!(slot, "icon" | "banner") {
        return Err("Unknown image slot".into());
    }
    let ext = src.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg") {
        return Err("Use a PNG, JPG, WebP, GIF or SVG image".into());
    }
    if fs::metadata(src).map_err(|e| e.to_string())?.len() > MAX_IMAGE_BYTES {
        return Err("That image is over 4 MB; please use a smaller one".into());
    }
    let dir = hud.join(ASSET_DIR);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // drop older images in this slot with another extension
    if let Ok(entries) = fs::read_dir(&dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.file_stem().is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case(slot)) {
                let _ = fs::remove_file(p);
            }
        }
    }
    let dest = dir.join(format!("{slot}.{ext}"));
    fs::copy(src, &dest).map_err(|e| e.to_string())?;
    Ok(format!("{ASSET_DIR}/{slot}.{ext}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_read_and_edit_losslessly() {
        let d = std::env::temp_dir().join(format!("tfhud-manifest-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("info.vdf"), "\"Sayo\"\n{\n\t\"ui_version\"\t\"3\"\n}\n").unwrap();

        let det = details(&d);
        assert!(!det.has_manifest);
        assert_eq!(det.manifest.name, "Sayo", "falls back to info.vdf");

        let m = Manifest { name: "bxhud sayo".into(), version: "1.0.0".into(), description: "Flush-left \"chat\"".into(), ..Default::default() };
        write(&d, &m).unwrap();
        let text = fs::read_to_string(d.join(FILE_NAME)).unwrap();
        assert!(text.starts_with("\"TFHUD\""));
        assert!(text.contains("\"manifest_version\""));
        assert!(!text.contains("\"homepage\""), "empty optional fields are left out");
        let back = read(&d).unwrap();
        assert_eq!(back.name, "bxhud sayo");
        assert_eq!(back.description, "Flush-left 'chat'");

        // hand edits survive a save
        let edited = text.replace("}\n", "\t// my note\n\t\"custom_key\"\t\"keep me\"\n}\n");
        fs::write(d.join(FILE_NAME), &edited).unwrap();
        let m2 = Manifest { version: "1.1.0".into(), homepage: "https://example.com".into(), description: String::new(), ..back };
        write(&d, &m2).unwrap();
        let after = fs::read_to_string(d.join(FILE_NAME)).unwrap();
        assert!(after.contains("// my note") && after.contains("\"custom_key\"\t\"keep me\""));
        assert!(after.contains("\"1.1.0\"") && after.contains("https://example.com"));
        assert!(!after.contains("\"description\""), "cleared fields are removed");

        // icon import
        fs::write(d.join("pic.png"), [0x89, b'P', b'N', b'G']).unwrap();
        let rel = import_image(&d, "icon", &d.join("pic.png")).unwrap();
        assert_eq!(rel, "tfhud/icon.png");
        assert!(image_url(&d, &rel).unwrap().starts_with("data:image/png;base64,"));
        assert!(image_url(&d, "../../etc/passwd.png").is_none(), "paths can't leave the HUD");
        let _ = fs::remove_dir_all(&d);
    }
}
