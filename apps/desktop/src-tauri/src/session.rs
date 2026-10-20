//! Editor session: open project, buffers with undo/redo, stock game assets and caches.
use hud_core::images::ImageResult;
use hud_core::project::FileEntry;
use hud_core::vfs::{index_project, GameAssets, ProjectIndex, Vfs};
use hud_syntax::{parse, Diagnostic, Encoding, Panel};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const HISTORY_LIMIT: usize = 300;

pub struct Buffer {
    pub original: String,
    pub content: String,
    pub encoding: Encoding,
    pub undo: Vec<String>,
    pub redo: Vec<String>,
}

impl Buffer {
    /// Records the current content in history and replaces it. No-op edits don't create history entries.
    pub fn apply(&mut self, next: String) {
        if next == self.content {
            return;
        }
        self.undo.push(std::mem::replace(&mut self.content, next));
        if self.undo.len() > HISTORY_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
    }
}

#[derive(Default)]
pub struct Session {
    pub root: Option<PathBuf>,
    pub files: Vec<FileEntry>,
    pub index: ProjectIndex,
    pub buffers: HashMap<String, Buffer>,
    pub game: Option<Arc<GameAssets>>,
    pub game_dir: Option<PathBuf>,
    pub images: HashMap<String, Result<ImageResult, String>>,
    pub tools: HashMap<String, PathBuf>,
    /// Lowercased localization tokens (stock tf_english.txt + project overrides), loaded on first use.
    pub strings: Option<HashMap<String, String>>,
}

pub type SessionState = Mutex<Session>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub path: String,
    pub source: String,
    pub panels: Vec<Panel>,
    pub diagnostics: Vec<Diagnostic>,
    pub dirty: bool,
    pub encoding: Encoding,
    pub can_undo: bool,
    pub can_redo: bool,
}

pub fn valid_relative(path: &str) -> Result<PathBuf, String> {
    let p = Path::new(path);
    if path.is_empty() || p.is_absolute() || p.components().any(|c| !matches!(c, std::path::Component::Normal(_))) {
        return Err(format!("Invalid project path '{path}'"));
    }
    Ok(p.to_path_buf())
}

impl Session {
    pub fn root(&self) -> Result<&PathBuf, String> {
        self.root.as_ref().ok_or_else(|| "No HUD open".to_string())
    }

    /// Absolute path of a project file, refusing anything that escapes the project folder.
    pub fn project_file(&self, path: &str) -> Result<PathBuf, String> {
        let root = self.root()?;
        let target = root.join(valid_relative(path)?);
        if let Ok(real) = dunce::canonicalize(&target) {
            if !real.starts_with(root) {
                return Err("Path escapes the HUD folder".into());
            }
            return Ok(real);
        }
        // new file: its parent must be inside the project
        let parent = target.parent().ok_or("Invalid path")?;
        if parent.exists() && !dunce::canonicalize(parent).map_err(|e| e.to_string())?.starts_with(root) {
            return Err("Path escapes the HUD folder".into());
        }
        Ok(target)
    }

    pub fn set_files(&mut self, files: Vec<FileEntry>) {
        self.index = index_project(&files.iter().map(|f| f.path.clone()).collect::<Vec<_>>());
        self.files = files;
    }

    pub fn file_paths(&self) -> Vec<String> {
        self.files.iter().map(|f| f.path.clone()).collect()
    }

    /// Canonical casing for a project path (case-insensitive lookup).
    pub fn canonical(&self, path: &str) -> String {
        self.index.get(&hud_core::vfs::normalize(path)).cloned().unwrap_or_else(|| path.to_string())
    }

    pub fn ensure_buffer(&mut self, path: &str) -> Result<String, String> {
        let path = self.canonical(path);
        if !self.buffers.contains_key(&path) {
            let file = self.project_file(&path)?;
            let meta = fs::metadata(&file).map_err(|e| format!("{path}: {e}"))?;
            if meta.len() > 8_000_000 {
                return Err("Text document too large to edit (over 8 MB)".into());
            }
            let bytes = fs::read(&file).map_err(|e| e.to_string())?;
            let (source, encoding) = hud_syntax::decode(&bytes).map_err(|e| format!("{path}: {e}"))?;
            self.buffers.insert(path.clone(), Buffer { original: source.clone(), content: source, encoding, undo: Vec::new(), redo: Vec::new() });
        }
        Ok(path)
    }

    pub fn document(&self, path: &str) -> Result<Document, String> {
        let buf = self.buffers.get(path).ok_or("Open the document first")?;
        let syntax = parse(&buf.content);
        // Animation scripts and console configs aren't KeyValues; report their own problems instead.
        let lower = path.to_ascii_lowercase();
        let diagnostics = if hud_core::lint::is_animation_script(&lower) {
            let (_, problems) = hud_core::anim::parse(&buf.content, path, true);
            let line_starts: Vec<usize> = std::iter::once(0).chain(buf.content.match_indices('\n').map(|(i, _)| i + 1)).collect();
            problems
                .into_iter()
                .map(|p| {
                    let start = line_starts.get(p.line.saturating_sub(1)).copied().unwrap_or(0);
                    let end = buf.content[start..].find('\n').map_or(buf.content.len(), |i| start + i);
                    Diagnostic { message: p.message, span: hud_syntax::Span { start, end } }
                })
                .collect()
        } else if lower.ends_with(".cfg") || lower.ends_with(".rc") || lower.ends_with(".md") {
            Vec::new()
        } else {
            syntax.diagnostics
        };
        Ok(Document {
            path: path.to_string(),
            source: buf.content.clone(),
            panels: syntax.panels,
            diagnostics,
            dirty: buf.content != buf.original,
            encoding: buf.encoding,
            can_undo: !buf.undo.is_empty(),
            can_redo: !buf.redo.is_empty(),
        })
    }

    pub fn with_vfs<R>(&self, f: impl FnOnce(&Vfs) -> R) -> Result<R, String> {
        let root = self.root()?.clone();
        let overlay = |p: &str| self.buffers.get(p).map(|b| b.content.clone());
        let vfs = Vfs { root: &root, index: &self.index, overlay: &overlay, game: self.game.as_deref() };
        Ok(f(&vfs))
    }
}
