# Contributing to TF2 HUD Studio

Thanks for helping. Bug reports, render mismatches, docs, wiki pages and code are all welcome.

## Before you start

- **Bugs:** use the [bug report](https://github.com/XeraFang-Solutions/TF2-HUD-Studio/issues/new?template=bug_report.yml) form.
- **The preview doesn't match TF2:** use the [render mismatch](https://github.com/XeraFang-Solutions/TF2-HUD-Studio/issues/new?template=hud_render_mismatch.yml)
  form. Include a screenshot from the game, if you can, and the HUD (or just the files involved).
- **New features:** open a [feature request](https://github.com/XeraFang-Solutions/TF2-HUD-Studio/issues/new?template=feature_request.yml)
  first if it's a big change, so we can agree on the approach before you spend time on it.
- **Security problems:** please follow [SECURITY.md](SECURITY.md) and **don't** open a public issue.

Everyone is expected to follow the [code of conduct](CODE_OF_CONDUCT.md).

## Setting up

You need Rust (stable), Node.js 20+ and the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS.

```bash
git clone https://github.com/XeraFang-Solutions/TF2-HUD-Studio.git
cd TF2-HUD-Studio
npm ci --prefix apps/desktop
npm run tauri:dev            # the app with hot reload
```

The architecture overview is in [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md). In short:

- `crates/hud-syntax`: the lossless KeyValues parser. Every edit must round-trip byte-for-byte.
- `crates/hud-core`: the file system, `#base` resolution, scheme, animations, images, lint and install code.
- `apps/desktop/src`: the React UI and the VGUI layout engine (`src/vgui/`).
- `apps/desktop/src-tauri`: Tauri commands that connect the two.

## Before you open a pull request

Run the same checks as CI:

```bash
cargo fmt --all                                       # rustfmt.toml: max_width 160
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm --prefix apps/desktop run build                   # tsc + vite
npm --prefix apps/desktop test
```

If you changed the parser or the layout engine, please also run the corpus tests against a real HUD or two:

```bash
HUD_CORPUS_PATH=/path/to/hud cargo test -p hud-syntax --test solar_light_corpus
HUD_CORPUS_PATH=/path/to/hud cargo test -p hud-core --test real_hud
```

### Guidelines

- **Keep HUD files lossless.** Never rewrite a whole file when a small edit will do. Comments, spacing,
  conditions and the case of keys must survive.
- **Match the game.** When the preview and TF2 disagree, TF2 is right. A test, or a link to the Source SDK behaviour,
  makes these changes much easier to review.
- **Add tests** for parser, resolver and layout changes. Synthetic fixtures go in `fixtures/synthetic/`.
- **Keep the UI consistent.** Use the colour tokens in `style.css` (`--panel`, `--accent`, …), and the themed
  `Select` and `SuggestInput` components instead of native `<select>` and `<datalist>`.
- **Write user-facing text plainly.** Say what happens and what to do next.
- **Keep commits small and focused,** with a message that says *why*.

## Versioning

Pull requests should **not** change [`VERSION`](VERSION). Maintainers bump it when releasing, because any change to
`VERSION` on `main` publishes a release. If you want to test the sync script:

```bash
npm run version:sync           # copies VERSION into package.json, Cargo.toml and tauri.conf.json
```

## Wiki and docs

The wiki's source lives in [`wiki/`](wiki). Edit the Markdown there and open a pull request. When it merges into
`main`, the [wiki workflow](.github/workflows/wiki.yml) publishes it. Link between pages with `[[Page Name]]` or
`[text](Page-Name)`.

## License of contributions

TF2 HUD Studio is licensed under the **GNU General Public License v2.0** (GPL-2.0-only). By opening a pull request you
agree that your contribution is licensed under the same terms, and that you have the right to submit it.
Don't include code or assets you can't license this way, such as Valve's files or other HUDs without permission.
