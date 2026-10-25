//! Steam launchers are not the same thing as Steam libraries. AppImages may inherit a reduced PATH.
use std::path::{Path, PathBuf};
use std::process::Command;

/// Compiled on Windows tests too, so the Linux command-building code doesn't escape type checking.
#[cfg_attr(windows, allow(dead_code))]
pub fn command(tf_dir: &Path) -> Option<Command> {
    let roots = hud_core::project::steam_roots();
    let dirs: Vec<_> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let native = native_candidates(tf_dir, &roots, &dirs, home.as_deref()).into_iter().find(|p| executable(p));
    let flatpak = dirs.iter().map(|d| d.join("flatpak")).chain([PathBuf::from("/usr/bin/flatpak"), PathBuf::from("/bin/flatpak")]).find(|p| executable(p));
    let flatpak_command = || {
        let exe = flatpak.as_ref()?;
        if !Command::new(exe).args(["info", "com.valvesoftware.Steam"]).output().is_ok_and(|o| o.status.success()) {
            return None;
        }
        let mut command = Command::new(exe);
        command.args(["run", "com.valvesoftware.Steam"]);
        Some(command)
    };
    if flatpak_install(tf_dir, &roots) {
        // Don't send a Flatpak-owned game to an unrelated native Steam client.
        return flatpak_command();
    }
    native.map(Command::new).or_else(flatpak_command)
}

pub fn executable(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else { return false };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o111 == 0 {
            return false;
        }
    }
    true
}

pub fn native_candidates(tf: &Path, roots: &[PathBuf], path: &[PathBuf], home: Option<&Path>) -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = path.iter().map(|dir| dir.join("steam")).collect();
    candidates.extend(["/usr/bin/steam", "/usr/games/steam", "/usr/local/bin/steam", "/run/current-system/sw/bin/steam"].map(PathBuf::from));
    if let Some(home) = home {
        candidates.extend([home.join(".local/bin/steam"), home.join(".nix-profile/bin/steam")]);
    }
    // Also cover a portable client alongside the configured game, even outside standard Steam roots.
    let owner = tf.ancestors().find(|p| p.file_name().is_some_and(|s| s == "steamapps")).and_then(Path::parent);
    for root in owner.into_iter().chain(roots.iter().map(PathBuf::as_path)) {
        candidates.extend([root.join("steam.sh"), root.join("steam")]);
    }
    candidates
}

pub fn flatpak_install(tf: &Path, roots: &[PathBuf]) -> bool {
    roots.iter().filter(|r| r.components().any(|c| c.as_os_str() == "com.valvesoftware.Steam")).any(|r| {
        hud_core::project::steam_libraries(r).iter().any(|lib| {
            let installed = lib.join("steamapps/common/Team Fortress 2/tf");
            match (dunce::canonicalize(&installed), dunce::canonicalize(tf)) {
                (Ok(a), Ok(b)) => a == b,
                _ => installed == tf,
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn executable_must_be_a_file_with_execute_permission_on_unix() {
        let dir = std::env::temp_dir().join(format!("steam-exe-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(!executable(&dir));
        let file = dir.join("steam");
        std::fs::write(&file, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
            assert!(!executable(&file));
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        assert!(executable(&file));
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn native_launcher_search_does_not_require_a_path_entry() {
        let base = PathBuf::from("portable-steam");
        let tf = base.join("steamapps/common/Team Fortress 2/tf");
        let paths = native_candidates(&tf, &[], &[], None);
        assert!(paths.contains(&base.join("steam.sh")));
        assert!(paths.contains(&PathBuf::from("/usr/games/steam")));
    }
    #[test]
    fn flatpak_secondary_library_is_associated_with_its_client() {
        let temp = std::env::temp_dir().join(format!("steam-owner-{}", std::process::id()));
        let root = temp.join("com.valvesoftware.Steam/data/Steam");
        let library = temp.join("external-library");
        std::fs::create_dir_all(root.join("steamapps")).unwrap();
        let value = library.to_string_lossy().replace('\\', "/");
        std::fs::write(root.join("steamapps/libraryfolders.vdf"), format!("\"libraryfolders\" {{ \"1\" {{ \"path\" \"{value}\" }} }}")).unwrap();
        assert!(flatpak_install(&library.join("steamapps/common/Team Fortress 2/tf"), &[root]));
        std::fs::remove_dir_all(temp).unwrap();
    }
}
