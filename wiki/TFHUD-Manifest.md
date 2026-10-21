# TFHUD.manifest

`TFHUD.manifest` sits at the top of a HUD folder, next to `info.vdf`. It describes the HUD for the [[HUD library|HUD-Library]].
It's a KeyValues file, like every other TF2 config, and TF2 itself ignores it.

```
"TFHUD"
{
	"manifest_version"	"1"
	"name"			"My HUD"
	"version"		"1.2.0"
	"author"		"me"
	"description"	"A short description of the HUD."
	"icon"			"tfhud/icon.png"
	"banner"		"tfhud/banner.png"
	"homepage"		"https://example.com"
	"source"		"https://github.com/me/my-hud"
	"license"		"CC BY-NC-SA 4.0"
	"tags"			"minimal, competitive, 4:3"
}
```

| Key | Meaning |
| --- | --- |
| `manifest_version` | Format version, currently `1`. |
| `name` | Display name. Falls back to the `info.vdf` name, then the folder name. |
| `version` | Your HUD's version, e.g. `1.2.0`. |
| `author`, `description`, `license`, `tags` | Free text. Tags are comma-separated. |
| `icon` | Square image, path relative to the HUD folder. PNG, JPG, WebP, GIF or SVG, up to 4 MB. |
| `banner` | Optional wide image, shown above the card. |
| `homepage`, `source` | Links. |

## Editing it
Use **HUD details**: the pencil on a library card, or the button in the **Install** dialog for the open HUD.
- Images you choose are copied into the HUD's `tfhud/` folder, so they travel with the HUD.
- Saving changes only the keys you edited. Comments, extra keys of your own and spacing are kept.
- Clearing a field removes its key.
- New HUDs made with **New HUD** start with a manifest.

Paths can't point outside the HUD folder (`..` and absolute paths are ignored).
