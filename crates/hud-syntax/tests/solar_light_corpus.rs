//! Round-trips a real HUD. Point HUD_CORPUS_PATH (or the older SOLARLIGHTHUD_PATH) at any HUD folder.
use hud_syntax::{decode, encode, parse};
use std::{fs, path::Path};

fn visit(path: &Path, files: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = fs::read_dir(path) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit(&path, files);
        } else if matches!(
            path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref(),
            Some("res" | "vdf" | "txt" | "cfg" | "rc" | "vmt")
        ) {
            files.push(path);
        }
    }
}

#[test]
fn corpus_files_round_trip_losslessly() {
    let Some(root) = std::env::var_os("HUD_CORPUS_PATH").or_else(|| std::env::var_os("SOLARLIGHTHUD_PATH")) else {
        eprintln!("HUD_CORPUS_PATH not set; corpus test skipped");
        return;
    };
    let mut files = Vec::new();
    visit(Path::new(&root), &mut files);
    assert!(!files.is_empty(), "no HUD text files found");
    let mut parsed = 0;
    for file in &files {
        let bytes = fs::read(file).unwrap();
        let Ok((source, encoding)) = decode(&bytes) else { continue };
        assert_eq!(encode(&source, encoding), bytes, "encoding round trip failed for {}", file.display());
        let document = parse(&source);
        let rebuilt: String = document.tokens.iter().map(|t| &source[t.span.start..t.span.end]).collect();
        assert_eq!(rebuilt, source, "lossless round trip failed for {}", file.display());
        if file.extension().is_some_and(|e| e.eq_ignore_ascii_case("res")) && !document.diagnostics.is_empty() {
            eprintln!("{}: {:?}", file.display(), document.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>());
        }
        parsed += 1;
    }
    assert!(parsed > 25, "expected a meaningful HUD corpus, got {parsed}");
    eprintln!("losslessly parsed {parsed} HUD text files");
}
