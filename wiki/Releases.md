# Releases

## Stable
[Releases](https://github.com/XeraFang-Solutions/TF2-HUD-Studio/releases/latest) are tagged `vMAJOR.MINOR.PATCH`. Each
one has a portable build for Windows (x64 `.exe`), Linux (x86_64 AppImage) and macOS (universal `.app` in a zip).

## Cutting edge
The [cutting-edge](https://github.com/XeraFang-Solutions/TF2-HUD-Studio/releases/tag/cutting-edge) pre-release is
rebuilt from the newest commit on `main` after every push. Its version is the stable version plus the commit, for
example `1.0.0-edge.g1a2b3c4`. Settings show the version, so bug reports can name the exact build.

Use it to try features early. They may be unfinished, so keep backups of your HUD.

## How versions are made
The `VERSION` file in the repository is the single source of truth. When maintainers change it on `main`, the release
workflow builds and publishes that version. See
[`docs/RELEASING.md`](https://github.com/XeraFang-Solutions/TF2-HUD-Studio/blob/main/docs/RELEASING.md).
