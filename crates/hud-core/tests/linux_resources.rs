//! Real file reads: Linux must retain stock PC conditions and find mixed-case custom material paths.
use hud_core::{images, project, resolve, vfs, vtf};
use std::fs;

#[test]
fn linux_keeps_desktop_layout_and_resolves_custom_and_patch_textures() {
    hud_syntax::set_platform(hud_syntax::Platform::Linux);
    let root = std::env::temp_dir().join(format!("linux-hud-resources-{}", std::process::id()));
    fs::create_dir_all(root.join("Resource/UI")).unwrap();
    fs::create_dir_all(root.join("Materials/VGUI/Custom")).unwrap();
    fs::write(
        root.join("Resource/UI/Health.res"),
        r#""Health" {
        "Value" { "xpos" "100" [$WIN32] "ypos" "r60" [$WIN32] "wide" "40" [$WIN32]
          "tall" "20" [$LINUX] "tall" "21" [$WINDOWS] }
        "Icon" { "ControlName" "ImagePanel" "image" "custom/patch" [$WIN32] }
    }"#,
    )
    .unwrap();
    fs::write(root.join("Materials/VGUI/Custom/Health.VMT"), r#""UnlitGeneric" { "$basetexture" "Materials/VGUI/Custom/Health.VTF" [$WIN32] }"#).unwrap();
    fs::write(root.join("Materials/VGUI/Custom/Patch.VMT"), r#""Patch" { "include" "vgui/custom/health.vmt" }"#).unwrap();
    fs::write(root.join("Materials/VGUI/Custom/Loop.VMT"), r#""Patch" { "include" "vgui/custom/loop.vmt" }"#).unwrap();
    let image = vtf::Image { width: 1, height: 1, rgba: vec![10, 20, 30, 255] };
    fs::write(root.join("Materials/VGUI/Custom/Health.VTF"), vtf::encode_bgra8888(&image, 0).unwrap()).unwrap();
    let files = project::scan(&root).unwrap().into_iter().map(|f| f.path).collect::<Vec<_>>();
    let index = vfs::index_project(&files);
    let overlay = |_: &str| None;
    let vfs = vfs::Vfs { root: &root, index: &index, overlay: &overlay, game: None };
    let resolved = resolve::resolve(&vfs, "resource/ui/health.res").unwrap();
    let panel = resolved.root().unwrap().child("Value").unwrap();
    assert_eq!(panel.get("xpos"), Some("100"));
    assert_eq!(panel.get("ypos"), Some("r60"));
    assert_eq!(panel.get("tall"), Some("20"));
    assert_eq!(resolved.root().unwrap().child("Icon").unwrap().get("image"), Some("custom/patch"));
    for name in ["custom/health", "custom/patch", "materials/vgui/custom/health.VMT", "materials/vgui/custom/health.VTF"] {
        let loaded = images::load(&vfs, name).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!((loaded.width, loaded.height), (1, 1));
        assert!(loaded.origin.contains("Health.VTF"));
    }
    assert!(images::load(&vfs, "custom/loop").is_err(), "VMT include loops must terminate");
    let tf = root.join("game/tf");
    fs::create_dir_all(tf.join("Resource/UI")).unwrap();
    fs::write(tf.join("gameinfo.txt"), "GameInfo {}").unwrap();
    fs::write(tf.join("Resource/UI/MixedCase.res"), "\"panel\" {}").unwrap();
    let game = vfs::GameAssets::open(&tf).unwrap();
    assert!(game.exists("resource/ui/mixedcase.res"));
    assert!(game.read("resource/ui/mixedcase.res").is_some());
    fs::remove_dir_all(root).unwrap();
}
