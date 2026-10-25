//! Resolves VGUI image names to textures and decodes them for the preview.
use crate::vfs::{normalize, Vfs};
use crate::vtf;
use base64::Engine;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageResult {
    /// `data:image/png;base64,...`
    pub data_url: String,
    pub width: u32,
    pub height: u32,
    pub material: Option<String>,
    pub texture: String,
    pub origin: String,
    pub format: String,
}

/// VGUI image names are relative to `materials/vgui/` (`../` escapes it) and have no extension.
pub fn material_path(image: &str) -> String {
    let lower = image.trim().replace('\\', "/").to_ascii_lowercase();
    let trimmed = lower.trim_end_matches(".vmt").trim_end_matches(".vtf");
    normalize(&format!("materials/vgui/{trimmed}.vmt"))
}

/// `$baseTexture` of a VMT (any shader), tolerant of quoting and comments.
pub fn vmt_base_texture(vmt: &str) -> Option<String> {
    let t = hud_syntax::tree(vmt);
    for root in t.roots.iter().filter(|c| c.condition.as_deref().is_none_or(hud_syntax::condition_met)) {
        for c in root.children.iter().filter(|c| c.condition.as_deref().is_none_or(hud_syntax::condition_met)) {
            if c.key.eq_ignore_ascii_case("$basetexture") {
                return c.value.clone();
            }
        }
        // `insert`/`replace` patches and proxies blocks: search one level deeper
        for b in root.children.iter().filter(|c| c.is_block() && c.condition.as_deref().is_none_or(hud_syntax::condition_met)) {
            if let Some(v) = b
                .children
                .iter()
                .find(|c| c.key.eq_ignore_ascii_case("$basetexture") && c.condition.as_deref().is_none_or(hud_syntax::condition_met))
                .and_then(|c| c.value.clone())
            {
                return Some(v);
            }
        }
    }
    None
}

pub fn texture_for_material(vfs: &Vfs, material: &str) -> Option<String> {
    fn load(vfs: &Vfs, material: &str, seen: &mut std::collections::HashSet<String>) -> Option<String> {
        let material = normalize(material);
        if seen.len() >= 16 || !seen.insert(material.clone()) {
            return None;
        }
        let (text, _) = vfs.read_text(&material)?;
        if let Some(texture) = vmt_base_texture(&text) {
            let texture = normalize(&texture);
            let texture = texture.strip_prefix("materials/").unwrap_or(&texture).trim_end_matches(".vtf");
            return Some(format!("materials/{texture}.vtf"));
        }
        // Patch materials inherit their image from another VMT unless insert/replace supplies one.
        let tree = hud_syntax::tree(&text);
        let patch = tree.roots.iter().find(|r| r.key.eq_ignore_ascii_case("patch"))?;
        let include = normalize(patch.get("include")?);
        let include = include.strip_prefix("materials/").unwrap_or(&include).trim_end_matches(".vmt");
        load(vfs, &format!("materials/{include}.vmt"), seen)
    }
    load(vfs, material, &mut std::collections::HashSet::new())
}

/// Resolves an image name or a direct path (`materials/...vtf`/`.vmt`) and decodes it to a PNG data URL.
pub fn load(vfs: &Vfs, name: &str) -> Result<ImageResult, String> {
    let lower = normalize(name);
    let (material, texture) = if lower.ends_with(".vtf") {
        (None, lower)
    } else if lower.ends_with(".vmt") {
        let t = texture_for_material(vfs, &lower).ok_or_else(|| format!("{lower} has no $baseTexture"))?;
        (Some(lower), t)
    } else {
        let m = material_path(name);
        match texture_for_material(vfs, &m) {
            Some(t) => (Some(m), t),
            None => (None, normalize(&format!("materials/vgui/{}.vtf", name.trim()))),
        }
    };
    let (bytes, origin) = vfs.read_bytes(&texture).ok_or_else(|| format!("Texture not found: {texture}"))?;
    let info = vtf::info(&bytes)?;
    let img = vtf::decode(&bytes)?;
    let png = vtf::to_png(&img)?;
    Ok(ImageResult {
        data_url: format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png)),
        width: img.width,
        height: img.height,
        material,
        texture,
        origin: origin.label(),
        format: info.format,
    })
}

pub fn exists(vfs: &Vfs, name: &str) -> bool {
    let m = material_path(name);
    vfs.exists(&m) || vfs.exists(&normalize(&format!("materials/vgui/{}.vtf", name.trim())))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths() {
        assert_eq!(material_path("replay/thumbnails/star"), "materials/vgui/replay/thumbnails/star.vmt");
        assert_eq!(material_path("../hud/cart_point_blue"), "materials/hud/cart_point_blue.vmt");
        assert_eq!(vmt_base_texture("\"UnlitGeneric\"\n{\n\t\"$baseTexture\" \"vgui/x\"\n}"), Some("vgui/x".into()));
    }
}
