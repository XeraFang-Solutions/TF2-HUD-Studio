# Releasing

Releases are driven by the **[`VERSION`](../VERSION)** file. Pushing a change to it on `main` is what makes a release.

## Stable release

1. Pick the new version using [SemVer](https://semver.org):
   - **patch** for fixes;
   - **minor** for new features;
   - **major** for breaking changes to settings or behaviour.
2. On a branch:
   ```bash
   echo 1.1.0 > VERSION
   npm run version:sync     # writes it into package.json, Cargo.toml and tauri.conf.json
   cargo check              # refreshes Cargo.lock
   git commit -am "Release 1.1.0"
   ```
3. Open a pull request and wait for CI to pass, then merge it into `main`.
4. [`release.yml`](../.github/workflows/release.yml) then:
   - checks that `VERSION` is valid and that tag `v1.1.0` doesn't exist yet;
   - builds the portable Windows `.exe`, the Linux `.AppImage` and the macOS universal `.app` (zipped);
   - creates the tag `v1.1.0` and a GitHub release with the three files and generated notes, marked as *latest*.
5. Edit the release notes on GitHub if you want to highlight anything.

The release workflow **only** runs when `VERSION` changes, or when you start it by hand. If a run fails, fix the
problem and start **Release** again from the Actions tab. The tag check stops it from publishing the same version twice.

## Cutting edge

[`cutting-edge.yml`](../.github/workflows/cutting-edge.yml) runs on every push to `main`, including release commits. It skips pushes that
only touch docs, the wiki, artwork or issue templates. It:

- sets the version to `<VERSION>-edge.g<short sha>` (for example `1.0.0-edge.g1a2b3c4`), so the build says exactly which commit it came from;
- builds the same three portable files;
- replaces the single rolling `cutting-edge` pre-release and moves its tag to the new commit.

The `g` prefix keeps the version valid SemVer even when the hash is all digits.

## Wiki

[`wiki.yml`](../.github/workflows/wiki.yml) copies `wiki/` into the repository's GitHub wiki whenever it changes on `main`.

**One-time setup:** GitHub only creates the wiki's git repository after its first page exists. Enable the wiki in the
repository's **Settings → Features**, open the **Wiki** tab, and save any page (for example "Home"). Then run
**Publish wiki** from the Actions tab. From then on, every change under `wiki/` is published automatically.

## Signing

Builds are not code-signed yet. To add signing later:

- **Windows:** sign the `.exe` with `signtool` in the build job, using a certificate stored as a secret.
- **macOS:** set `APPLE_CERTIFICATE`, `APPLE_SIGNING_IDENTITY` and the notarization secrets that the Tauri CLI reads.
