// Connects themes to the app: applies the chosen (or draft) theme, loads TF2's fonts for it and saves changes.
import { useEffect, useMemo } from 'react';
import { api } from './api.ts';
import { getState, guard, set, updateSettings, useStore } from './store.ts';
import { applyTheme, findTheme, parseTheme, themePrefs, tf2FontsUsed, type Theme, type ThemePrefs } from './theme.ts';

const CACHE_KEY = 'hudstudio.theme';

/** Before React renders: the last theme used, so the window doesn't flash the default colors. */
export function applyCachedTheme() {
  try {
    const t = parseTheme(JSON.parse(localStorage.getItem(CACHE_KEY) ?? 'null'));
    if (t) applyTheme(t);
  } catch {
    /* storage unavailable: the stylesheet's TF2 defaults stay */
  }
}

export function currentPrefs(): ThemePrefs {
  return themePrefs(getState().app?.settings.ui);
}

/** The theme on screen: the one being edited, else the saved choice. */
export function useTheme(): Theme {
  const ui = useStore((s) => s.app?.settings.ui);
  const draft = useStore((s) => s.themeDraft);
  const saved = useMemo(() => findTheme(themePrefs(ui)), [ui]);
  return draft ?? saved;
}

const uiFaces: FontFace[] = [];
let fontsFor: string | null | undefined;

async function loadUiFonts(tfDir: string | null) {
  if (fontsFor === tfDir) return;
  fontsFor = tfDir;
  for (const f of uiFaces.splice(0)) document.fonts.delete(f);
  const fonts = tfDir ? ((await guard(null, () => api.uiFonts())) ?? []) : [];
  const found: string[] = [];
  for (const f of fonts) {
    try {
      const bin = Uint8Array.from(atob(f.base64), (c) => c.charCodeAt(0));
      const face = new FontFace(`hs-${f.id}`, bin.buffer);
      await face.load();
      document.fonts.add(face);
      uiFaces.push(face);
      found.push(f.id);
    } catch {
      /* unreadable font file: the theme's fallback font is used */
    }
  }
  set({ uiFonts: found });
}

/** Mount once (in App): keeps <html> styled with the current theme and TF2's fonts loaded. */
export function useAppearance() {
  const theme = useTheme();
  const tfDir = useStore((s) => s.app?.settings.tfDir ?? null);
  const ready = useStore((s) => !!s.app);
  useEffect(() => {
    applyTheme(theme);
    try {
      localStorage.setItem(CACHE_KEY, JSON.stringify(theme));
    } catch {
      /* first paint just uses the defaults */
    }
  }, [theme]);
  useEffect(() => {
    if (ready && tf2FontsUsed(theme).length) void loadUiFonts(tfDir);
  }, [ready, tfDir, theme]);
}

/** Saves the chosen theme and the user's own themes into settings. */
export async function saveThemePrefs(prefs: ThemePrefs) {
  const app = getState().app;
  if (!app) return;
  await updateSettings({ ui: { ...app.settings.ui, themeId: prefs.themeId, customThemes: prefs.customThemes } });
}
