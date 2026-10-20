//! Project-wide problems: syntax errors, broken #base chains, undefined scheme colors/fonts,
//! dangling pin_to_sibling targets, missing images, and animation script errors.
use crate::anim;
use crate::images;
use crate::resolve::{resolve, RNode};
use crate::scheme::{parse_rgba, Scheme};
use crate::vfs::Vfs;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    pub severity: String,
    pub file: String,
    pub offset: Option<usize>,
    pub line: usize,
    pub message: String,
}

pub fn line_of(text: &str, offset: usize) -> usize {
    text[..offset.min(text.len())].matches('\n').count() + 1
}

/// Files that are KeyValues text (console scripts and animation scripts are not).
pub fn is_keyvalues(path: &str) -> bool {
    let p = path.to_ascii_lowercase();
    let anim = p.rsplit('/').next().is_some_and(|n| n.starts_with("hudanimations") && !n.contains("manifest"));
    (p.ends_with(".res") || p.ends_with(".vdf") || p.ends_with(".vmt") || p.ends_with(".txt")) && !anim
}

pub fn is_animation_script(path: &str) -> bool {
    let n = path.to_ascii_lowercase();
    let name = n.rsplit('/').next().unwrap_or("");
    name.starts_with("hudanimations") && name.ends_with(".txt") && !name.contains("manifest")
}

fn is_color_key(k: &str) -> bool {
    let k = k.to_ascii_lowercase();
    k.ends_with("color") || k.ends_with("color_override") || k.ends_with("color1_override") || k.ends_with("color2_override") || k.starts_with("teamcolor")
}

fn is_font_key(k: &str) -> bool {
    let k = k.to_ascii_lowercase();
    k == "font" || k.ends_with("font") || k.starts_with("font_") || k == "textfont" || k == "font_minmode"
}

struct Ctx<'a> {
    vfs: &'a Vfs<'a>,
    scheme: &'a Scheme,
    file: &'a str,
    text: &'a str,
    check_images: bool,
    out: &'a mut Vec<Problem>,
}

impl Ctx<'_> {
    fn push(&mut self, severity: &str, offset: Option<usize>, message: String) {
        let line = offset.map_or(0, |o| line_of(self.text, o));
        self.out.push(Problem { severity: severity.into(), file: self.file.into(), offset, line, message });
    }
    fn walk(&mut self, node: &RNode) {
        let siblings: Vec<String> = node
            .blocks()
            .flat_map(|b| {
                let mut v = vec![b.key.to_ascii_lowercase()];
                if let Some(f) = b.get("fieldName") {
                    v.push(f.to_ascii_lowercase())
                }
                v
            })
            .collect();
        let file = self.file;
        for child in node.blocks() {
            for p in child.children.iter().filter(|c| !c.is_block() && c.file == file) {
                let v = p.value.as_deref().unwrap_or("").trim();
                if v.is_empty() {
                    continue;
                }
                if is_color_key(&p.key) && v.chars().any(|c| c.is_ascii_alphabetic()) && parse_rgba(v).is_none() && self.scheme.color(v).is_none() {
                    self.push("warning", p.offset, format!("{}.{}: color '{v}' is not defined in the scheme", child.key, p.key));
                } else if is_font_key(&p.key) && !self.scheme.fonts.is_empty() && !self.scheme.has_font(v) {
                    self.push("warning", p.offset, format!("{}.{}: font '{v}' is not defined in the scheme", child.key, p.key));
                } else if p.key.eq_ignore_ascii_case("pin_to_sibling") && !siblings.contains(&v.to_ascii_lowercase()) {
                    self.push("info", p.offset, format!("{} is pinned to '{v}', which isn't a sibling in this file (fine if the game creates it)", child.key));
                } else if self.check_images
                    && (p.key.eq_ignore_ascii_case("image") || p.key.to_ascii_lowercase().starts_with("teambg_"))
                    && !v.starts_with('#')
                    && !images::exists(self.vfs, v)
                {
                    self.push("warning", p.offset, format!("{}.{}: image '{v}' not found in the HUD or stock TF2 materials", child.key, p.key));
                }
            }
            self.walk(child);
        }
    }
}

/// Files TF2 loads from a custom HUD folder by path. Other folders (a HUD's `#customization`, `_dev`, template
/// libraries…) only matter when something TF2 loads pulls them in through `#base`.
pub fn is_loaded_path(path: &str) -> bool {
    let l = path.to_ascii_lowercase();
    l.starts_with("resource/") || l.starts_with("scripts/") || l.starts_with("materials/") || l == "info.vdf"
}

/// Loaded files plus every project file they reach through `#base`/`#include`.
pub fn reachable_files(vfs: &Vfs, files: &[String]) -> std::collections::HashSet<String> {
    let mut set: std::collections::HashSet<String> = files.iter().filter(|f| is_loaded_path(f)).cloned().collect();
    for f in files.iter().filter(|f| is_loaded_path(f) && f.to_ascii_lowercase().ends_with(".res")) {
        if let Some(r) = resolve(vfs, f) {
            for label in r.chain {
                if let Some(p) = vfs.project_path(&label) {
                    set.insert(p.clone());
                }
            }
        }
    }
    set
}

pub fn run(vfs: &Vfs, files: &[String], scheme: &Scheme) -> Vec<Problem> {
    let mut out = Vec::new();
    let check_images = vfs.game.is_some();
    let reachable = reachable_files(vfs, files);
    for file in files.iter().filter(|f| reachable.contains(*f)) {
        let lower = file.to_ascii_lowercase();
        if is_animation_script(&lower) {
            if let Some((text, _)) = vfs.read_text(file) {
                let (_, probs) = anim::parse(&text, file, true);
                out.extend(probs.into_iter().map(|p| Problem { severity: "error".into(), file: p.file, offset: None, line: p.line, message: p.message }));
            }
            continue;
        }
        if !is_keyvalues(&lower) {
            continue;
        }
        let Some((text, _)) = vfs.read_text(file) else { continue };
        let t = hud_syntax::tree(&text);
        for d in &t.diagnostics {
            out.push(Problem {
                severity: "error".into(),
                file: file.clone(),
                offset: Some(d.span.start),
                line: line_of(&text, d.span.start),
                message: d.message.clone(),
            });
        }
        if !lower.ends_with(".res") {
            continue;
        }
        let Some(resolved) = resolve(vfs, file) else { continue };
        // TF2 silently skips a missing #base, and HUDs use that for optional user-override files.
        for m in resolved.missing.iter().filter(|m| m.from == *file) {
            out.push(Problem {
                severity: "info".into(),
                file: file.clone(),
                offset: Some(m.offset),
                line: line_of(&text, m.offset),
                message: format!("#base file '{}' not found — TF2 skips it (fine if it's an optional override file)", m.path),
            });
        }
        // duplicate sibling blocks in this file: the engine reads only the first
        fn dupes(nodes: &[hud_syntax::KvNode], text: &str, file: &str, out: &mut Vec<Problem>) {
            let mut seen: Vec<String> = Vec::new();
            // repeated "animation" trigger blocks inside a panel are legitimate
            for n in nodes.iter().filter(|n| n.is_block() && !n.key.eq_ignore_ascii_case("animation")) {
                let k = format!("{}|{}", n.key.to_ascii_lowercase(), n.condition.clone().unwrap_or_default());
                if seen.contains(&k) {
                    out.push(Problem {
                        severity: "info".into(),
                        file: file.into(),
                        offset: Some(n.span.start),
                        line: line_of(text, n.span.start),
                        message: format!("'{}' is defined again here — TF2 applies both, so these settings override the earlier block", n.key),
                    });
                } else {
                    seen.push(k)
                }
                dupes(&n.children, text, file, out);
            }
        }
        dupes(&t.roots, &text, file, &mut out);
        let scheme_file = lower.contains("clientscheme") || lower.starts_with("resource/scheme/") || lower.contains("sourcescheme");
        if scheme_file {
            continue;
        }
        if let Some(root) = resolved.root() {
            let mut ctx = Ctx { vfs, scheme, file, text: &text, check_images, out: &mut out };
            ctx.walk(root);
        }
    }
    // animation semantics across the whole manifest
    let anims = anim::load(vfs);
    for e in anims.events.iter().filter(|e| e.editable && e.shadowed_by.is_none()) {
        for c in &e.commands {
            if let Some(ev) = &c.event {
                if !anims.events.iter().any(|x| x.name.eq_ignore_ascii_case(ev)) {
                    out.push(Problem {
                        severity: "warning".into(),
                        file: e.file.clone(),
                        offset: None,
                        line: c.line,
                        message: format!("{} references unknown event '{ev}'", c.kind),
                    });
                }
            }
            if c.kind == "Animate" {
                let prop = c.property.as_deref().unwrap_or("").to_ascii_lowercase();
                let v = c.value.as_deref().unwrap_or("");
                if prop.ends_with("color") && v.chars().any(|ch| ch.is_ascii_alphabetic()) && scheme.color(v).is_none() {
                    out.push(Problem {
                        severity: "warning".into(),
                        file: e.file.clone(),
                        offset: None,
                        line: c.line,
                        message: format!(
                            "Color '{v}' used by {}.{} is not defined in the scheme",
                            c.panel.as_deref().unwrap_or("?"),
                            c.property.as_deref().unwrap_or("?")
                        ),
                    });
                }
            }
        }
    }
    for e in anims.events.iter().filter(|e| e.editable) {
        if let Some(first) = &e.shadowed_by {
            if first != &e.file {
                out.push(Problem {
                    severity: "info".into(),
                    file: e.file.clone(),
                    offset: Some(e.start),
                    line: e.line,
                    message: format!("Event '{}' is ignored: '{}' is loaded earlier in the manifest and defines it first", e.name, first),
                });
            }
        }
    }
    out
}
