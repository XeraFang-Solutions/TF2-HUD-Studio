//! Turning images into HUD materials: import a PNG/TGA/JPG/… as `materials/vgui/<name>.vtf` + `.vmt`, and write
//! an edited PNG copy back into its VTF.
//!
//! PNG → BGRA8888 (lossless, no mipmaps) is built in. Other formats go through ImageMagick first, and compressed
//! DXT5 output goes through VTFCmd, when those tools are installed (see the Tools panel).
use crate::session::SessionState;
use crate::tools::tool_path;
use hud_core::vtf;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::State;

type Res<T> = Result<T, String>;

/// Where "Edit a PNG copy" puts the PNG for a texture path (one file per texture, so names never collide).
pub fn png_copy_path(texture: &str) -> PathBuf {
    let flat: String = texture
        .trim_start_matches(['/', '\\'])
        .replace(['/', '\\'], "__")
        .chars()
        .map(|c| if c.is_alphanumeric() || "._-".contains(c) { c } else { '_' })
        .collect();
    let flat = flat.strip_suffix(".vtf").unwrap_or(&flat).to_string();
    std::env::temp_dir().join("tf2-hud-studio").join("png").join(format!("{flat}.png"))
}

fn hidden_command(exe: &Path) -> Command {
    #[allow(unused_mut)]
    let mut c = Command::new(exe);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    c
}

fn run(mut c: Command, what: &str) -> Res<()> {
    let out = c.output().map_err(|e| format!("Couldn't run {what}: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let msg = if err.trim().is_empty() { String::from_utf8_lossy(&out.stdout).to_string() } else { err.to_string() };
        return Err(format!("{what} failed: {}", msg.trim()));
    }
    Ok(())
}

fn tmp_dir() -> Res<PathBuf> {
    let d = std::env::temp_dir().join("tf2-hud-studio").join(format!("convert-{}", std::process::id()));
    fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    Ok(d)
}

/// Loads any image as RGBA, using ImageMagick for non-PNG files.
fn load_rgba(src: &Path, state: &State<'_, SessionState>) -> Res<vtf::Image> {
    let ext = src.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    if ext == "png" {
        return vtf::from_png(&fs::read(src).map_err(|e| format!("{}: {e}", src.display()))?);
    }
    if ext == "vtf" {
        return vtf::decode(&fs::read(src).map_err(|e| e.to_string())?);
    }
    let magick = tool_path(state, "magick").ok_or_else(|| format!("To import .{ext} files, install ImageMagick (or save the image as PNG first)."))?;
    let out = tmp_dir()?.join("import.png");
    let mut c = hidden_command(&magick);
    // "convert" (ImageMagick 6) and "magick" (7) take the same arguments here; [0] = first layer/frame
    c.arg(format!("{}[0]", src.display())).arg(format!("PNG32:{}", out.display()));
    run(c, "ImageMagick")?;
    let img = vtf::from_png(&fs::read(&out).map_err(|e| e.to_string())?);
    let _ = fs::remove_file(&out);
    img
}

/// Encodes to VTF bytes: built-in BGRA8888, or DXT5 through VTFCmd.
fn to_vtf(img: &vtf::Image, compress: bool, state: &State<'_, SessionState>) -> Res<Vec<u8>> {
    if !compress {
        return vtf::encode_bgra8888(img, vtf::HUD_FLAGS);
    }
    let vtfcmd = tool_path(state, "vtfcmd")
        .ok_or("Compressed (DXT5) textures need VTFCmd, which comes with VTFEdit and VTFEdit Reloaded. Set it up in Tools, or import uncompressed.")?;
    let dir = tmp_dir()?;
    let png = dir.join("hudstudio_import.png");
    fs::write(&png, vtf::to_png(img)?).map_err(|e| e.to_string())?;
    let mut c = hidden_command(&vtfcmd);
    c.args(["-file"]).arg(&png).args(["-output"]).arg(&dir).args([
        "-format",
        "DXT5",
        "-alphaformat",
        "DXT5",
        "-nomipmaps",
        "-flag",
        "CLAMPS",
        "-flag",
        "CLAMPT",
        "-flag",
        "NOLOD",
        "-silent",
    ]);
    run(c, "VTFCmd")?;
    let out = dir.join("hudstudio_import.vtf");
    let bytes = fs::read(&out).map_err(|_| "VTFCmd didn't produce a VTF".to_string())?;
    let _ = fs::remove_file(&out);
    let _ = fs::remove_file(&png);
    Ok(bytes)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOptions {
    /// Image name relative to materials/vgui, e.g. "replay/thumbnails/myicon".
    name: String,
    /// DXT5 through VTFCmd instead of lossless BGRA8888.
    #[serde(default)]
    compress: bool,
    /// Scale up to the next power of two on each side.
    #[serde(default)]
    power_of_two: bool,
    /// Replace an existing texture with the same name.
    #[serde(default)]
    overwrite: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    /// Value for an element's `image` property.
    image: String,
    texture: String,
    material: Option<String>,
    width: u32,
    height: u32,
}

fn clean_name(name: &str) -> Res<String> {
    let n = name.trim().replace('\\', "/").trim_matches('/').to_ascii_lowercase();
    let n = n.strip_suffix(".vtf").or(n.strip_suffix(".vmt")).unwrap_or(&n).to_string();
    if n.is_empty() || n.split('/').any(|s| s.is_empty() || s == "." || s == "..") || !n.chars().all(|c| c.is_ascii_alphanumeric() || "_-/.".contains(c)) {
        return Err("Use a name made of letters, numbers, _ - and / (folders), e.g. replay/thumbnails/myicon".into());
    }
    Ok(n)
}

/// Imports an image file into the open HUD as materials/vgui/<name>.vtf and a matching .vmt.
#[tauri::command]
pub async fn image_import(source: String, options: ImportOptions, state: State<'_, SessionState>) -> Res<ImportResult> {
    let name = clean_name(&options.name)?;
    let rel_vtf = format!("materials/vgui/{name}.vtf");
    let rel_vmt = format!("materials/vgui/{name}.vmt");
    let (vtf_path, vmt_path) = {
        let s = state.lock().map_err(|_| "State poisoned")?;
        (s.project_file(&rel_vtf)?, s.project_file(&rel_vmt)?)
    };
    if vtf_path.exists() && !options.overwrite {
        return Err(format!("{rel_vtf} already exists"));
    }
    let mut img = load_rgba(Path::new(&source), &state)?;
    if options.power_of_two {
        img = vtf::resize(&img, vtf::next_pow2(img.width), vtf::next_pow2(img.height));
    }
    let bytes = to_vtf(&img, options.compress, &state)?;
    fs::create_dir_all(vtf_path.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(&vtf_path, bytes).map_err(|e| e.to_string())?;
    let material = if vmt_path.exists() {
        None
    } else {
        fs::write(&vmt_path, vtf::hud_vmt(&format!("vgui/{name}"))).map_err(|e| e.to_string())?;
        Some(rel_vmt)
    };
    let mut s = state.lock().map_err(|_| "State poisoned")?;
    let root = s.root()?.clone();
    if let Ok(files) = hud_core::project::scan(&root) {
        s.set_files(files);
    }
    s.images.clear();
    Ok(ImportResult { image: name, texture: rel_vtf, material, width: img.width, height: img.height })
}

/// Writes the edited PNG copy of `texture` back into the HUD's VTF (creating an override for stock textures).
#[tauri::command]
pub async fn image_png_back(texture: String, compress: bool, state: State<'_, SessionState>) -> Res<String> {
    let png = png_copy_path(&texture);
    if !png.is_file() {
        return Err("There's no PNG copy of that texture. Use “Edit a PNG copy” first.".into());
    }
    let img = vtf::from_png(&fs::read(&png).map_err(|e| e.to_string())?)?;
    let bytes = to_vtf(&img, compress, &state)?;
    let rel = texture.trim_start_matches(['/', '\\']).replace('\\', "/");
    let mut s = state.lock().map_err(|_| "State poisoned")?;
    let out = s.project_file(&rel)?;
    fs::create_dir_all(out.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(&out, bytes).map_err(|e| e.to_string())?;
    let root = s.root()?.clone();
    if let Ok(files) = hud_core::project::scan(&root) {
        s.set_files(files);
    }
    s.images.clear();
    Ok(rel)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_and_paths() {
        assert_eq!(clean_name("Replay/Thumbnails/MyIcon.vtf").unwrap(), "replay/thumbnails/myicon");
        assert!(clean_name("../x").is_err());
        assert!(clean_name("a b").is_err());
        let p = png_copy_path("materials/vgui/replay/x.vtf");
        assert!(p.ends_with("png/materials__vgui__replay__x.png"));
    }
}
