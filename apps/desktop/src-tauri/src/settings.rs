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
    /// Folder the user keeps their HUDs in.
    pub hud_library: Option<String>,
    /// How HUDs are installed into tf/custom: a link (junction/symlink) or a copy.
    pub install_mode: hud_core::library::InstallMode,
    /// Launching TF2 to test a HUD.
    pub launch: LaunchSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LaunchSettings {
    /// "64" (standard) or "32" (legacy, Windows only).
    pub bits: String,
    /// "direct" starts the game executable (lets you pick 32/64-bit); "steam" asks Steam to start it.
    pub method: String,
    /// Window size for itemtest; detected from the display on first launch when unset.
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// Load straight into itemtest (with sv_lan 1 and a borderless window).
    pub itemtest: bool,
    /// Open TF2's remote console (-usercon) on localhost with a password generated per launch.
    pub rcon: bool,
    /// Have the game write console.log (-condebug) and show it in HUD Studio.
    pub console_log: bool,
    /// Run hud_reloadscheme in the game after saving.
    pub reload_on_save: bool,
    /// Extra launch options, e.g. `-novid +exec myconfig`.
    pub extra_args: String,
}

impl Default for LaunchSettings {
    fn default() -> Self {
        LaunchSettings {
            bits: "64".into(),
            method: if cfg!(windows) { "direct".into() } else { "steam".into() },
            width: None,
            height: None,
            itemtest: true,
            rcon: true,
            console_log: true,
            reload_on_save: true,
            extra_args: String::new(),
        }
    }
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
            hud_library: None,
            install_mode: Default::default(),
            launch: Default::default(),
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
