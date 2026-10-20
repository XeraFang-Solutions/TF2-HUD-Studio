//! Exercises the stock-file paths with a TF2-like folder (FAKE_TF_DIR → a `tf` dir with tf2_misc_dir.vpk).
use hud_core::{images, project, resolve, scheme, vfs};
use std::path::PathBuf;

#[test]
fn stock_fallbacks() {
    let Some(tf) = std::env::var_os("FAKE_TF_DIR").map(PathBuf::from) else {
        eprintln!("FAKE_TF_DIR not set; skipped");
        return;
    };
    let game = vfs::GameAssets::open(&tf).unwrap();
    eprintln!("{} stock entries", game.vpks.iter().map(|v| v.len()).sum::<usize>());
    let empty = vfs::index_project(&[]);
    let overlay = |_: &str| None;
    let root = tf.join("custom/nothing");
    let v = vfs::Vfs { root: &root, index: &empty, overlay: &overlay, game: Some(&game) };

    let hl = resolve::resolve(&v, "scripts/hudlayout.res").expect("stock hudlayout");
    assert!(hl.chain[0].starts_with("stock:"));
    assert!(hl.root().unwrap().blocks().count() > 50);
    let s = scheme::load(&v);
    assert!(s.colors.len() > 100, "stock scheme chain resolves through #base inside the VPK");
    let fonts = scheme::font_data(&v, &s);
    assert!(fonts.iter().any(|f| f.path == "resource/tf2build.ttf"), "loose tf/resource fonts are found");

    let img = images::load(&v, "replay/thumbnails/star").expect("stock material decodes");
    eprintln!("star: {}x{} {} from {}", img.width, img.height, img.format, img.origin);
    assert!(img.data_url.starts_with("data:image/png;base64,"));

    let (text, _) = v.read_text("resource/tf_english.txt").unwrap();
    assert!(text.contains("Stock token"), "UTF-16 stock text decodes");

    let dir = std::env::temp_dir().join(format!("hudscaffold{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let made = project::scaffold(&dir, "Stock Copy", "stock", Some(&game)).unwrap();
    let files: Vec<String> = project::scan(&made).unwrap().into_iter().map(|f| f.path).collect();
    eprintln!("stock template files: {files:?}");
    for f in ["resource/clientscheme.res", "resource/scheme/colors.res", "scripts/hudlayout.res", "resource/ui/hudplayerhealth.res", "info.vdf"] {
        assert!(files.iter().any(|x| x == f), "{f} copied");
    }
    let made_empty = project::scaffold(&dir, "Empty One", "empty", None).unwrap();
    let files: Vec<String> = project::scan(&made_empty).unwrap().into_iter().map(|f| f.path).collect();
    assert!(!files.iter().any(|f| f.ends_with("hudlayout.res")), "empty HUD must not ship a partial hudlayout");
}

#[test]
fn zip_export_round_trips() {
    let Some(src) = std::env::var_os("HUD_CORPUS_PATH").map(PathBuf::from) else { return };
    let dest = std::env::temp_dir().join(format!("hudzip{}.zip", std::process::id()));
    let n = project::export_zip(&src, &dest).unwrap();
    let files = project::scan(&src).unwrap();
    assert_eq!(n, files.len());
    let mut z = zip::ZipArchive::new(std::fs::File::open(&dest).unwrap()).unwrap();
    assert_eq!(z.len(), n);
    let first = z.by_index(0).unwrap().name().to_string();
    assert!(first.contains('/'), "files are inside a top-level folder: {first}");
    eprintln!("zipped {n} files, first {first}");
}
