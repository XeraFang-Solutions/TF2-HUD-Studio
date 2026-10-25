use hud_core::project;
use std::{fs, path::PathBuf};

#[test]
fn discovers_secondary_libraries_in_both_steam_formats() {
    let root = std::env::temp_dir().join(format!("steam-discovery-{}", std::process::id()));
    let steam = root.join("Steam");
    let modern = root.join("second library");
    let legacy = root.join("old library");
    fs::create_dir_all(steam.join("steamapps")).unwrap();
    for library in [&modern, &legacy] {
        let tf = library.join("steamapps/common/Team Fortress 2/tf");
        fs::create_dir_all(tf.join("custom/test/resource/ui")).unwrap();
        fs::write(tf.join("gameinfo.txt"), b"GameInfo {}").unwrap();
    }
    let path = |p: &PathBuf| p.to_string_lossy().replace('\\', "/");
    fs::write(
        steam.join("steamapps/libraryfolders.vdf"),
        format!("\"libraryfolders\" {{ \"1\" {{ \"path\" \"{}\" }} \"2\" \"{}\" }}", path(&modern), path(&legacy)),
    )
    .unwrap();
    let installs = project::discover_from_roots(&[steam.clone(), steam]);
    assert_eq!(installs.len(), 2, "the same root must not duplicate detected installs");
    assert!(installs.iter().all(|install| install.huds.len() == 1));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn includes_xdg_native_and_flatpak_roots() {
    let home = std::env::temp_dir().join("steam-home");
    let data = home.join("custom-data");
    let roots = project::unix_steam_roots(&home, Some(&data));
    for expected in [
        data.join("Steam"),
        home.join(".steam/root"),
        home.join(".steam/debian-installation"),
        home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
    ] {
        assert!(roots.contains(&expected), "{}", expected.display());
    }
}
