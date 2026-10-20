//! Runs the resolver, scheme, animations and lint against a real HUD (HUD_CORPUS_PATH). Skipped when unset.
use hud_core::{anim, lint, project, resolve, scheme, vfs};
use std::path::PathBuf;

#[test]
fn real_hud_resolves() {
    let Some(root) = std::env::var_os("HUD_CORPUS_PATH").map(PathBuf::from) else {
        eprintln!("HUD_CORPUS_PATH not set; skipped");
        return;
    };
    let files: Vec<String> = project::scan(&root).unwrap().into_iter().map(|f| f.path).collect();
    let index = vfs::index_project(&files);
    let overlay = |_: &str| None;
    let v = vfs::Vfs { root: &root, index: &index, overlay: &overlay, game: None };

    let s = scheme::load(&v);
    eprintln!("scheme chain {:?}", s.chain);
    eprintln!(
        "{} colors, {} fonts, {} borders, font files {:?}",
        s.colors.len(),
        s.fonts.len(),
        s.borders.len(),
        s.font_files.iter().map(|f| (&f.path, f.found)).collect::<Vec<_>>()
    );
    assert!(s.file.is_none() || (!s.colors.is_empty() && !s.fonts.is_empty()), "a HUD that ships a clientscheme defines colors and fonts");
    let fd = scheme::font_data(&v, &s);
    for f in &fd {
        eprintln!("font {:?} cell {:.3} ascent {:.3}", f.families, f.cell_ratio, f.ascent_ratio);
    }

    // every .res file must resolve, and every #base it names must exist or be reported
    for f in files.iter().filter(|f| f.to_ascii_lowercase().ends_with(".res")) {
        let r = resolve::resolve(&v, f).unwrap_or_else(|| panic!("{f} failed to resolve"));
        assert!(!r.chain.is_empty());
    }

    let hl = resolve::resolve(&v, "scripts/hudlayout.res").unwrap();
    let root_node = hl.root().unwrap();
    let mut mapped = 0;
    for e in root_node.blocks() {
        let files = resolve::element_files(&v, &e.key);
        if !files.is_empty() {
            mapped += 1
        }
    }
    eprintln!("hudlayout: {} elements, {} mapped to .res files", root_node.blocks().count(), mapped);

    let a = anim::load(&v);
    eprintln!("animations: files {:?}, {} events, {} problems {:?}", a.files, a.events.len(), a.problems.len(), a.problems.iter().take(5).collect::<Vec<_>>());

    let problems = lint::run(&v, &files, &s);
    let mut by: std::collections::BTreeMap<String, usize> = Default::default();
    for p in &problems {
        *by.entry(p.severity.clone()).or_default() += 1
    }
    eprintln!("problems {by:?}");
    for p in problems.iter().take(25) {
        eprintln!("  {} {}:{} {}", p.severity, p.file, p.line, p.message);
    }
}
