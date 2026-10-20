//! External apps: catalogs of known text editors and image editors per platform, detection, and launching
//! (text files at a line, images as-is or as a PNG copy).
//! Arguments come from the catalog (or the user's own custom template); nothing goes through a shell except the
//! macOS terminal hand-off, which quotes every argument.
use crate::session::SessionState;
use crate::StoreState;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::State;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Gui,
    /// Runs inside a terminal window (vim, nano, helix…).
    Terminal,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    Text,
    Image,
}

struct Editor {
    category: Category,
    id: &'static str,
    name: &'static str,
    kind: Kind,
    /// Executable names searched on PATH (Linux/macOS) — and on Windows when they end in .exe.
    commands: &'static [&'static str],
    /// Windows executables relative to Program Files / Program Files (x86) / %LOCALAPPDATA%\Programs. `*` in a path
    /// segment is a wildcard (versioned installs such as `JetBrains\*\bin\idea64.exe`).
    windows: &'static [&'static str],
    /// macOS app bundle (relative to /Applications, may contain `*` like `Adobe Photoshop */Adobe Photoshop *.app`)
    /// and, if it has one, its command-line tool inside the bundle.
    mac_app: Option<(&'static str, Option<&'static str>)>,
    /// Arguments to open `{file}` at `{line}`; empty when the editor has no such option.
    line_args: &'static [&'static str],
}

const FILE: &[&str] = &["{file}"];

macro_rules! ed {
    ($id:literal, $name:literal, $kind:expr, $cmds:expr, $win:expr, $mac:expr, $line:expr) => {
        Editor { category: Category::Text, id: $id, name: $name, kind: $kind, commands: $cmds, windows: $win, mac_app: $mac, line_args: $line }
    };
}
macro_rules! img {
    ($id:literal, $name:literal, $cmds:expr, $win:expr, $mac:expr) => {
        Editor { category: Category::Image, id: $id, name: $name, kind: Kind::Gui, commands: $cmds, windows: $win, mac_app: $mac, line_args: &[] }
    };
}

use Kind::{Gui, Terminal};

const GOTO: &[&str] = &["-g", "{file}:{line}"];
const COLON: &[&str] = &["{file}:{line}"];
const PLUS: &[&str] = &["+{line}", "{file}"];
const DASH_L: &[&str] = &["-l", "{line}", "{file}"];
const JB: &[&str] = &["--line", "{line}", "{file}"];

#[rustfmt::skip]
static CATALOG: &[Editor] = &[
    ed!("vscode", "Visual Studio Code", Gui, &["code"], &["Microsoft VS Code\\Code.exe"], Some(("Visual Studio Code.app", Some("Contents/Resources/app/bin/code"))), GOTO),
    ed!("vscode-insiders", "VS Code Insiders", Gui, &["code-insiders"], &["Microsoft VS Code Insiders\\Code - Insiders.exe"], Some(("Visual Studio Code - Insiders.app", Some("Contents/Resources/app/bin/code-insiders"))), GOTO),
    ed!("vscodium", "VSCodium", Gui, &["codium"], &["VSCodium\\VSCodium.exe"], Some(("VSCodium.app", Some("Contents/Resources/app/bin/codium"))), GOTO),
    ed!("cursor", "Cursor", Gui, &["cursor"], &["cursor\\Cursor.exe"], Some(("Cursor.app", Some("Contents/Resources/app/bin/cursor"))), GOTO),
    ed!("windsurf", "Windsurf", Gui, &["windsurf"], &["Windsurf\\Windsurf.exe"], Some(("Windsurf.app", Some("Contents/Resources/app/bin/windsurf"))), GOTO),
    ed!("zed", "Zed", Gui, &["zed", "zeditor"], &["Zed\\Zed.exe"], Some(("Zed.app", Some("Contents/MacOS/cli"))), COLON),
    ed!("sublime", "Sublime Text", Gui, &["subl"], &["Sublime Text\\sublime_text.exe", "Sublime Text 3\\sublime_text.exe"], Some(("Sublime Text.app", Some("Contents/SharedSupport/bin/subl"))), COLON),
    ed!("notepadpp", "Notepad++", Gui, &["notepad++.exe"], &["Notepad++\\notepad++.exe"], None, &["-n{line}", "{file}"]),
    ed!("notepad3", "Notepad3", Gui, &["Notepad3.exe"], &["Notepad3\\Notepad3.exe"], None, &["/g", "{line}", "{file}"]),
    ed!("notepad", "Notepad (Windows)", Gui, &["notepad.exe"], &[], None, &[]),
    ed!("emeditor", "EmEditor", Gui, &["EmEditor.exe"], &["EmEditor\\EmEditor.exe"], None, &["/l", "{line}", "{file}"]),
    ed!("ultraedit", "UltraEdit", Gui, &["uedit64.exe"], &["IDM Computer Solutions\\UltraEdit\\uedit64.exe"], None, &["{file}/{line}"]),
    ed!("pulsar", "Pulsar", Gui, &["pulsar"], &["Pulsar\\Pulsar.exe"], Some(("Pulsar.app", Some("Contents/Resources/pulsar.sh"))), COLON),
    ed!("lapce", "Lapce", Gui, &["lapce"], &["Lapce\\lapce.exe"], Some(("Lapce.app", None)), &[]),
    ed!("kate", "Kate", Gui, &["kate"], &["Kate\\bin\\kate.exe"], Some(("kate.app", None)), DASH_L),
    ed!("kwrite", "KWrite", Gui, &["kwrite"], &[], None, DASH_L),
    ed!("gedit", "gedit", Gui, &["gedit"], &[], None, PLUS),
    ed!("gnome-text-editor", "GNOME Text Editor", Gui, &["gnome-text-editor"], &[], None, &[]),
    ed!("mousepad", "Mousepad", Gui, &["mousepad"], &[], None, &[]),
    ed!("pluma", "Pluma", Gui, &["pluma"], &[], None, PLUS),
    ed!("xed", "Xed", Gui, &["xed"], &[], None, PLUS),
    ed!("featherpad", "FeatherPad", Gui, &["featherpad"], &[], None, &[]),
    ed!("geany", "Geany", Gui, &["geany"], &["Geany\\bin\\geany.exe"], Some(("Geany.app", None)), DASH_L),
    ed!("emacs", "Emacs", Gui, &["emacs"], &["Emacs\\*\\bin\\runemacs.exe", "Emacs\\bin\\runemacs.exe"], Some(("Emacs.app", Some("Contents/MacOS/Emacs"))), PLUS),
    ed!("gvim", "gVim", Gui, &["gvim"], &["Vim\\*\\gvim.exe"], Some(("MacVim.app", Some("Contents/bin/mvim"))), PLUS),
    ed!("neovide", "Neovide", Gui, &["neovide"], &["Neovide\\neovide.exe"], Some(("Neovide.app", Some("Contents/MacOS/neovide"))), &[]),
    ed!("idea", "IntelliJ IDEA", Gui, &["idea"], &["JetBrains\\*\\bin\\idea64.exe"], Some(("IntelliJ IDEA.app", Some("Contents/MacOS/idea"))), JB),
    ed!("webstorm", "WebStorm", Gui, &["webstorm"], &["JetBrains\\*\\bin\\webstorm64.exe"], Some(("WebStorm.app", Some("Contents/MacOS/webstorm"))), JB),
    ed!("rustrover", "RustRover", Gui, &["rustrover"], &["JetBrains\\*\\bin\\rustrover64.exe"], Some(("RustRover.app", Some("Contents/MacOS/rustrover"))), JB),
    ed!("pycharm", "PyCharm", Gui, &["pycharm"], &["JetBrains\\*\\bin\\pycharm64.exe"], Some(("PyCharm.app", Some("Contents/MacOS/pycharm"))), JB),
    ed!("clion", "CLion", Gui, &["clion"], &["JetBrains\\*\\bin\\clion64.exe"], Some(("CLion.app", Some("Contents/MacOS/clion"))), JB),
    ed!("rider", "Rider", Gui, &["rider"], &["JetBrains\\*\\bin\\rider64.exe"], Some(("Rider.app", Some("Contents/MacOS/rider"))), JB),
    ed!("fleet", "Fleet", Gui, &["fleet"], &[], Some(("Fleet.app", None)), &[]),
    ed!("textmate", "TextMate", Gui, &["mate"], &[], Some(("TextMate.app", Some("Contents/MacOS/mate"))), DASH_L),
    ed!("bbedit", "BBEdit", Gui, &["bbedit"], &[], Some(("BBEdit.app", Some("Contents/Helpers/bbedit_tool"))), PLUS),
    ed!("coteditor", "CotEditor", Gui, &["cot"], &[], Some(("CotEditor.app", Some("Contents/SharedSupport/bin/cot"))), DASH_L),
    ed!("nova", "Nova", Gui, &["nova"], &[], Some(("Nova.app", None)), &[]),
    ed!("textedit", "TextEdit (macOS)", Gui, &[], &[], Some(("TextEdit.app", None)), &[]),
    ed!("vim", "Vim (terminal)", Terminal, &["vim", "vim.exe"], &["Vim\\*\\vim.exe"], None, PLUS),
    ed!("nvim", "Neovim (terminal)", Terminal, &["nvim", "nvim.exe"], &["Neovim\\bin\\nvim.exe"], None, PLUS),
    ed!("nano", "nano (terminal)", Terminal, &["nano", "nano.exe"], &[], None, PLUS),
    ed!("micro", "micro (terminal)", Terminal, &["micro", "micro.exe"], &[], None, &[]),
    ed!("helix", "Helix (terminal)", Terminal, &["hx", "hx.exe", "helix"], &[], None, COLON),
    ed!("kakoune", "Kakoune (terminal)", Terminal, &["kak"], &[], None, PLUS),
    ed!("emacs-nw", "Emacs (terminal)", Terminal, &["emacs", "emacs.exe"], &[], None, &["-nw", "+{line}", "{file}"]),

    img!("gimp", "GIMP", &["gimp", "gimp-3.0", "gimp-2.10"], &["GIMP 3\\bin\\gimp-3.0.exe", "GIMP 3\\bin\\gimp-3.exe", "GIMP 3\\bin\\gimp.exe", "GIMP 2\\bin\\gimp-2.10.exe"], Some(("GIMP*.app", None))),
    img!("photoshop", "Adobe Photoshop", &[], &["Adobe\\Adobe Photoshop*\\Photoshop.exe"], Some(("Adobe Photoshop*/Adobe Photoshop*.app", None))),
    img!("photoshop-elements", "Photoshop Elements", &[], &["Adobe\\Photoshop Elements*\\PhotoshopElementsEditor.exe"], Some(("Adobe Photoshop Elements*/Adobe Photoshop Elements*Editor.app", None))),
    img!("krita", "Krita", &["krita"], &["Krita (x64)\\bin\\krita.exe", "Krita\\bin\\krita.exe"], Some(("krita.app", None))),
    img!("paintdotnet", "Paint.NET", &[], &["paint.net\\paintdotnet.exe"], None),
    img!("affinity", "Affinity", &[], &["Affinity\\Affinity\\Affinity.exe"], Some(("Affinity.app", None))),
    img!("affinity-photo-2", "Affinity Photo 2", &[], &["Affinity\\Photo 2\\Photo.exe"], Some(("Affinity Photo 2.app", None))),
    img!("affinity-photo", "Affinity Photo", &[], &["Affinity\\Affinity Photo\\Photo.exe", "Affinity\\Photo\\Photo.exe"], Some(("Affinity Photo.app", None))),
    img!("pixelmator", "Pixelmator Pro", &[], &[], Some(("Pixelmator Pro.app", None))),
    img!("acorn", "Acorn", &[], &[], Some(("Acorn.app", None))),
    img!("preview", "Preview (macOS)", &[], &[], Some(("Preview.app", None))),
    img!("paintshop", "Corel PaintShop Pro", &[], &["Corel\\*PaintShop Pro*\\Corel PaintShop Pro.exe"], None),
    img!("clipstudio", "Clip Studio Paint", &[], &["CELSYS\\*\\CLIP STUDIO PAINT\\CLIPStudioPaint.exe"], Some(("CLIP STUDIO*/CLIP STUDIO PAINT.app", None))),
    img!("aseprite", "Aseprite", &["aseprite"], &["Aseprite\\Aseprite.exe", "Steam\\steamapps\\common\\Aseprite\\Aseprite.exe"], Some(("Aseprite.app", None))),
    img!("pinta", "Pinta", &["pinta"], &["Pinta\\Pinta.exe"], Some(("Pinta.app", None))),
    img!("mspaint", "Paint (Windows)", &["mspaint.exe"], &[], None),
    img!("inkscape", "Inkscape", &["inkscape"], &["Inkscape\\bin\\inkscape.exe"], Some(("Inkscape.app", None))),
    img!("photofiltre", "PhotoFiltre", &[], &["PhotoFiltre*\\PhotoFiltre*.exe"], None),
    img!("irfanview", "IrfanView", &[], &["IrfanView\\i_view64.exe", "IrfanView\\i_view32.exe"], None),
    img!("xnviewmp", "XnView MP", &["xnviewmp"], &["XnViewMP\\xnviewmp.exe"], Some(("XnViewMP.app", None))),
    img!("mypaint", "MyPaint", &["mypaint"], &["MyPaint\\bin\\mypaint.exe"], None),
    img!("kolourpaint", "KolourPaint", &["kolourpaint"], &[], None),
    img!("vtfedit", "VTFEdit", &["VTFEdit.exe"], &["VTFEdit\\VTFEdit.exe", "Nem's Tools\\VTFEdit\\VTFEdit.exe"], None),
    img!("vtfedit-reloaded", "VTFEdit Reloaded", &["VTFEdit-Reloaded.exe", "VTFEdit Reloaded.exe"], &["VTFEdit Reloaded\\VTFEdit Reloaded.exe", "VTFEdit Reloaded\\VTFEdit-Reloaded.exe", "VTFEdit Reloaded\\VTFEdit.exe"], None),
];

/// The user's choice, stored in settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EditorSetting {
    /// "system", a catalog id, or "custom".
    pub id: String,
    /// Executable override (required for "custom", optional for catalog entries that weren't detected).
    pub path: Option<String>,
    /// Custom argument template, e.g. `-g "{file}:{line}"`. Default `{file}`.
    pub args: Option<String>,
}

impl Default for EditorSetting {
    fn default() -> Self {
        EditorSetting { id: "system".into(), path: None, args: None }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorInfo {
    id: String,
    name: String,
    detected: bool,
    path: Option<String>,
    terminal: bool,
    opens_at_line: bool,
}

/// How a detected editor is started.
enum Launch {
    Exe(PathBuf),
    /// macOS app bundle without a usable command-line tool: `open -a <app> <file>`.
    MacApp(PathBuf),
}

fn on_path(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths).map(|d| d.join(name)).find(|p| p.is_file())
}

#[cfg(windows)]
fn windows_roots() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"].iter().filter_map(std::env::var_os).map(PathBuf::from).collect();
    if let Some(l) = std::env::var_os("LOCALAPPDATA") {
        v.push(PathBuf::from(&l).join("Programs"));
        v.push(PathBuf::from(l));
    }
    v
}

/// `*` wildcard match for one path segment (case-insensitive).
fn seg_match(pat: &str, name: &str) -> bool {
    let (p, n) = (pat.to_ascii_lowercase(), name.to_ascii_lowercase());
    let parts: Vec<&str> = p.split('*').collect();
    if parts.len() == 1 {
        return p == n;
    }
    let mut rest = n.as_str();
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            match rest.strip_prefix(part) {
                Some(r) => rest = r,
                None => return false,
            }
        } else if i == parts.len() - 1 {
            return rest.ends_with(part);
        } else {
            match rest.find(part) {
                Some(at) => rest = &rest[at + part.len()..],
                None => return false,
            }
        }
    }
    true
}

/// Paths under `root` matching `pattern` (segments split on `sep`, `*` wildcards allowed), newest-looking first.
#[cfg_attr(all(unix, not(target_os = "macos")), allow(dead_code))]
fn glob(root: &Path, pattern: &str, sep: char) -> Vec<PathBuf> {
    let mut current = vec![root.to_path_buf()];
    for part in pattern.split(sep) {
        let mut next = Vec::new();
        for dir in &current {
            if part.contains('*') {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    let mut subs: Vec<PathBuf> =
                        entries.flatten().map(|e| e.path()).filter(|p| p.file_name().is_some_and(|n| seg_match(part, &n.to_string_lossy()))).collect();
                    subs.sort();
                    subs.reverse(); // "2025" before "2024"
                    next.extend(subs);
                }
            } else {
                next.push(dir.join(part));
            }
        }
        current = next;
    }
    current.into_iter().filter(|p| p.exists()).collect()
}

fn detect(e: &Editor) -> Option<Launch> {
    #[cfg(windows)]
    {
        for rel in e.windows {
            for root in windows_roots() {
                if let Some(p) = glob(&root, rel, '\\').into_iter().find(|p| p.is_file()) {
                    return Some(Launch::Exe(p));
                }
            }
        }
        for c in e.commands {
            let name = if c.contains('.') { c.to_string() } else { format!("{c}.exe") };
            if let Some(p) = on_path(&name) {
                return Some(Launch::Exe(p));
            }
        }
        None
    }
    #[cfg(target_os = "macos")]
    {
        if let Some((app, cli)) = e.mac_app {
            let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
            for dir in [PathBuf::from("/Applications"), home.join("Applications"), PathBuf::from("/System/Applications")] {
                if let Some(bundle) = glob(&dir, app, '/').into_iter().find(|p| p.is_dir()) {
                    if let Some(cli) = cli {
                        let tool = bundle.join(cli);
                        if tool.is_file() {
                            return Some(Launch::Exe(tool));
                        }
                    }
                    if let Some(p) = e.commands.iter().find_map(|c| on_path(c)) {
                        return Some(Launch::Exe(p));
                    }
                    return Some(Launch::MacApp(bundle));
                }
            }
        }
        e.commands.iter().filter(|c| !c.ends_with(".exe")).find_map(|c| on_path(c)).map(Launch::Exe)
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = (e.windows, e.mac_app);
        e.commands.iter().filter(|c| !c.ends_with(".exe")).find_map(|c| on_path(c)).map(Launch::Exe)
    }
}

fn fill(template: &[&str], file: &Path, line: u32) -> Vec<String> {
    let f = file.to_string_lossy();
    template.iter().map(|a| a.replace("{file}", &f).replace("{line}", &line.to_string())).collect()
}

/// Splits a user template like `-g "{file}:{line}" --wait` into arguments (double quotes group words).
fn split_template(t: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut any = false;
    for c in t.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            c if c.is_whitespace() && !quoted => {
                if any || !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
                any = false;
            }
            c => cur.push(c),
        }
    }
    if any || !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn spawn(exe: &Path, args: &[String], terminal: bool) -> Result<(), String> {
    if !terminal {
        return Command::new(exe).args(args).spawn().map(|_| ()).map_err(|e| format!("Could not start {}: {e}", exe.display()));
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        Command::new(exe).args(args).creation_flags(CREATE_NEW_CONSOLE).spawn().map(|_| ()).map_err(|e| e.to_string())
    }
    #[cfg(target_os = "macos")]
    {
        let quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
        let shell = std::iter::once(quote(&exe.to_string_lossy())).chain(args.iter().map(|a| quote(a))).collect::<Vec<_>>().join(" ");
        let script = format!(
            "tell application \"Terminal\" to do script \"{}\"\ntell application \"Terminal\" to activate",
            shell.replace('\\', "\\\\").replace('"', "\\\"")
        );
        Command::new("osascript").arg("-e").arg(script).spawn().map(|_| ()).map_err(|e| e.to_string())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let mut candidates: Vec<(String, Vec<&str>)> = Vec::new();
        if let Ok(t) = std::env::var("TERMINAL") {
            candidates.push((t, vec!["-e"]));
        }
        for (t, sep) in [
            ("x-terminal-emulator", vec!["-e"]),
            ("gnome-terminal", vec!["--"]),
            ("konsole", vec!["-e"]),
            ("kitty", vec![]),
            ("alacritty", vec!["-e"]),
            ("wezterm", vec!["start", "--"]),
            ("foot", vec![]),
            ("xfce4-terminal", vec!["-x"]),
            ("tilix", vec!["-e"]),
            ("xterm", vec!["-e"]),
        ] {
            candidates.push((t.to_string(), sep));
        }
        for (term, sep) in candidates {
            if let Some(t) = on_path(&term).or_else(|| Some(PathBuf::from(&term)).filter(|p| p.is_file())) {
                return Command::new(t).args(sep).arg(exe).args(args).spawn().map(|_| ()).map_err(|e| e.to_string());
            }
        }
        Err("No terminal emulator found. Set the TERMINAL environment variable or choose a graphical editor.".into())
    }
}

#[tauri::command]
pub async fn editors_list(category: Category) -> Result<Vec<EditorInfo>, String> {
    let mut out: Vec<EditorInfo> = CATALOG
        .iter()
        .filter(|e| e.category == category)
        .filter(|e| match std::env::consts::OS {
            "windows" => !e.windows.is_empty() || e.commands.iter().any(|c| c.ends_with(".exe")) || e.kind == Terminal,
            "macos" => e.mac_app.is_some() || e.commands.iter().any(|c| !c.ends_with(".exe")),
            _ => e.commands.iter().any(|c| !c.ends_with(".exe")),
        })
        .map(|e| {
            let found = detect(e);
            let path = found.as_ref().map(|l| match l {
                Launch::Exe(p) | Launch::MacApp(p) => p.to_string_lossy().to_string(),
            });
            EditorInfo {
                id: e.id.into(),
                name: e.name.into(),
                detected: found.is_some(),
                path,
                terminal: e.kind == Terminal,
                opens_at_line: !e.line_args.is_empty(),
            }
        })
        .collect();
    out.sort_by_key(|e| (!e.detected, e.name.to_ascii_lowercase()));
    Ok(out)
}

/// Starts the chosen app on `file`. Returns a display name for status messages.
fn launch(setting: &EditorSetting, category: Category, file: &Path, line: u32) -> Result<String, String> {
    match setting.id.as_str() {
        "system" | "" => {
            open::that_detached(file).map_err(|e| format!("No default app is set for this file type ({e}). Pick an app in Settings."))?;
            Ok("your default app".into())
        }
        "custom" => {
            let exe = PathBuf::from(setting.path.as_deref().ok_or("Choose the program in Settings")?);
            if !exe.exists() {
                return Err(format!("{} not found — check Settings", exe.display()));
            }
            if exe.extension().is_some_and(|x| x == "app") {
                Command::new("open").arg("-a").arg(&exe).arg(file).spawn().map_err(|e| e.to_string())?;
                return Ok(exe.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default());
            }
            let template = setting.args.clone().filter(|t| !t.trim().is_empty()).unwrap_or_else(|| "{file}".into());
            let mut args: Vec<String> =
                split_template(&template).into_iter().map(|a| a.replace("{file}", &file.to_string_lossy()).replace("{line}", &line.to_string())).collect();
            if !template.contains("{file}") {
                args.push(file.to_string_lossy().to_string());
            }
            spawn(&exe, &args, false)?;
            Ok(exe.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default())
        }
        id => {
            let e = CATALOG.iter().find(|e| e.id == id && e.category == category).ok_or("Unknown app; pick one again in Settings")?;
            let launch = match setting.path.as_deref().map(PathBuf::from).filter(|p| p.exists()) {
                Some(p) if p.extension().is_some_and(|x| x == "app") => Launch::MacApp(p),
                Some(p) => Launch::Exe(p),
                None => detect(e).ok_or_else(|| format!("{} wasn't found on this computer. Choose its program in Settings.", e.name))?,
            };
            match launch {
                Launch::MacApp(bundle) => {
                    Command::new("open").arg("-a").arg(&bundle).arg(file).spawn().map_err(|e| e.to_string())?;
                }
                Launch::Exe(exe) => {
                    let args = if e.line_args.is_empty() { fill(FILE, file, line) } else { fill(e.line_args, file, line) };
                    spawn(&exe, &args, e.kind == Terminal)?;
                }
            }
            Ok(e.name.into())
        }
    }
}

/// Opens a project file in the configured text editor, at `line` when the editor supports it.
#[tauri::command]
pub async fn editor_open(path: String, line: Option<u32>, state: State<'_, SessionState>, store: State<'_, StoreState>) -> Result<String, String> {
    let file = state.lock().map_err(|_| "State poisoned")?.project_file(&path)?;
    if !file.is_file() {
        return Err("File not found in the HUD folder".into());
    }
    let setting = store.lock().map_err(|_| "State poisoned")?.settings.editor.clone();
    launch(&setting, Category::Text, &file, line.unwrap_or(1).max(1))
}

/// Opens an image in the configured image editor. `as_png` decodes a VTF (or any material the HUD can see,
/// including stock ones) into a PNG copy first, for editors that can't read VTF.
#[tauri::command]
pub async fn image_editor_open(path: String, as_png: bool, state: State<'_, SessionState>, store: State<'_, StoreState>) -> Result<String, String> {
    let setting = store.lock().map_err(|_| "State poisoned")?.settings.image_editor.clone();
    let file = if as_png {
        let s = state.lock().map_err(|_| "State poisoned")?;
        let png = s.with_vfs(|v| -> Result<Vec<u8>, String> {
            let (bytes, _) = v.read_bytes(&path).ok_or_else(|| format!("{path} not found"))?;
            hud_core::vtf::to_png(&hud_core::vtf::decode(&bytes)?)
        })??;
        let dir = std::env::temp_dir().join("tf2-hud-studio");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let stem = Path::new(&path).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "image".into());
        let out = dir.join(format!("{stem}.png"));
        std::fs::write(&out, png).map_err(|e| e.to_string())?;
        out
    } else {
        let file = state.lock().map_err(|_| "State poisoned")?.project_file(&path)?;
        if !file.is_file() {
            return Err("That image isn't part of your HUD (it comes from TF2). Open a PNG copy instead.".into());
        }
        file
    };
    launch(&setting, Category::Image, &file, 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wildcard_segments() {
        assert!(seg_match("Adobe Photoshop*", "Adobe Photoshop 2025"));
        assert!(seg_match("*PaintShop Pro*", "Corel PaintShop Pro 2023 (64-bit)"));
        assert!(seg_match("PhotoFiltre*.exe", "PhotoFiltre7.exe"));
        assert!(!seg_match("GIMP*.app", "Krita.app"));
        let root = std::env::temp_dir().join(format!("globtest{}", std::process::id()));
        std::fs::create_dir_all(root.join("JetBrains/IntelliJ IDEA 2024.1/bin")).unwrap();
        std::fs::create_dir_all(root.join("JetBrains/IntelliJ IDEA 2025.2/bin")).unwrap();
        std::fs::write(root.join("JetBrains/IntelliJ IDEA 2024.1/bin/idea64.exe"), b"").unwrap();
        std::fs::write(root.join("JetBrains/IntelliJ IDEA 2025.2/bin/idea64.exe"), b"").unwrap();
        let hits = glob(&root, "JetBrains\\*\\bin\\idea64.exe", '\\');
        assert_eq!(hits.len(), 2);
        assert!(hits[0].to_string_lossy().contains("2025.2"), "newest first");
    }
    #[test]
    fn templates() {
        assert_eq!(split_template(r#"-g "{file}:{line}" --wait"#), vec!["-g", "{file}:{line}", "--wait"]);
        assert_eq!(split_template(r#"--title "" {file}"#), vec!["--title", "", "{file}"]);
        assert_eq!(fill(&["-n{line}", "{file}"], Path::new("C:/a b/x.res"), 12), vec!["-n12", "C:/a b/x.res"]);
    }
    #[test]
    fn catalog_ids_are_unique() {
        let mut ids: Vec<&str> = CATALOG.iter().map(|e| e.id).collect();
        ids.sort();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n);
        assert!(CATALOG.iter().filter(|e| e.category == Category::Text).count() >= 40);
        assert!(CATALOG.iter().filter(|e| e.category == Category::Image).count() >= 20);
    }
}
