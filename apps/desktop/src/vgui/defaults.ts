// What VGUI uses when a HUD file leaves a property out. Values follow the engine's own fallback order
// (Source SDK vgui_controls: Button/Label/Panel ApplySchemeSettings): a "*_override" key, else the scheme's
// BaseSettings entry (Button.BgColor, Label.TextColor…), else the engine's hard-coded color.
// The preview and the Inspector use these, so a HUD that relies on defaults looks the way TF2 draws it.
import { controlKind } from './catalog.ts';

export type DefaultSource = { value: string; from: string };
type Rule = { also?: string[]; base?: string; fallback?: string; same?: string };

const WHITE = '255 255 255 255';
const BLACK = '0 0 0 255';

/** Button state colors: each state falls back to the default state, like vgui::Button. */
const BUTTON: Record<string, Rule> = {
  defaultfgcolor_override: { also: ['fgcolor_override', 'fgcolor'], base: 'Button.TextColor', fallback: WHITE },
  defaultbgcolor_override: { base: 'Button.BgColor', fallback: BLACK },
  armedfgcolor_override: { base: 'Button.ArmedTextColor', same: 'defaultfgcolor_override' },
  armedbgcolor_override: { base: 'Button.ArmedBgColor', same: 'defaultbgcolor_override' },
  depressedfgcolor_override: { base: 'Button.DepressedTextColor', same: 'defaultfgcolor_override' },
  depressedbgcolor_override: { base: 'Button.DepressedBgColor', same: 'defaultbgcolor_override' },
  selectedfgcolor_override: { base: 'Button.SelectedTextColor', same: 'defaultfgcolor_override' },
  selectedbgcolor_override: { base: 'Button.SelectedBgColor', same: 'defaultbgcolor_override' },
  disabledfgcolor1_override: { base: 'Button.DisabledFgColor1', fallback: '117 117 117 255' },
  disabledfgcolor2_override: { base: 'Button.DisabledFgColor2', fallback: '30 30 30 255' },
  // CExImageButton tints its SubImage with these
  image_drawcolor: { fallback: WHITE },
  image_armedcolor: { same: 'image_drawcolor' },
  image_selectedcolor: { same: 'image_drawcolor' },
  image_depressedcolor: { same: 'image_drawcolor' },
  paintbackground: { fallback: '1' },
  font: { base: '', fallback: 'Default' },
};

const LABEL: Record<string, Rule> = {
  fgcolor: { base: 'Label.TextColor', fallback: WHITE },
  bgcolor: { base: 'Label.BgColor', fallback: '0 0 0 0' },
  disabledfgcolor1_override: { base: 'Label.DisabledFgColor1', fallback: '117 117 117 255' },
  disabledfgcolor2_override: { base: 'Label.DisabledFgColor2', fallback: '30 30 30 255' },
  font: { base: '', fallback: 'Default' },
  textalignment: { fallback: 'west' },
};

const IMAGE: Record<string, Rule> = {
  drawcolor: { fallback: WHITE },
};

function rulesFor(control: string): Record<string, Rule> {
  const kind = controlKind(control);
  if (kind === 'button') return BUTTON;
  if (kind === 'label' || kind === 'richtext') return LABEL;
  if (kind === 'image') return IMAGE;
  return {};
}

/** Synonyms the engine reads for the same setting (fgcolor_override on a button is its default text color). */
const ALIASES: Record<string, Record<string, string>> = {
  button: { fgcolor_override: 'defaultfgcolor_override', fgcolor: 'defaultfgcolor_override' },
  label: { fgcolor_override: 'fgcolor' },
};

/**
 * The value VGUI uses for `key` on a `control` when the HUD doesn't set it. `own` reads the element's own settings
 * (so armed colors can fall back to the element's default colors); `baseSetting` reads the scheme's BaseSettings.
 */
export function defaultFor(control: string, key: string, own: (k: string) => string | undefined, baseSetting: (name: string) => string | undefined): DefaultSource | undefined {
  const kind = controlKind(control);
  const rules = rulesFor(control);
  const k = ALIASES[kind === 'richtext' ? 'label' : kind]?.[key.toLowerCase()] ?? key.toLowerCase();
  const seen = new Set<string>();
  let cur: string | undefined = k;
  while (cur && !seen.has(cur)) {
    seen.add(cur);
    if (cur !== k) {
      const set = own(cur);
      if (set !== undefined && set !== '') return { value: set, from: cur };
    }
    const rule: Rule | undefined = rules[cur];
    if (!rule) return undefined;
    for (const a of rule.also ?? []) {
      const set = own(a);
      if (set !== undefined && set !== '') return { value: set, from: a };
    }
    if (rule.base) {
      const b = baseSetting(rule.base);
      if (b !== undefined && b !== '') return { value: b, from: `scheme ${rule.base}` };
    }
    if (rule.same) {
      cur = rule.same;
      continue;
    }
    return rule.fallback !== undefined ? { value: rule.fallback, from: 'TF2 default' } : undefined;
  }
  return undefined;
}

/** Every key with a known default for this control (for the Inspector). */
export function defaultKeys(control: string): string[] {
  return Object.keys(rulesFor(control));
}
