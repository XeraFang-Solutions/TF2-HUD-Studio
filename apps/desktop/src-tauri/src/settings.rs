//! Versioned user settings stored as JSON in the app config folder (kept separate from HUD files).
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub version: u32,
    /// TF2's `tf` folder, used for stock files, fonts and materials.
    pub tf_dir: Option<String>,
    /// External tool id → executable path.
    pub tools: HashMap<String, String>,
    /// Most recent first.
    pub recent_projects: Vec<String>,
    /// Back up files before overwriting them on save.
    pub backups: bool,
    /// External text editor for "Open in editor".
    pub editor: crate::editors::EditorSetting,
    /// External image editor for "Edit image".
    pub image_editor: crate::editors::EditorSetting,
    /// Platform used to evaluate [$WIN32]/[$LINUX]/[$OSX] conditions in HUD files.
    pub platform: hud_syntax::Platform,
    /// Free-form UI preferences owned by the frontend (layout, resolution, toggles).
    pub ui: serde_json::Value,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            version: 1,
            tf_dir: None,
            tools: HashMap::new(),
            recent_projects: Vec::new(),
            backups: true,
            editor: Default::default(),
            image_editor: Default::default(),
            platform: hud_syntax::Platform::Windows,
            ui: serde_json::Value::Object(Default::default()),
        }
    }
}

pub struct Store {
    pub path: PathBuf,
    pub settings: Settings,
}

impl Store {
    pub fn load(dir: &Path) -> Store {
        let path = dir.join("settings.json");
        let settings = fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        Store { path, settings }
    }
    pub fn save(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_string_pretty(&self.settings).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        let _ = fs::remove_file(&self.path);
        fs::rename(&tmp, &self.path).map_err(|e| e.to_string())
    }
    pub fn push_recent(&mut self, project: &str) {
        self.settings.recent_projects.retain(|p| !p.eq_ignore_ascii_case(project));
        self.settings.recent_projects.insert(0, project.to_string());
        self.settings.recent_projects.truncate(12);
    }
}
