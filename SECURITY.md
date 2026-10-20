# Security policy

## Supported versions

Security fixes go into the **latest release** and the **cutting-edge** build. Older releases are not patched, so
please update to the latest version before you report a problem.

| Version | Supported |
| --- | --- |
| Latest `v1.x` release | ✅ |
| `cutting-edge` | ✅ |
| Anything older | ❌ |

## Reporting a vulnerability

**Please don't open a public issue for security problems.** Report them privately instead:

1. Go to the repository's **Security** tab and click
   **[Report a vulnerability](https://github.com/XeraFang-Solutions/TF2-HUD-Studio/security/advisories/new)**
   (GitHub private vulnerability reporting).
2. Include:
   - what the problem is and what an attacker could do with it;
   - the steps to reproduce it, and a sample HUD or file if one is involved;
   - the version (shown at the bottom of **Settings**) and your OS.

What to expect:
- **Acknowledgement** within **5 days**.
- An **assessment** within **14 days**.
- **Credit** in the release notes, if you'd like it.

We ask that you give us a reasonable amount of time to release a fix before you disclose the issue publicly.

## Scope

TF2 HUD Studio opens HUD folders that people download from the internet, so these are in scope:

- **Crafted HUD files** (`.res`, `.txt`, `.vdf`, `.vmt`, `.vtf`, fonts, VPKs) that crash the app in a way you can
  exploit, or that make it read or write files **outside** the opened HUD folder, the TF2 folder or the app's own data
  folder.
- **Path traversal** in install, zip export, backups or "customize a TF2 file".
- **Running a program the user didn't choose**, for example by getting a value from a HUD file into an editor or tool launch.
- **Problems in the release pipeline** that could let someone tamper with published builds.

Out of scope:
- **Unsigned builds.** These are a known limitation and are documented in the README.
- **Programs you chose yourself** in Settings or Tools doing what they normally do.
- **Bugs in TF2 itself, or in third-party HUDs.**

## How the app protects you

- **File access:** the backend only reads and writes paths inside the open HUD, the configured TF2 folder and its own
  data folders. Every path from the UI is resolved and checked against those roots.
- **External programs:** editors and tools are started directly, never through a shell. The only exception is
  terminal editors on macOS and Linux, which open in a terminal window with a quoted command line. Only the file being
  edited (and, for text editors, a line number) is passed to them.
- **Network:** the app never makes network requests. It does not download or install anything.
