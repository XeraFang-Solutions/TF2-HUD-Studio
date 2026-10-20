//! Optional external asset tools. Only known tool ids can be registered; registrations persist in settings.
//! Launches never go through a shell and only ever pass a single project file path as the argument.
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
    path: Option<String>,
    available: bool,
    detected: bool,
    /// File extensions this tool can sensibly open.
    opens: Vec<String>,
}

// Text and image editors live in editors.rs (chosen from a dropdown in Settings); these are the other asset tools.
const TOOL_CATALOG: [(&str, &str, &[&str], &[&str]); 2] =
    [("crowbar", "Crowbar", &["Crowbar.exe", "crowbar"], &["mdl", "qc"]), ("blender", "Blender", &["blender.exe", "blender"], &["blend", "smd", "dmx"])];

fn tool_def(id: &str) -> Result<(&'static str, &'static [&'static str], &'static [&'static str]), String> {
    TOOL_CATALOG.iter().find(|(key, ..)| *key == id).map(|(_, name, names, opens)| (*name, *names, *opens)).ok_or_else(|| "Unknown tool id".into())
}

fn search_tool(names: &[&str]) -> Option<PathBuf> {
    if let Some(paths) = std::env::var_os("PATH") {
        for folder in std::env::split_paths(&paths) {
            for name in names {
                let path = folder.join(name);
                if path.is_file() {
                    return dunce::canonicalize(&path).ok();
                }
            }
        }
    }
    #[cfg(windows)]
    {
        let mut roots: Vec<PathBuf> = ["ProgramFiles", "ProgramFiles(x86)"].iter().filter_map(|e| std::env::var_os(e)).map(PathBuf::from).collect();
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            roots.push(PathBuf::from(local).join("Programs"));
        }
        for dir in roots {
            for sub in ["Crowbar", "Blender Foundation\\Blender"] {
                for name in names {
                    let path = dir.join(sub).join(name);
                    if path.is_file() {
                        return dunce::canonicalize(&path).ok();
                    }
                }
            }
            // Blender installs into versioned folders: Blender Foundation\Blender 4.2\blender.exe
            if let Ok(entries) = std::fs::read_dir(dir.join("Blender Foundation")) {
                for e in entries.flatten() {
                    for name in names {
                        let p = e.path().join(name);
                        if p.is_file() {
                            return dunce::canonicalize(&p).ok();
                        }
                    }
                }
            }
        }
    }
    None
}

fn resolve_tool(state: &State<'_, SessionState>, id: &str) -> Result<PathBuf, String> {
    let (_, names, _) = tool_def(id)?;
    let s = state.lock().map_err(|_| "State poisoned")?;
    s.tools.get(id).filter(|p| p.is_file()).cloned().or_else(|| search_tool(names)).ok_or_else(|| "Tool not registered or detected".into())
}

#[tauri::command]
pub async fn tools_list(state: State<'_, SessionState>) -> Result<Vec<ExternalTool>, String> {
    let registered: Vec<(String, PathBuf)> = state.lock().map_err(|_| "State poisoned")?.tools.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    Ok(TOOL_CATALOG
        .iter()
        .map(|(id, name, names, opens)| {
            let reg = registered.iter().find(|(k, _)| k == id).map(|(_, p)| p.clone()).filter(|p| p.is_file());
            let detected = if reg.is_none() { search_tool(names) } else { None };
            let is_detected = reg.is_none() && detected.is_some();
            let path = reg.or(detected);
            ExternalTool {
                id: id.to_string(),
                name: name.to_string(),
                available: path.is_some(),
                detected: is_detected,
                path: path.map(|p| p.to_string_lossy().to_string()),
                opens: opens.iter().map(|s| s.to_string()).collect(),
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
