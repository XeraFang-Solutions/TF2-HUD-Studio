# External Tools integration — alpha contract

The **External Tools** panel supports optional tools that the author already has installed. Current entries: VTFEdit, VTFEdit Reloaded, Crowbar, GIMP and Blender.

## Current behavior
- Search PATH (and select Windows installation folders) for known executable names.
- Manually register a known tool's executable with a native file chooser.
- Launch a registered/discovered application without arguments.
- Never bundle, download or silently install third-party software.
- All registrations are **session-only**, not yet persisted.

## Next integration milestone
1. Persist registrations in a versioned user preferences file, separate from HUD project metadata.
2. Implement capability descriptors (VTF import/export, VMT editing, model inspection, texture creation).
3. Give users an explicit `Open in...` action from the asset browser, with per-tool argument profiles.
4. Support optional configured batch conversion when a tool explicitly exposes a supported CLI; no assumed or invented command-line flags.
5. Add preflight checks, file watcher refresh, process exit status, and audit logs for pipeline operations.
6. Built-in open-source readers for common materials should remain available without requiring these tools.

## Safety
The editor only launches known registered IDs, validates the selected executable exists, does not execute shell commands or compose command strings, and does not pass arbitrary flags. Future integrations must validate file-path scope and require user consent for tool-side project file changes.
