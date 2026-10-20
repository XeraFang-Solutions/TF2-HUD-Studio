//! Client scheme (colors, fonts, borders, custom font files) resolved through `#base` like the engine does.
use crate::fontmeta;
use crate::resolve::{resolve, RNode};
use crate::vfs::{normalize, Vfs};
use base64::Engine;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorDef {
    pub name: String,
    pub raw: String,
    pub rgba: Option<[u8; 4]>,
    pub file: String,
    pub editable: bool,
    pub id: String,
    pub offset: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FontVariant {
    pub key: String,
    pub props: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FontDef {
    pub name: String,
    pub variants: Vec<FontVariant>,
    pub file: String,
    pub editable: bool,
    pub id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FontFile {
    pub path: String,
    pub declared_name: Option<String>,
    pub found: bool,
    pub origin: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Scheme {
    pub file: Option<String>,
    pub chain: Vec<String>,
    pub colors: Vec<ColorDef>,
    pub base_settings: Vec<(String, String)>,
    pub fonts: Vec<FontDef>,
    pub borders: Vec<RNode>,
    pub font_files: Vec<FontFile>,
}

/// Parses "R G B A" (alpha optional). Returns None for names and malformed values.
pub fn parse_rgba(raw: &str) -> Option<[u8; 4]> {
    let parts: Vec<&str> = raw.split_whitespace().collect();
    if parts.len() < 3 || parts.len() > 4 {
        return None;
    }
    let mut out = [255u8; 4];
    for (i, p) in parts.iter().enumerate() {
        // the engine uses sscanf("%d %d %d %d") then clamps to a byte
        let digits: String = p.chars().take_while(|c| c.is_ascii_digit() || *c == '-').collect();
        let v: i64 = digits.parse().ok()?;
        out[i] = v.clamp(0, 255) as u8;
    }
    Some(out)
}

impl Scheme {
    pub fn color(&self, name_or_value: &str) -> Option<[u8; 4]> {
        if let Some(c) = parse_rgba(name_or_value) {
            return Some(c);
        }
        let mut name = name_or_value.to_string();
        for _ in 0..4 {
            let def = self.colors.iter().find(|c| c.name.eq_ignore_ascii_case(&name))?;
            if let Some(c) = def.rgba {
                return Some(c);
            }
            name = def.raw.clone();
        }
        None
    }
    pub fn has_font(&self, name: &str) -> bool {
        self.fonts.iter().any(|f| f.name.eq_ignore_ascii_case(name))
    }
}

pub fn load(vfs: &Vfs) -> Scheme {
    let mut scheme =
        Scheme { file: None, chain: Vec::new(), colors: Vec::new(), base_settings: Vec::new(), fonts: Vec::new(), borders: Vec::new(), font_files: Vec::new() };
    let Some(resolved) = resolve(vfs, "resource/clientscheme.res") else { return scheme };
    scheme.file = Some(resolved.origin.label());
    scheme.chain = resolved.chain.clone();
    let Some(root) = resolved.roots.iter().find(|r| r.key.eq_ignore_ascii_case("Scheme")) else { return scheme };
    if let Some(colors) = root.child("Colors") {
        for c in colors.children.iter().filter(|c| !c.is_block()) {
            if scheme.colors.iter().any(|e| e.name.eq_ignore_ascii_case(&c.key)) {
                continue; // first definition wins, like FindKey
            }
            let raw = c.value.clone().unwrap_or_default();
            scheme.colors.push(ColorDef {
                name: c.key.clone(),
                rgba: parse_rgba(&raw),
                raw,
                file: c.file.clone(),
                editable: c.editable,
                id: c.id.clone(),
                offset: c.offset,
            });
        }
    }
    if let Some(base) = root.child("BaseSettings") {
        for c in base.children.iter().filter(|c| !c.is_block()) {
            scheme.base_settings.push((c.key.clone(), c.value.clone().unwrap_or_default()));
        }
    }
    if let Some(fonts) = root.child("Fonts") {
        for f in fonts.blocks() {
            if scheme.fonts.iter().any(|e| e.name.eq_ignore_ascii_case(&f.key)) {
                continue;
            }
            let variants = f
                .blocks()
                .map(|v| FontVariant {
                    key: v.key.clone(),
                    props: v.children.iter().filter(|p| !p.is_block()).map(|p| (p.key.to_ascii_lowercase(), p.value.clone().unwrap_or_default())).collect(),
                })
                .collect();
            scheme.fonts.push(FontDef { name: f.key.clone(), variants, file: f.file.clone(), editable: f.editable, id: f.id.clone() });
        }
    }
    if let Some(borders) = root.child("Borders") {
        scheme.borders = borders.blocks().cloned().collect();
    }
    if let Some(files) = root.child("CustomFontFiles") {
        for entry in &files.children {
            let (path, declared) = if entry.is_block() {
                (entry.get("font").unwrap_or_default().to_string(), entry.get("name").map(str::to_string))
            } else {
                (entry.value.clone().unwrap_or_default(), None)
            };
            if path.is_empty() {
                continue;
            }
            let found = vfs.read_bytes(&path).map(|(_, o)| o.label());
            scheme.font_files.push(FontFile { path, declared_name: declared, found: found.is_some(), origin: found });
        }
    }
    scheme
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FontData {
    /// Family names the preview should register this file under (declared scheme name + the file's own family).
    pub families: Vec<String>,
    pub path: String,
    pub base64: String,
    /// Windows GDI cell height / em size; VGUI "tall" is a cell height, CSS font-size is an em size.
    pub cell_ratio: f32,
    pub ascent_ratio: f32,
}

/// Loads every custom font the scheme declares plus TF2's own fonts, ready for `FontFace` in the webview.
pub fn font_data(vfs: &Vfs, scheme: &Scheme) -> Vec<FontData> {
    let mut paths: Vec<(String, Option<String>)> = scheme.font_files.iter().map(|f| (f.path.clone(), f.declared_name.clone())).collect();
    for stock in ["resource/tf2.ttf", "resource/tf2build.ttf", "resource/tf2secondary.ttf", "resource/tf2professor.ttf"] {
        if !paths.iter().any(|(p, _)| normalize(p) == stock) {
            paths.push((stock.into(), None));
        }
    }
    let mut out = Vec::new();
    for (path, declared) in paths {
        let Some((bytes, _)) = vfs.read_bytes(&path) else { continue };
        let meta = fontmeta::read(&bytes);
        let mut families = Vec::new();
        if let Some(d) = declared {
            families.push(d)
        }
        if let Some(m) = &meta {
            for f in [&m.family, &m.full_name, &m.typographic_family].into_iter().flatten() {
                if !families.iter().any(|x| x.eq_ignore_ascii_case(f)) {
                    families.push(f.clone())
                }
            }
        }
        if families.is_empty() {
            continue;
        }
        let (cell, ascent) = meta.as_ref().map(|m| (m.cell_ratio, m.ascent_ratio)).unwrap_or((1.2, 0.95));
        out.push(FontData { families, path, base64: base64::engine::general_purpose::STANDARD.encode(&bytes), cell_ratio: cell, ascent_ratio: ascent });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rgba() {
        assert_eq!(parse_rgba("255 0 10"), Some([255, 0, 10, 255]));
        assert_eq!(parse_rgba("1 2 3 4"), Some([1, 2, 3, 4]));
        assert_eq!(parse_rgba("232 184 99  255"), Some([232, 184, 99, 255]));
        assert_eq!(parse_rgba("White"), None);
    }
}
