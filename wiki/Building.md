# Building

## Requirements
- [Rust](https://rustup.rs) (stable), Node.js 20+.
- [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/):
  - **Windows:** Microsoft C++ Build Tools and WebView2.
  - **macOS:** Xcode Command Line Tools.
  - **Debian/Ubuntu:**
    ```bash
    sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
    ```

## Run and build
```bash
npm ci --prefix apps/desktop
npm run tauri:dev                       # development, with hot reload
npm run tauri -- build --no-bundle      # portable executable in target/release/
npm run tauri -- build --bundles appimage   # Linux AppImage
npm run tauri -- build --target universal-apple-darwin --bundles app   # macOS universal .app
```

## Tests
```bash
npm test                                   # Rust workspace + frontend
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
HUD_CORPUS_PATH=/path/to/hud cargo test -p hud-syntax --test solar_light_corpus
HUD_CORPUS_PATH=/path/to/hud cargo test -p hud-core --test real_hud
FAKE_TF_DIR=/path/to/tf cargo test -p hud-core --test fake_game
```

See [`CONTRIBUTING.md`](https://github.com/XeraFang-Solutions/TF2-HUD-Studio/blob/main/CONTRIBUTING.md) and
[`docs/ARCHITECTURE.md`](https://github.com/XeraFang-Solutions/TF2-HUD-Studio/blob/main/docs/ARCHITECTURE.md).
