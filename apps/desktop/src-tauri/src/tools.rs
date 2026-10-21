//! External asset tools. Only known tool ids can be registered; registrations persist in settings.
//! Launches never go through a shell. Asset opening passes a single project file path;
//! marketplace extraction uses fixed 7-Zip command arguments in a private staging folder.
//!
//! * GUI tools: Crowbar (models), Blender, VTFEdit and VTFEdit Reloaded (textures).
//! * Command-line converters used by image import: VTFCmd (VTFLib's CLI, ships with VTFEdit/VTFEdit Reloaded) for
//!   compressed (DXT) VTFs, and ImageMagick for turning TGA/JPG/PSD/WebP… into PNG first.
//!   PNG → BGRA8888 VTF needs neither: HUD Studio does that itself.
use crate::session::SessionState;
use crate::StoreState;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::State;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalTool {
    id: String,
    name: String,
    /// "gui" or "cli".
    kind: String,
    about: String,
    path: Option<String>,
    available: bool,
    detected: bool,
    /// File extensions this tool can sensibly open.
    opens: Vec<String>,
}

struct ToolDef {
    id: &'static str,
    name: &'static str,
    kind: &'static str,
    about: &'static str,
    exe: &'static [&'static str],
    #[cfg_attr(not(windows), allow(dead_code))]
    /// Install folders under Program Files / LocalAppData\Programs (Windows); `*` matches one path segment.
    dirs: &'static [&'static str],
    opens: &'static [&'static str],
}

const TEXTURE_DIRS: &[&str] = &["VTFEdit", "Nem's Tools\\VTFEdit", "VTFEdit Reloaded", "VTFEdit-Reloaded", "VTFEdit Reloaded\\*", "VTFLib", "VTFLib\\*"];

const TOOLS: &[ToolDef] = &[
    ToolDef {
        id: "7zip",
        name: "7-Zip",
        kind: "cli",
        about: "Extracts marketplace 7z and RAR HUD downloads (ZIP works without it)",
        exe: &["7z.exe", "7zz.exe", "7zz", "7z"],
        dirs: &["7-Zip"],
        opens: &[],
    },
    ToolDef {
        id: "peazip-7z",
        name: "PeaZip (7-Zip backend)",
        kind: "cli",
        about: "Uses PeaZip's bundled 7z executable to extract marketplace HUDs",
        exe: &["7z.exe", "7zz", "7z"],
        dirs: &["PeaZip\\res\\bin\\7z", "PeaZip\\res\\bin\\7z\\*"],
        opens: &[],
    },
    ToolDef {
        id: "crowbar",
        name: "Crowbar",
        kind: "gui",
        about: "Decompile and compile Source models",
        exe: &["Crowbar.exe", "crowbar"],
        dirs: &["Crowbar"],
        opens: &["mdl", "qc"],
    },
    ToolDef {
        id: "blender",
        name: "Blender",
        kind: "gui",
        about: "3D modelling (with the Source Tools add-on)",
        exe: &["blender.exe", "blender"],
        dirs: &["Blender Foundation\\Blender", "Blender Foundation\\*"],
        opens: &["blend", "smd", "dmx"],
    },
    ToolDef {
        id: "vtfedit",
        name: "VTFEdit",
        kind: "gui",
        about: "View, edit and convert VTF textures",
        exe: &["VTFEdit.exe"],
        dirs: &["VTFEdit", "Nem's Tools\\VTFEdit"],
        opens: &["vtf", "vmt"],
    },
    ToolDef {
        id: "vtfedit-reloaded",
        name: "VTFEdit Reloaded",
        kind: "gui",
        about: "The maintained fork of VTFEdit",
        exe: &["VTFEdit.exe", "VTFEditReloaded.exe"],
        dirs: &["VTFEdit Reloaded", "VTFEdit-Reloaded", "VTFEdit Reloaded\\*"],
        opens: &["vtf", "vmt"],
    },
    ToolDef {
        id: "vtfcmd",
        name: "VTFCmd",
        kind: "cli",
        about: "Command-line VTF converter (for compressed DXT5 imports)",
        exe: &["VTFCmd.exe", "vtfcmd", "VTFCmd"],
        dirs: TEXTURE_DIRS,
        opens: &[],
    },
    ToolDef {
        id: "magick",
        name: "ImageMagick",
        kind: "cli",
        about: "Converts TGA, JPG, PSD, WebP and more to PNG before import",
        exe: &["magick.exe", "magick"],
        dirs: &["ImageMagick-*"],
        opens: &[],
    },
];

fn tool_def(id: &str) -> Result<&'static ToolDef, String> {
    TOOLS.iter().find(|t| t.id == id).ok_or_else(|| "Unknown tool id".into())
}

#[cfg(windows)]
fn expand(base: &Path, pattern: &str) -> Vec<PathBuf> {
    let mut found = vec![base.to_path_buf()];
    for seg in pattern.split('\\') {
        let mut next = Vec::new();
        for f in &found {
            if let Some(prefix) = seg.strip_suffix('*') {
                if let Ok(entries) = std::fs::read_dir(f) {
                    let mut v: Vec<PathBuf> = entries
                        .flatten()
                        .filter(|e| e.file_name().to_string_lossy().to_ascii_lowercase().starts_with(&prefix.to_ascii_lowercase()))
                        .map(|e| e.path())
                        .filter(|p| p.is_dir())
                        .collect();
                    v.sort();
                    v.reverse(); // newest version first
                    next.extend(v);
                }
            } else {
                next.push(f.join(seg));
            }
        }
        found = next;
    }
    found
}

fn search_tool(def: &ToolDef) -> Option<PathBuf> {
    if let Some(paths) = std::env::var_os("PATH") {
        for folder in std::env::split_paths(&paths) {
            for name in def.exe {
                let path = folder.join(name);
                if path.is_file() {
                    return dunce::canonicalize(&path).ok();
                }
            }
        }
    }
    #[cfg(windows)]
    {
        let mut roots: Vec<PathBuf> =
            ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"].iter().filter_map(|e| std::env::var_os(e)).map(PathBuf::from).collect();
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            roots.push(PathBuf::from(&local).join("Programs"));
            roots.push(PathBuf::from(local));
        }
        for root in roots {
            for d in def.dirs {
                for dir in expand(&root, d) {
                    for name in def.exe {
                        let p = dir.join(name);
                        if p.is_file() {
                            return dunce::canonicalize(&p).ok();
                        }
                    }
                }
            }
        }
    }
    #[cfg(not(windows))]
    {
        // ImageMagick 6 ships `convert` instead of `magick` (only safe to use outside Windows, where convert.exe is a disk tool)
        if def.id == "magick" {
            if let Some(paths) = std::env::var_os("PATH") {
                for folder in std::env::split_paths(&paths) {
                    let p = folder.join("convert");
                    if p.is_file() {
                        return Some(p);
                    }
                }
            }
        }
    }
    None
}

/// The registered or detected executable for a tool id.
pub fn tool_path(state: &State<'_, SessionState>, id: &str) -> Option<PathBuf> {
    let def = tool_def(id).ok()?;
    let registered = state.lock().ok()?.tools.get(id).filter(|p| p.is_file()).cloned();
    registered.or_else(|| search_tool(def))
}

fn resolve_tool(state: &State<'_, SessionState>, id: &str) -> Result<PathBuf, String> {
    tool_def(id)?;
    tool_path(state, id).ok_or_else(|| "Tool not registered or detected".into())
}

#[tauri::command]
pub async fn tools_list(state: State<'_, SessionState>) -> Result<Vec<ExternalTool>, String> {
    let registered: Vec<(String, PathBuf)> = state.lock().map_err(|_| "State poisoned")?.tools.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    Ok(TOOLS
        .iter()
        .map(|def| {
            let reg = registered.iter().find(|(k, _)| k == def.id).map(|(_, p)| p.clone()).filter(|p| p.is_file());
            let detected = if reg.is_none() { search_tool(def) } else { None };
            let is_detected = reg.is_none() && detected.is_some();
            let path = reg.or(detected);
            ExternalTool {
                id: def.id.to_string(),
                name: def.name.to_string(),
                kind: def.kind.to_string(),
                about: def.about.to_string(),
                available: path.is_some(),
                detected: is_detected,
                path: path.map(|p| p.to_string_lossy().to_string()),
                opens: def.opens.iter().map(|s| s.to_string()).collect(),
            }
        })
        .collect())
}

#[tauri::command]
pub fn tools_register(id: String, path: Option<String>, state: State<'_, SessionState>, store: State<'_, StoreState>) -> Result<(), String> {
    tool_def(&id)?;
    let mut s = state.lock().map_err(|_| "State poisoned")?;
    let mut st = store.lock().map_err(|_| "State poisoned")?;
    match path {
        Some(p) => {
            let real = dunce::canonicalize(Path::new(&p)).map_err(|e| format!("Cannot find executable: {e}"))?;
            if !real.is_file() {
                return Err("Tool path must be a file".into());
            }
            st.settings.tools.insert(id.clone(), real.to_string_lossy().to_string());
            s.tools.insert(id, real);
        }
        None => {
            st.settings.tools.remove(&id);
            s.tools.remove(&id);
        }
    }
    st.save()
}

#[tauri::command]
pub fn tools_launch(id: String, state: State<'_, SessionState>) -> Result<(), String> {
    let def = tool_def(&id)?;
    if def.kind == "cli" {
        return Err(format!("{} is a command-line tool; HUD Studio uses it for imports.", def.name));
    }
    let exe = resolve_tool(&state, &id)?;
    Command::new(exe).spawn().map_err(|e| format!("Could not launch tool: {e}"))?;
    Ok(())
}

/// Opens one project file in a registered tool (the file path is the only argument).
#[tauri::command]
pub fn tools_open_with(id: String, path: String, state: State<'_, SessionState>) -> Result<(), String> {
    let exe = resolve_tool(&state, &id)?;
    let file = state.lock().map_err(|_| "State poisoned")?.project_file(&path)?;
    if !file.is_file() {
        return Err("File not found in the HUD folder".into());
    }
    Command::new(exe).arg(file).spawn().map_err(|e| format!("Could not launch tool: {e}"))?;
    Ok(())
}
